use std::{
    collections::HashMap,
    ffi::CString,
    path::Path,
    slice,
    sync::{Arc, Mutex},
};

use anyhow::Context;
use once_cell::sync::Lazy;
use tracing::{error, info};

use super::ztd::ZtdArchive;
use crate::{
    resource_manager::{
        bfresourcemgr::BFResourcePtr,
        ztfile::{ztfile_to_raw_resource, ZTFile, ZTFileType},
    },
    util::{get_from_memory, get_string_from_memory},
};

static LAZY_RESOURCE_MAP: Lazy<Mutex<HashMap<String, LazyResource>>> = Lazy::new(|| Mutex::new(HashMap::new()));

struct LazyResourceMap {}

#[derive(Clone)]
enum ResourceBacking {
    LazyZipFile { archive: Arc<Mutex<ZtdArchive>> },
    LoadedZipFile { archive: Arc<Mutex<ZtdArchive>>, data: u32 },
    Custom { data: u32 },
}

struct ConcreteResource {
    archive_name: Option<String>,
    filename: String,
    type_: ZTFileType,
    data: u32,
}

struct LazyResource {
    pub backing: ResourceBacking,
    pub filename: String,
    pub type_: ZTFileType,
}

impl LazyResourceMap {
    fn remove(file_name: String) -> Option<()> {
        let mut binding = LAZY_RESOURCE_MAP.lock().unwrap();
        let value = binding.remove(&file_name)?;

        LazyResourceMap::drop_inner(value);
        Some(())
    }

    fn drop_inner(resource: LazyResource) {
        let data = match resource.backing {
            ResourceBacking::LoadedZipFile { data, archive: _ } => data,
            ResourceBacking::Custom { data } => data,
            ResourceBacking::LazyZipFile { archive: _ } => {
                return;
            }
        };
        let bf_resource_ptr = unsafe { Box::from_raw(data as *mut BFResourcePtr) };
        match resource.type_ {
            ZTFileType::Ini
            | ZTFileType::Ai
            | ZTFileType::Ani
            | ZTFileType::Cfg
            | ZTFileType::Lyt
            | ZTFileType::Scn
            | ZTFileType::Uca
            | ZTFileType::Ucs
            | ZTFileType::Ucb
            | ZTFileType::Toml
            | ZTFileType::Txt => {
                let data_string = unsafe { CString::from_raw(data as *mut i8) };
                drop(data_string);
            }
            ZTFileType::Animation | ZTFileType::Bmp | ZTFileType::Lle | ZTFileType::Tga | ZTFileType::Wav | ZTFileType::Palette | ZTFileType::Zoo => {
                let data_vec: Box<[u8]> =
                    unsafe { Box::from_raw(slice::from_raw_parts_mut(bf_resource_ptr.data_ptr as *mut _, bf_resource_ptr.content_size as usize)) };
                drop(data_vec);
            }
        }
        drop(bf_resource_ptr);
    }

    fn insert_lazy(file_name: String, archive: Arc<Mutex<ZtdArchive>>) {
        let file_type = match ZTFileType::try_from(Path::new(&file_name)) {
            Ok(file_type) => file_type,
            Err(e) => {
                error!("Error inserting file: {} error: {}", file_name, e);
                return;
            }
        };

        let mut binding = LAZY_RESOURCE_MAP.lock().unwrap();
        if let Some(existing) = binding.insert(
            file_name.clone().to_ascii_lowercase(),
            LazyResource {
                backing: ResourceBacking::LazyZipFile { archive },
                filename: file_name.clone(),
                type_: file_type,
            },
        ) {
            LazyResourceMap::drop_inner(existing);
        }
    }

    fn insert_custom(file_name: String, file_type: ZTFileType, data: u32) {
        let mut binding = LAZY_RESOURCE_MAP.lock().unwrap();
        if let Some(existing) = binding.insert(
            file_name.to_ascii_lowercase(),
            LazyResource {
                backing: ResourceBacking::Custom { data },
                filename: file_name.clone(),
                type_: file_type,
            },
        ) {
            LazyResourceMap::drop_inner(existing);
        }
    }

    fn get(key: &str) -> anyhow::Result<Option<ConcreteResource>> {
        let mut binding = LAZY_RESOURCE_MAP.lock().unwrap();
        let lowercase_key = key.to_ascii_lowercase();
        let Some(resource) = binding.get_mut(&lowercase_key) else {
            info!("LazyResource not found: {}", lowercase_key);
            return Ok(None);
        };

        // TODO: Use std::mem::take/replace to avoid cloning
        let (archive_name, data) = match resource.backing.clone() {
            ResourceBacking::LazyZipFile { archive } => {
                let mut binding = archive.lock().unwrap();
                let archive_name = binding.name().to_string();
                let mut file = binding
                    .by_name(&resource.filename)
                    .with_context(|| format!("Error finding file in archive: {}", resource.filename))?;
                let mut file_buffer = vec![0u8; file.size() as usize].into_boxed_slice();

                file.read_exact(&mut file_buffer)
                    .with_context(|| format!("Error reading file: {}", resource.filename))?;

                let ztfile = ZTFile::builder()
                    .file_name(resource.filename.clone())
                    .file_size(file_buffer.len() as u32)
                    .type_(resource.type_)
                    .raw_data(file_buffer)
                    .build();
                let data = ztfile_to_raw_resource(&archive_name, resource.filename.clone(), ztfile)?;
                resource.backing = ResourceBacking::LoadedZipFile { archive: archive.clone(), data };
                (Some(archive_name.clone()), data)
            }
            ResourceBacking::LoadedZipFile { archive, data } => {
                let binding = archive.lock().unwrap();
                (Some(binding.name().to_string()), data)
            }
            ResourceBacking::Custom { data } => (None, data),
        };

        Ok(Some(ConcreteResource {
            archive_name: archive_name.clone(),
            filename: resource.filename.clone(),
            type_: resource.type_,
            data,
        }))
    }

    fn loaded_len() -> usize {
        let binding = LAZY_RESOURCE_MAP.lock().unwrap();
        binding
            .values()
            .filter(|x| matches!(x.backing, ResourceBacking::LoadedZipFile { .. } | ResourceBacking::Custom { .. }))
            .count()
    }

    fn not_loaded_len() -> usize {
        let binding = LAZY_RESOURCE_MAP.lock().unwrap();
        binding.values().filter(|x| matches!(x.backing, ResourceBacking::LazyZipFile { .. })).count()
    }

    fn len() -> usize {
        let binding = LAZY_RESOURCE_MAP.lock().unwrap();
        binding.len()
    }

    fn contains_key(key: &str) -> bool {
        let binding = LAZY_RESOURCE_MAP.lock().unwrap();
        binding.contains_key(key)
    }

    fn file_names() -> Vec<String> {
        let binding = LAZY_RESOURCE_MAP.lock().unwrap();
        binding.keys().cloned().collect()
    }
}

pub fn check_file(file_name: &str) -> bool {
    LazyResourceMap::contains_key(&file_name.to_lowercase())
}

pub fn get_file_ptr(file_name: &str) -> Option<u32> {
    if let Ok(Some(resource)) = LazyResourceMap::get(&file_name.to_lowercase()) {
        Some(resource.data)
    } else {
        None
    }
}

pub fn get_file(file_name: &str) -> Option<(String, Box<[u8]>)> {
    match LazyResourceMap::get(file_name) {
        Ok(Some(file)) => {
            let resource_ptr = get_from_memory::<BFResourcePtr>(file.data);
            let tmp_slice = unsafe { slice::from_raw_parts(resource_ptr.data_ptr as *const _, resource_ptr.content_size as usize) };
            let mut new_slice = vec![0; resource_ptr.content_size as usize];
            new_slice.copy_from_slice(tmp_slice);
            Some((get_string_from_memory(resource_ptr.bf_zip_name_ptr), new_slice.into_boxed_slice()))
        }
        Ok(None) => {
            info!("File not found: {}", file_name);
            None
        }
        Err(e) => {
            info!("Error getting file: {} error: {}", file_name, e);
            None
        }
    }
}

pub fn get_file_names() -> Vec<String> {
    LazyResourceMap::file_names()
}

pub fn get_num_resources() -> usize {
    LazyResourceMap::len()
}

pub fn add_ztfile(path: &Path, file_name: String, ztfile: ZTFile) {
    let Some(ztd_path) = path.to_str() else {
        error!("Failed to convert path to string: {}", path.display());
        return;
    };
    let mut ztd_path = ztd_path.to_string();
    ztd_path = ztd_path.replace('\\', "/").replace("./", "zip::./");
    let lowercase_filename = file_name.to_lowercase();

    let bf_zip_name_ptr = match CString::new(ztd_path.clone()) {
        Ok(c_string) => c_string.into_raw() as u32,
        Err(e) => {
            error!("Error converting zip name to CString: {} -> {}", ztd_path, e);
            return;
        }
    };
    let bf_resource_name_ptr = match CString::new(lowercase_filename.clone()) {
        Ok(c_string) => c_string.into_raw() as u32,
        Err(e) => {
            error!("Error converting resource name to CString: {} -> {}", lowercase_filename, e);
            return;
        }
    };

    match ztfile {
        ZTFile::Text(data, file_type, length) => {
            let ptr = data.into_raw() as u32;
            let resource_ptr = Box::into_raw(Box::new(BFResourcePtr {
                num_refs: 100, // We set this very high to prevent the game from unloading the resource
                bf_zip_name_ptr,
                bf_resource_name_ptr,
                data_ptr: ptr,
                content_size: length,
            }));

            LazyResourceMap::insert_custom(lowercase_filename.clone(), file_type, resource_ptr as u32);
        }
        ZTFile::RawBytes(data, file_type, length) => {
            let ptr = data.as_ptr() as u32;
            std::mem::forget(data);
            let resource_ptr = Box::into_raw(Box::new(BFResourcePtr {
                num_refs: 100, // We set this very high to prevent the game from unloading the resource
                bf_zip_name_ptr,
                bf_resource_name_ptr,
                data_ptr: ptr,
                content_size: length,
            }));

            LazyResourceMap::insert_custom(lowercase_filename.clone(), file_type, resource_ptr as u32);
        }
    }
}

pub fn add_lazy(file_name: String, archive: Arc<Mutex<ZtdArchive>>) {
    LazyResourceMap::insert_lazy(file_name, archive);
}
