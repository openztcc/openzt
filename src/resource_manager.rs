use std::{fmt::Display, slice, str};
use std::{
    collections::{HashMap, HashSet}, ffi::CString, fmt, fs::File, io::{self, BufReader, Read}, path::{Path, PathBuf}, sync::{Mutex, Arc},
};

use anyhow::{Context, anyhow};

use bf_configparser::ini::{Ini, WriteOptions};
use once_cell::sync::Lazy;
use retour_utils::hook_module;
use tracing::{error, info};
use walkdir::WalkDir;
use zip::read::{ZipArchive, ZipFile};

use regex::Regex;

use crate::{
    animation::Animation,
    console::{add_to_command_register, CommandError},
    debug_dll::{get_from_memory, get_string_from_memory, save_to_memory},
    mods,
    string_registry::{add_string_to_registry, get_string_from_registry},
};

const GLOBAL_BFRESOURCEMGR_ADDRESS: u32 = 0x006380C0;

#[derive(Debug, Clone)]
pub enum ZTFile {
    Text(CString, ZTFileType, u32),
    RawBytes(Box<[u8]>, ZTFileType, u32),
}

#[derive(Debug, Clone)]
pub enum ZTFileType {
    Ai,
    Ani,
    Cfg,
    Lyt,
    Scn,
    Uca,
    Ucs,
    Ucb,
    Ini,
    Txt,
    Toml,
    Animation,
    Palette,
    TGA,
    Wav,
    Lle,
    Bmp,
}

// impl From<String> for ZTFileType {
    // fn from(file_name: String)
// }

impl From<&Path> for ZTFileType {
    fn from(path: &Path) -> Self {
        let file_extension = path.extension().unwrap_or_default().to_str().unwrap_or_default();
        match file_extension {
            "ai" => ZTFileType::Ai,
            "ani" => ZTFileType::Ani,
            "cfg" => ZTFileType::Cfg,
            "lyt" => ZTFileType::Lyt,
            "scn" => ZTFileType::Scn,
            "uca" => ZTFileType::Uca,
            "ucs" => ZTFileType::Ucs,
            "ucb" => ZTFileType::Ucb,
            "ini" => ZTFileType::Ini,
            "txt" => ZTFileType::Txt,
            "toml" => ZTFileType::Toml,
            "tga" => ZTFileType::TGA,
            "wav" => ZTFileType::Wav,
            "lle" => ZTFileType::Lle,
            "bmp" => ZTFileType::Bmp,
            _ => ZTFileType::Animation,
        }
    }
}

impl From<BFResourcePtr> for ZTFile {
    fn from(bf_resource_ptr: BFResourcePtr) -> Self {
        let filename = get_string_from_memory(bf_resource_ptr.bf_resource_name_ptr);
        let file_extension = Path::new(&filename)
            .extension()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default();
        let file_size = bf_resource_ptr.content_size;
        let data = bf_resource_ptr.data_ptr;
        match file_extension {
            "ai" => ZTFile::Text(
                unsafe { CString::from_raw(data as *mut i8) },
                ZTFileType::Ai,
                file_size,
            ),
            "cfg" => ZTFile::Text(
                unsafe { CString::from_raw(data as *mut i8) },
                ZTFileType::Cfg,
                file_size,
            ),
            "lyt" => ZTFile::Text(
                unsafe { CString::from_raw(data as *mut i8) },
                ZTFileType::Lyt,
                file_size,
            ),
            "scn" => ZTFile::Text(
                unsafe { CString::from_raw(data as *mut i8) },
                ZTFileType::Scn,
                file_size,
            ),
            "uca" => ZTFile::Text(
                unsafe { CString::from_raw(data as *mut i8) },
                ZTFileType::Uca,
                file_size,
            ),
            "ucs" => ZTFile::Text(
                unsafe { CString::from_raw(data as *mut i8) },
                ZTFileType::Ucs,
                file_size,
            ),
            "ucb" => ZTFile::Text(
                unsafe { CString::from_raw(data as *mut i8) },
                ZTFileType::Ucb,
                file_size,
            ),
            "ani" => ZTFile::Text(
                unsafe { CString::from_raw(data as *mut i8) },
                ZTFileType::Ani,
                file_size,
            ),
            "ini" => ZTFile::Text(
                unsafe { CString::from_raw(data as *mut i8) },
                ZTFileType::Ini,
                file_size,
            ),
            "txt" => ZTFile::Text(
                unsafe { CString::from_raw(data as *mut i8) },
                ZTFileType::Txt,
                file_size,
            ),
            "tga" => ZTFile::RawBytes(
                unsafe {
                    Box::from_raw(slice::from_raw_parts_mut(data as *mut _, file_size as usize))
                },
                ZTFileType::TGA,
                file_size,
            ),
            "pal" => ZTFile::RawBytes(
                unsafe {
                    Box::from_raw(slice::from_raw_parts_mut(data as *mut _, file_size as usize))
                },
                ZTFileType::Palette,
                file_size,
            ),
            "wav" => ZTFile::RawBytes(
                unsafe {
                    Box::from_raw(slice::from_raw_parts_mut(data as *mut _, file_size as usize))
                },
                ZTFileType::Wav,
                file_size,
            ),
            "lle" => ZTFile::RawBytes(
                unsafe {
                    Box::from_raw(slice::from_raw_parts_mut(data as *mut _, file_size as usize))
                },
                ZTFileType::Lle,
                file_size,
            ),
            "bmp" => ZTFile::RawBytes(
                unsafe {
                    Box::from_raw(slice::from_raw_parts_mut(data as *mut _, file_size as usize))
                },
                ZTFileType::Bmp,
                file_size,
            ),
            _ => ZTFile::RawBytes(
                unsafe {
                    Box::from_raw(slice::from_raw_parts_mut(data as *mut _, file_size as usize))
                },
                ZTFileType::Animation,
                file_size,
            ),
        }
    }
}

impl ZTFile {
    pub fn new(file_name: String, file_size: u32, data: Box<[u8]>) -> anyhow::Result<ZTFile> {
        let file_extension = Path::new(&file_name).extension().unwrap_or_default().to_str().unwrap_or_default();
        match file_extension {
            "ai" | "cfg" | "lyt" | "scn" | "uca" | "ucs" | "ucb" | "ani" | "ini" | "txt" | "toml" => {

                ZTFile::new_text(file_name, file_size, CString::new(data)?)
            },
            // Handles "tga", "pal", "wav", "lle", "bmp" or ""
            _ => {
                Ok(ZTFile::new_raw_bytes(file_name, file_size, data))
            },
        }
    }

    pub fn new_text_from_bytes(file_name: String, file_size: u32, data: Box<[u8]>) -> anyhow::Result<ZTFile> {
        let c_string = CString::new(str::from_utf8(&data)?)?;
        ZTFile::new_text(file_name, file_size, c_string)
    }
    
    pub fn new_text(file_name: String, file_size: u32, data: CString) -> anyhow::Result<ZTFile> {
        let file_extension = Path::new(&file_name).extension().unwrap_or_default().to_str().unwrap_or_default();
        match file_extension {
            "ai" => Ok(ZTFile::Text(data, ZTFileType::Ai, file_size)),
            "cfg" => Ok(ZTFile::Text(data, ZTFileType::Cfg, file_size)),
            "lyt" => Ok(ZTFile::Text(data, ZTFileType::Lyt, file_size)),
            "scn" => Ok(ZTFile::Text(data, ZTFileType::Scn, file_size)),
            "uca" => Ok(ZTFile::Text(data, ZTFileType::Uca, file_size)),
            "ucs" => Ok(ZTFile::Text(data, ZTFileType::Ucs, file_size)),
            "ucb" => Ok(ZTFile::Text(data, ZTFileType::Ucb, file_size)),
            "ani" => Ok(ZTFile::Text(data, ZTFileType::Ani, file_size)),
            "ini" => Ok(ZTFile::Text(data, ZTFileType::Ini, file_size)),
            "txt" => Ok(ZTFile::Text(data, ZTFileType::Txt, file_size)),
            "toml" => Ok(ZTFile::Text(data, ZTFileType::Toml, file_size)),
            _ => Err(anyhow!("Invalid file type")),
        }
    }

    pub fn new_raw_bytes(file_name: String, file_size: u32, data: Box<[u8]>) -> ZTFile {
        let file_extension = Path::new(&file_name).extension().unwrap_or_default().to_str().unwrap_or_default();
        match file_extension {
            "tga" => ZTFile::RawBytes(data, ZTFileType::TGA, file_size),
            "pal" => ZTFile::RawBytes(data, ZTFileType::Palette, file_size),
            "wav" => ZTFile::RawBytes(data, ZTFileType::Wav, file_size),
            "lle" => ZTFile::RawBytes(data, ZTFileType::Lle, file_size),
            "bmp" => ZTFile::RawBytes(data, ZTFileType::Bmp, file_size),
            _ => ZTFile::RawBytes(data, ZTFileType::Animation, file_size),
        }
    }
}

pub trait FromZipFile<T> {
    fn from_zip_file(file: &mut ZipFile) -> io::Result<T>;
}

// pub fn add_txt_file_to_map_with_path_override(entry: &Path, file: &mut LazyResource, path: String) {
//     let mut buffer = vec![0; file.size() as usize].into_boxed_slice();
//     match file.read_exact(&mut buffer) {
//         Ok(bytes_read) => bytes_read,
//         Err(e) => {
//             error!("Error reading file: {} {} -> {}", entry.display(), file.name(), e);
//             return;
//         }
//     };

//     let intermediate_string = String::from_utf8_lossy(&buffer).to_string();

//     let file_size = intermediate_string.len();
//     let file_contents = match CString::new(intermediate_string) {
//         Ok(c_string) => c_string,
//         Err(e) => {
//             error!("Error converting file contents to CString: {} {} -> {}", entry.display(), file.name(), e);
//             return;
//         }
//     };

//     let ztfile = match ZTFile::new_text(path.clone(), file_size as u32, file_contents) {
//         Ok(ztfile) => ztfile,
//         Err(e) => {
//             error!("Error creating ZTFile from text: {} {} -> {}", entry.display(), file.name(), e);
//             return;
//         }
//     };

//     add_ztfile(entry, path, ztfile);
// }

// pub fn add_txt_file_to_map(entry: &Path, file: &mut ZipFile) {
//     let file_name = file.name().to_string().to_lowercase();

//     add_txt_file_to_map_with_path_override(entry, file, file_name)
// }

// pub fn add_raw_bytes_to_map_with_path_override(entry: &Path, file: &mut ZipFile, path: String) {
//     let mut buffer = vec![0; file.size() as usize].into_boxed_slice();
//     match file.read_exact(&mut buffer) {
//         Ok(_) => {}
//         Err(e) => {
//             error!("Error reading file: {} {} -> {}", entry.display(), file.name(), e);
//             return;
//         }
//     };

//     let file_size = file.size() as u32;
//     add_ztfile(entry, path.clone(), ZTFile::new_raw_bytes(path, file_size, buffer));
// }

// pub fn add_raw_bytes_file_to_map(entry: &Path, file: &mut ZipFile) {
//     let file_name = file.name().to_string().to_lowercase();
//     add_raw_bytes_to_map_with_path_override(entry, file, file_name)
// }

// Contains a mapping of file_paths to BFResourcePtrs
// static RESOURCE_STRING_TO_PTR_MAP: Lazy<Mutex<HashMap<String, u32>>> = Lazy::new(|| Mutex::new(HashMap::new()));

// static RESOURCE_PTR_PTR_SET: Lazy<Mutex<HashSet<u32>>> = Lazy::new(|| Mutex::new(HashSet::new()));

// pub fn add_ptr_ptr(ptr_ptr: u32) {
//     let mut binding = RESOURCE_PTR_PTR_SET.lock().unwrap();
//     binding.insert(ptr_ptr);
// }

// pub fn check_ptr_ptr(ptr_ptr: u32) -> bool {
//     let binding = RESOURCE_PTR_PTR_SET.lock().unwrap();
//     binding.contains(&ptr_ptr)
// }

// pub fn check_file(file_name: &str) -> bool {
//     let binding = RESOURCE_STRING_TO_PTR_MAP.lock().unwrap();
//     binding.contains_key(&file_name.to_lowercase())
// }

// pub fn get_file_ptr(file_name: &str) -> Option<u32> {
//     let binding = RESOURCE_STRING_TO_PTR_MAP.lock().unwrap();
//     binding.get(&file_name.to_lowercase()).copied()
// }

fn get_num_mod_ids() -> usize {
    let binding = MOD_ID_SET.lock().unwrap();
    binding.len()
}

fn command_list_resource_strings(args: Vec<&str>) -> Result<String, CommandError> {
    if args.len() > 1 {
        return Err(CommandError::new("Too many arguments".to_string()));
    }
    let binding = LAZY_RESOURCE_MAP.lock().unwrap();
    let mut result_string = String::new();
    for resource_string in binding.files() {
        if args.len() == 1 && !resource_string.starts_with(args[0]) {
            continue;
        }
        result_string.push_str(&format!("{}\n", resource_string));
    }
    Ok(result_string)
}

fn command_list_openzt_resource_strings(_args: Vec<&str>) -> Result<String, CommandError> {
    let binding = LAZY_RESOURCE_MAP.lock().unwrap();
    let mut result_string = String::new();
    for resource_string in binding.files() {
        if resource_string.starts_with("openzt") {
            result_string.push_str(&format!("{}\n", resource_string));
        }
    }
    Ok(result_string)
}

fn ztfile_to_raw_resource(path: &String, file_name: String, ztfile: ZTFile)  -> anyhow::Result<u32> {
    // let Some(ztd_path) = path.to_str() else {
    //     return Err(anyhow!("Failed to convert path to string: {}", path.display()));
    // };
    let mut ztd_path = path.clone();
    ztd_path = ztd_path.replace("./", "zip::./").replace('\\', "/");
    let lowercase_filename = file_name.to_lowercase();

    let bf_zip_name_ptr = match CString::new(ztd_path.clone()) {
        Ok(c_string) => c_string.into_raw() as u32,
        Err(e) => {
            return Err(anyhow!("Error converting zip name to CString: {} -> {}", ztd_path, e));
        }
    };
    let bf_resource_name_ptr = match CString::new(lowercase_filename.clone()) {
        Ok(c_string) => c_string.into_raw() as u32,
        Err(e) => {
            return Err(anyhow!("Error converting resource name to CString: {} -> {}", lowercase_filename, e));
        }
    };

    match ztfile {
        ZTFile::Text(data, _, length) => {
            let ptr = data.into_raw() as u32;
            let resource_ptr = Box::into_raw(Box::new(BFResourcePtr {
                num_refs: 100, // We set this very high to prevent the game from unloading the resource
                bf_zip_name_ptr,
                bf_resource_name_ptr,
                data_ptr: ptr,
                content_size: length,
            }));

            return Ok(resource_ptr as _);
        }
        ZTFile::RawBytes(data, _, length) => {
            let ptr = data.as_ptr() as u32;
            std::mem::forget(data);
            let resource_ptr = Box::into_raw(Box::new(BFResourcePtr {
                num_refs: 100, // We set this very high to prevent the game from unloading the resource
                bf_zip_name_ptr,
                bf_resource_name_ptr,
                data_ptr: ptr,
                content_size: length,
            }));

            return Ok(resource_ptr as _);
        }
    }
}

fn add_ztfile(path: &Path, file_name: String, ztfile: ZTFile) {
    let Some(ztd_path) = path.to_str() else {
        error!("Failed to convert path to string: {}", path.display());
        return;
    };
    let mut ztd_path = ztd_path.to_string();
    ztd_path = ztd_path.replace("./", "zip::./").replace('\\', "/");
    let lowercase_filename = file_name.to_lowercase();

    // let mut binding = RESOURCE_STRING_TO_PTR_MAP.lock().unwrap();
    let mut binding = LAZY_RESOURCE_MAP.lock().unwrap();

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
        ZTFile::Text(data, _, length) => {
            let ptr = data.into_raw() as u32;
            let resource_ptr = Box::into_raw(Box::new(BFResourcePtr {
                num_refs: 100, // We set this very high to prevent the game from unloading the resource
                bf_zip_name_ptr,
                bf_resource_name_ptr,
                data_ptr: ptr,
                content_size: length,
            }));

            binding.insert_custom(file_name.clone(), resource_ptr as u32);
        }
        ZTFile::RawBytes(data, _, length) => {
            let ptr = data.as_ptr() as u32;
            std::mem::forget(data);
            let resource_ptr = Box::into_raw(Box::new(BFResourcePtr {
                num_refs: 100, // We set this very high to prevent the game from unloading the resource
                bf_zip_name_ptr,
                bf_resource_name_ptr,
                data_ptr: ptr,
                content_size: length,
            }));

            binding.insert_custom(lowercase_filename.clone(), resource_ptr as u32);
        }
    }
}

pub fn modify_ztfile<F>(file_name: &str, modifier: F) -> Result<(), &'static str>
where
    F: Fn(&mut BFResourcePtr),
{
    let Some(bf_resource_ptr_ptr) = get_file_ptr(file_name) else {
        info!("File not found: {}", file_name);
        return Err("File not found");
    };
    let mut bf_resource_ptr = get_from_memory::<BFResourcePtr>(bf_resource_ptr_ptr);

    modifier(&mut bf_resource_ptr);

    save_to_memory::<BFResourcePtr>(bf_resource_ptr_ptr, bf_resource_ptr.clone());

    Ok(())
}

pub fn modify_ztfile_as_ini<F>(file_name: &str, modifier: F) -> Result<(), &'static str>
where
    F: Fn(&mut Ini),
{
    modify_ztfile(file_name, |file: &mut BFResourcePtr| {
        let c_string = unsafe { CString::from_raw(file.data_ptr as *mut i8) };
        let c_string_as_string = c_string.to_string_lossy().to_string();
        let mut cfg = Ini::new_cs();
        cfg.set_comment_symbols(&[';', '#', ':']);
        if let Err(err) = cfg.read(c_string_as_string) {
            error!("Error reading ini: {}", err);
            return;
        };

        modifier(&mut cfg);

        let mut write_options = WriteOptions::default();
        write_options.space_around_delimiters = true;
        write_options.blank_lines_between_sections = 1;
        let new_string = cfg.pretty_writes(&write_options);
        file.content_size = new_string.len() as u32;

        let Ok(new_c_string) = CString::new(new_string) else {
            error!("Error converting ini to CString after modifying {} writing unchanged version", file_name);
            return;
        };
        file.data_ptr = new_c_string.into_raw() as u32;
    })
}

pub fn modify_ztfile_as_animation<F>(file_name: &str, modifier: F) -> Result<(), &'static str>
where
    F: Fn(&mut Animation),
{
    modify_ztfile(file_name, |file: &mut BFResourcePtr| {
        info!("Modifying animation");
        let data_vec: Box<[u8]> = unsafe {
            Box::from_raw(slice::from_raw_parts_mut(
                file.data_ptr as *mut _,
                file.content_size as usize,
            ))
        };
        let mut animation = Animation::parse(&data_vec);
        modifier(&mut animation);
        let (new_animation_bytes, length) = animation.write();
        let boxed_slice = new_animation_bytes.into_boxed_slice();
        let data_ptr = boxed_slice.as_ptr() as u32;
        std::mem::forget(boxed_slice);
        file.data_ptr = data_ptr;
        file.content_size = length as u32;
    })
}

#[derive(Debug)]
#[repr(C)]
struct BFResourceMgr {
    resource_array_start: u32,
    resource_array_end: u32,
    resource_array_buffer_end: u32,
    unknown_u32_1: u32,
    unknown_u32_2: u32,
    unknown_u8_1: u8,
}

#[derive(Debug)]
#[repr(C)]
struct BFResourceDir {
    class: u32,
    unknown_u32_1: u32,
    dir_name_string_start: u32,
    dir_name_string_end: u32,
    unknown_u32_2: u32,
    num_child_files: u32,
    unknown_u32_3: u32,
}

#[derive(Debug)]
#[repr(C)]
struct BFResourceZip {
    class: u32,
    unknown_u32_1: u32,
    unknown_u32_2: u32,
    unknown_u32_3: u32,
    zip_name_string_start: u32,
    contents_tree: u32, //? contents end?
}

#[derive(Debug)]
#[repr(C)]
struct BFResourceDirContents {
    dir: BFResourceDir,
    zips: Vec<BFResourceZip>,
}

#[derive(Debug)]
#[repr(C)]
struct BFResource {
    bf_resource_ptr_ptr: u32,
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct BFResourcePtr {
    pub num_refs: u32,
    pub bf_zip_name_ptr: u32,
    pub bf_resource_name_ptr: u32,
    pub data_ptr: u32,
    pub content_size: u32,
}

#[derive(Debug)]
#[repr(C)]
struct GXLLEAnim {
    padding: [u8; 5],
    bfresource_maybe: u32,
}

impl fmt::Display for BFResourcePtr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "BFResourcePtr {{ num_refs: {:#x}, bf_zip_name: {}, bf_resource_name: {}, data_ptr: {:#x}, content_size: {:#x} }}",
            self.num_refs,
            get_string_from_memory(self.bf_zip_name_ptr),
            get_string_from_memory(self.bf_resource_name_ptr),
            self.data_ptr,
            self.content_size
        )
    }
}

trait Name {
    fn name(&self) -> String;
}

impl Name for BFResourceDir {
    fn name(&self) -> String {
        get_string_from_memory(self.dir_name_string_start)
    }
}

impl Name for BFResourceZip {
    fn name(&self) -> String {
        get_string_from_memory(self.zip_name_string_start)
    }
}

fn read_bf_resource_mgr_from_memory() -> BFResourceMgr {
    get_from_memory::<BFResourceMgr>(GLOBAL_BFRESOURCEMGR_ADDRESS)
}

fn read_bf_resource_dir_contents_from_memory() -> Vec<BFResourceDirContents> {
    info!("Reading BFResourceDir from memory");
    let bf_resource_mgr = read_bf_resource_mgr_from_memory();
    let mut bf_resource_dir_contents: Vec<BFResourceDirContents> = Vec::new();
    let mut bf_resource_dir_ptr = bf_resource_mgr.resource_array_start;
    let mut bf_resource_zips: Vec<BFResourceZip> = Vec::new();
    let mut current_bf_resource_dir = get_from_memory::<BFResourceDir>(get_from_memory::<u32>(bf_resource_dir_ptr));
    bf_resource_dir_ptr += 4;

    while bf_resource_dir_ptr < bf_resource_mgr.resource_array_end {
        let class = get_from_memory::<u32>(get_from_memory::<u32>(bf_resource_dir_ptr));
        match class {
            0x630aec => {
                bf_resource_dir_contents.push(BFResourceDirContents {
                    dir: current_bf_resource_dir,
                    zips: bf_resource_zips,
                });
                current_bf_resource_dir = get_from_memory::<BFResourceDir>(get_from_memory::<u32>(bf_resource_dir_ptr));
                bf_resource_zips = Vec::new();
                bf_resource_dir_ptr += 4;
            }
            0x630b0c => {
                bf_resource_zips.push(get_from_memory::<BFResourceZip>(get_from_memory::<u32>(bf_resource_dir_ptr)));
                bf_resource_dir_ptr += 4;
            }
            _ => {
                error!("Unknown class: 0x{:X}", class);
                bf_resource_dir_ptr += 4;
            }
        }
    }
    bf_resource_dir_contents.push(BFResourceDirContents {
        dir: current_bf_resource_dir,
        zips: bf_resource_zips,
    });
    bf_resource_dir_contents
}

impl fmt::Display for BFResourceMgr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "BFResourceMgr {{ resource_array_start: 0x{:X}, resource_array_end: 0x{:X}, resource_array_buffer_end: 0x{:X}, unknown_u32_1: 0x{:X}, unknown_u32_2: 0x{:X}, unknown_u8_1: 0x{:X} }}",
            self.resource_array_start, self.resource_array_end, self.resource_array_buffer_end, self.unknown_u32_1, self.unknown_u32_2, self.unknown_u8_1
        )
    }
}

impl fmt::Display for BFResourceDir {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let dir_name_string = get_string_from_memory(self.dir_name_string_start);
        write!(
            f,
            "BFResourceDir {{ class: 0x{:X}, unknown_u32_1: 0x{:X}, dir_name: {}, num_bfr_zip: 0x{:X}, unknown_u32_2: 0x{:X} }}",
            self.class, self.unknown_u32_1, dir_name_string, self.num_child_files, self.unknown_u32_2
        )
    }
}

impl fmt::Display for BFResourceZip {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let zip_name_string = get_string_from_memory(self.zip_name_string_start);
        write!(
            f,
            "BFResourceZip {{ class: 0x{:X}, unknown_u32_1: 0x{:X}, unknown_u32_2: 0x{:X}, unknown_u32_3: 0x{:X}, zip_name: {}, contents_tree: 0x{:X} }}",
            self.class, self.unknown_u32_1, self.unknown_u32_2, self.unknown_u32_3, zip_name_string, self.contents_tree
        )
    }
}

fn command_list_resources(_args: Vec<&str>) -> Result<String, CommandError> {
    let mut result_string = String::new();
    let bf_resource_dir_contents = read_bf_resource_dir_contents_from_memory();
    for bf_resource_dir_content in bf_resource_dir_contents {
        let bf_resource_dir = bf_resource_dir_content.dir;
        result_string.push_str(&format!(
            "{} ({})\n",
            get_string_from_memory(bf_resource_dir.dir_name_string_start),
            bf_resource_dir.num_child_files
        ));
        let bf_resource_zips = bf_resource_dir_content.zips;
        for bf_resource_zip in bf_resource_zips {
            result_string.push_str(&format!("{}\n", get_string_from_memory(bf_resource_zip.zip_name_string_start)));
        }
    }
    Ok(result_string)
}

fn command_get_bf_resource_mgr(_args: Vec<&str>) -> Result<String, CommandError> {
    let bf_resource_mgr = read_bf_resource_mgr_from_memory();
    Ok(format!("{}", bf_resource_mgr))
}

pub fn init() {
    add_to_command_register("list_resources".to_owned(), command_list_resources);
    add_to_command_register("get_bfresourcemgr".to_owned(), command_get_bf_resource_mgr);
    if unsafe { zoo_resource_mgr::init_detours() }.is_err() {
        error!("Failed to init resource_mgr detours");
    };
    add_to_command_register("list_resource_strings".to_string(), command_list_resource_strings);
    add_to_command_register("list_openzt_resource_strings".to_string(), command_list_openzt_resource_strings);
    add_to_command_register("list_openzt_mods".to_string(), command_list_openzt_mod_ids);
    add_to_command_register("list_openzt_locations_habitats".to_string(), command_list_openzt_locations_habitats);
}

#[hook_module("zoo.exe")]
pub mod zoo_resource_mgr {
    use bf_configparser::ini::Ini;
    use tracing::{info, span};

    use super::{check_file, get_file_ptr, get_location_or_habitat_by_id, load_resources, BFResourcePtr};
    use crate::debug_dll::{get_ini_path, get_string_from_memory, save_to_memory};

    #[hook(unsafe extern "thiscall" BFResource_attempt, offset = 0x00003891)]
    fn zoo_bf_resource_attempt(this_ptr: u32, file_name: u32) -> u8 {
        if bf_resource_inner(this_ptr, file_name) {
            return 1;
        }
        unsafe { BFResource_attempt.call(this_ptr, file_name) }
    }

    //47f4
    #[hook(unsafe extern "thiscall" BFResource_prepare, offset = 0x000047f4)]
    fn zoo_bf_resource_prepare(this_ptr: u32, file_name: u32) -> u8 {
        let string = get_string_from_memory(file_name);
        if bf_resource_inner(this_ptr, file_name) {
            return 1;
        }

        let return_value = unsafe { BFResource_prepare.call(this_ptr, file_name) };
        return_value
    }

    fn bf_resource_inner(this_ptr: u32, file_name: u32) -> bool {
        let mut file_name_string = get_string_from_memory(file_name).to_lowercase();
        if file_name_string.starts_with("openzt_resource") {
            match parse_openzt_resource_string(file_name_string.clone()) {
                Ok(resource_name) => {
                    file_name_string = resource_name;
                }
                Err(e) => {
                    info!("Failed to parse openzt resource string: {} {}", file_name_string, e);
                    return false;
                }
            }
        }
        if check_file(&file_name_string)
            && let Some(ptr) = get_file_ptr(&file_name_string)
        {
            let mut bfrp = unsafe { Box::from_raw(ptr as *mut BFResourcePtr) };

            bfrp.num_refs = 100;

            let ptr = Box::into_raw(bfrp) as u32;

            save_to_memory(this_ptr, ptr);
            true
        } else {
            false
        }
    }

    fn parse_openzt_resource_string(file_name: String) -> Result<String, &'static str> {
        if file_name.starts_with("openzt_resource") {
            let mut split = file_name.split('/').collect::<Vec<&str>>();
            if split.len() == 2 || split.len() == 3 {
                return Ok(split[1].to_owned());
            }
        }
        Err("Invalid openzt resource string")
    }

    #[hook(unsafe extern "thiscall" BFResourceMgr_constructor, offset = 0x0012903f)]
    fn zoo_bf_resource_mgr_constructor(this_ptr: u32) -> u32 {
        info!("BFResourceMgr::constructor({:X})", this_ptr);

        use std::time::Instant;
        let now = Instant::now();

        let return_value = unsafe { BFResourceMgr_constructor.call(this_ptr) };

        let elapsed = now.elapsed();
        info!("Vanilla loading took {:.2?}", elapsed);

        let ini_path = get_ini_path();
        let mut zoo_ini = Ini::new();
        zoo_ini.set_comment_symbols(&['#']);
        if let Err(e) = zoo_ini.load(ini_path) {
            info!("Failed to load zoo.ini: {}", e);
            return return_value;
        };
        if let Some(paths) = zoo_ini.get("resource", "path") {
            info!("Loading resources from: {}", paths);
            load_resources(paths.split(';').map(|s| s.to_owned()).collect());
            info!("Resources loaded");
        }
        return_value
    }

    #[hook(unsafe extern "cdecl" ZTUI_general_getInfoImageName, offset = 0x000f85d2)]
    fn zoo_ui_general_get_info_image_name(id: u32) -> u32 {
        let return_value = match get_location_or_habitat_by_id(id) {
            Some(resource_ptr) => resource_ptr,
            None => unsafe { ZTUI_general_getInfoImageName.call(id) },
        };
        return_value
    }
}

///Indicates when the handler should be called
/// BeforeOpenZTMods means they are run on any files in but before any OpenZT mods are loaded
/// AfterOpenZTMods is the same as above but after OpenZT mods are loaded
/// AfterFiltering is run on files that are referenced in *.cfg files only
#[derive(Clone, PartialEq)]
pub enum RunStage {
    BeforeOpenZTMods,
    AfterOpenZTMods,
    AfterFiltering
}

pub type IniHandlerFunction = fn(&String, &String, Ini) -> Option<(String, String, Ini)>;
pub type AnimationHandlerFunction = fn(&String, &String, Animation) -> Option<(String, String, Animation)>;
pub type RawBytesHandlerFunction = fn(&String, &String, Box<[u8]>) -> Option<(String, String, Box<[u8]>)>;

pub struct HandlerBuilder<const HasStage: bool, const HasHandler: bool> {
    matcher_prefix: Option<String>,
    matcher_suffix: Option<String>,
    ini_handler: Option<IniHandlerFunction>,
    animation_handler: Option<AnimationHandlerFunction>,
    raw_bytes_handler: Option<RawBytesHandlerFunction>,
    stage: Option<RunStage>,
}

impl<const HasStage: bool, const HasHandler: bool> HandlerBuilder<HasStage, HasHandler> {
    pub fn prefix(mut self, prefix: &str) -> HandlerBuilder<HasStage, HasHandler> {
        self.matcher_prefix = Some(prefix.to_owned());
        self
    }

    pub fn suffix(mut self, suffix: &str) -> HandlerBuilder<HasStage, HasHandler> {
        self.matcher_suffix = Some(suffix.to_owned());
        self
    }

    pub fn ini_handler(self, handler: IniHandlerFunction) -> HandlerBuilder<HasStage, true> {
        HandlerBuilder {
            matcher_prefix: self.matcher_prefix,
            matcher_suffix: self.matcher_suffix,
            ini_handler: Some(handler),
            animation_handler: self.animation_handler,
            raw_bytes_handler: self.raw_bytes_handler,
            stage: self.stage
        }
    }

    pub fn animation_handler(self, handler: AnimationHandlerFunction) -> HandlerBuilder<HasStage, true> {
        HandlerBuilder {
            matcher_prefix: self.matcher_prefix,
            matcher_suffix: self.matcher_suffix,
            ini_handler: self.ini_handler,
            animation_handler: Some(handler),
            raw_bytes_handler: self.raw_bytes_handler,
            stage: self.stage
        }
    }

    pub fn raw_bytes_handler(self, handler: RawBytesHandlerFunction) -> HandlerBuilder<HasStage, true> {
        HandlerBuilder {
            matcher_prefix: self.matcher_prefix,
            matcher_suffix: self.matcher_suffix,
            ini_handler: self.ini_handler,
            animation_handler: self.animation_handler,
            raw_bytes_handler: Some(handler),
            stage: self.stage
        }
    }

    pub fn run_stage(self, stage: RunStage) -> HandlerBuilder<true, HasHandler> {
        HandlerBuilder {
            matcher_prefix: self.matcher_prefix,
            matcher_suffix: self.matcher_suffix,
            ini_handler: self.ini_handler,
            animation_handler: self.animation_handler,
            raw_bytes_handler: self.raw_bytes_handler,
            stage: Some(stage)
        }
    }
}

impl HandlerBuilder<true, true> {
    pub fn build(self) -> Handler {
        unsafe {
            Handler {
                matcher_prefix: self.matcher_prefix,
                matcher_suffix: self.matcher_suffix,
                ini_handler: self.ini_handler,
                animation_handler: self.animation_handler,
                raw_bytes_handler: self.raw_bytes_handler,
                stage: self.stage.unwrap_unchecked()
            }
        }
    }
}

#[derive(Clone)]
pub struct Handler {
    matcher_prefix: Option<String>,
    matcher_suffix: Option<String>,
    ini_handler: Option<IniHandlerFunction>,
    animation_handler: Option<AnimationHandlerFunction>,
    raw_bytes_handler: Option<RawBytesHandlerFunction>,
    stage: RunStage
}

impl Handler {
    pub fn builder() -> HandlerBuilder<false, false> {
        HandlerBuilder::<false, false> {
            matcher_prefix: None,
            matcher_suffix: None,
            ini_handler: None,
            animation_handler: None,
            raw_bytes_handler: None,
            stage: None
        }
    }

    fn handle(&self, file_name: &String) {
        if let Some(prefix) = &self.matcher_prefix {
            if !file_name.starts_with(prefix) {
                return;
            }
        }
        if let Some(suffix) = &self.matcher_suffix {
            if !file_name.ends_with(suffix) {
                return;
            }
        }

        let new_file = match ZTFileType::from(Path::new(&file_name)) {
            ZTFileType::Ini | ZTFileType::Ai | ZTFileType::Ani | ZTFileType::Cfg | ZTFileType::Lyt | ZTFileType::Scn | ZTFileType::Uca | ZTFileType::Ucs | ZTFileType::Ucb => {
                if let Some(handler) = self.ini_handler {
                    let Some((archive_name, file)) = get_file(file_name) else {
                        error!("Error getting file: {}", file_name);
                        return;
                    };
                    let mut ini = Ini::new();
                    let Ok(input_string) = str::from_utf8(&file) else {
                        error!("Error converting file to string: {}", file_name);
                        return;
                    };
                    ini.read(input_string.to_string());
                    if let Some((new_file_name, new_file_path, new_ini)) = handler(&archive_name, file_name, ini) {
                        
                    }
                }
            }
            ZTFileType::Animation => {
                if let Some(handler) = self.animation_handler {
                    let Some((archive_name, file)) = get_file(file_name) else {
                        error!("Error getting file: {}", file_name);
                        return;
                    };
                    let animation = Animation::parse(&file);
                    if let Some((new_file_name, new_file_path, new_animation)) = handler(&archive_name, file_name, animation) {
                        
                    }
                }
            }
            ZTFileType::Bmp | ZTFileType::Lle | ZTFileType::TGA | ZTFileType::Wav | ZTFileType::Palette => {
                if let Some(handler) = self.raw_bytes_handler {
                    let Some((archive_name, mut file)) = get_file(file_name) else {
                        error!("Error getting file: {}", file_name);
                        return;
                    };
                    if let Some((new_file_name, new_file_path, new_data)) = handler(&archive_name, file_name, file) {

                    }
                }
            }

            _ => return
        };
        // TODO: Write file back to LazyResourceMap, with modified flag set

    }
}

fn get_file(file_name: &str) -> Option<(String, Box<[u8]>)> {
    let mut map = LAZY_RESOURCE_MAP.lock().unwrap();
    match map.get(&file_name.to_lowercase()) {
        Ok(Some(file)) => {
            let resource_ptr = get_from_memory::<BFResourcePtr>(file.data);
            let tmp_slice = unsafe {slice::from_raw_parts(resource_ptr.data_ptr as *const _, resource_ptr.content_size as usize) };
            let mut new_slice = vec![0; resource_ptr.content_size as usize];
            new_slice.copy_from_slice(tmp_slice);
            Some((get_string_from_memory(resource_ptr.bf_zip_name_ptr), new_slice.into_boxed_slice()))
        },
        _ => None
    }
}

// Note: We are excluding ztat* files until we need to override anything inside them, as they have a rediculous amount of files
fn get_ztd_resources(dir: &Path, recursive: bool) -> Vec<PathBuf> {
    let mut resources = Vec::new();
    if !dir.is_dir() {
        return resources;
    }
    let walker = WalkDir::new(dir).follow_links(true).max_depth(if recursive { 0 } else { 1 });
    for entry in walker {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                error!("Error walking directory: {}", e);
                continue;
            }
        };
        let Some(filename) = entry.file_name().to_str() else {
            error!("Error getting filename: {:?}", entry);
            continue;
        };
        if filename.to_lowercase().ends_with(".ztd") && !filename.starts_with("ztat") {
            resources.push(entry.path().to_path_buf());
        }
    }
    resources
}

// struct ZipFileReference {
//     zip: Arc<Mutex<ZipArchive<BufReader<File>>>>,
//     filename: String,
// }

// impl ZipFileReference {
//     fn new(zip: Arc<Mutex<ZipArchive<BufReader<File>>>>, filename: String) -> Self {
//         Self { zip, filename }
//     }
// }


static LAZY_RESOURCE_MAP: Lazy<Mutex<LazyResourceMap>> = Lazy::new(|| Mutex::new(LazyResourceMap::new()));

pub struct LazyResourceMap {
    map: HashMap<String, LazyResource>,
}

#[derive(Clone)]
enum ResourceBacking {
    LazyZipFile{archive_name: String, archive: Arc<Mutex<ZipArchive<BufReader<File>>>>},
    LoadedZipFile{archive_name: String, archive: Arc<Mutex<ZipArchive<BufReader<File>>>>, data: u32},
    Custom{data: u32},
}

struct ConcreteResource {
    archive_name: Option<String>,
    filename: String,
    type_: ZTFileType,
    data: u32,
}

struct LazyResource {
    pub backing: ResourceBacking,
    // pub archive_path: String,
    pub filename: String,
    // pub archive: Option<Arc<Mutex<ZipArchive<BufReader<File>>>>>,
    pub type_: ZTFileType,
    // pub modified: bool,
    // pub data: Option<u32>, // a pointer to a BFResourcePtr
}

impl LazyResourceMap {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
    
    // TODO: Unload old resource if it exists
    fn insert_lazy(&mut self, archive_path: String, filename: String, archive: Arc<Mutex<ZipArchive<BufReader<File>>>>) {
        // let modified = self.map.get(&filename).is_some();

        self.map.insert(filename.clone(), LazyResource {
            backing: ResourceBacking::LazyZipFile{archive_name: archive_path.clone(), archive},
            // archive_path,
            filename: filename.clone(),
            // Some(archive),
            type_: ZTFileType::from(Path::new(&filename)),
            // data: None,
        });
    }

    // TODO: Unload old resource if it exists
    fn insert_loaded(&mut self, resource: LazyResource) {
        self.map.insert(resource.filename.clone(), resource);
    }

    fn insert_custom(&mut self, filename: String, data: u32) {
        self.map.insert(filename.clone(), LazyResource {
            backing: ResourceBacking::Custom{data},
            filename: filename.clone(),
            type_: ZTFileType::from(Path::new(&filename)),
        });
    }

    fn get(&mut self, key: &str) -> anyhow::Result<Option<ConcreteResource>> {
        let Some(resource) = self.map.get_mut(key) else {
            return Ok(None);
        };

        // TODO: Use std::mem::take/replace to avoid cloning
        let (archive_name, data) = match resource.backing.clone() {
            ResourceBacking::LazyZipFile{archive_name, archive} => {
                let mut binding = archive.lock().unwrap();
                let mut file = binding.by_name(&resource.filename).with_context(|| format!("Error finding file in archive: {}", resource.filename))?;
                let mut file_buffer = vec![0u8; file.size() as usize].into_boxed_slice();

                file.read_exact(&mut file_buffer).with_context(|| format!("Error reading file: {}", resource.filename))?;
                let ztfile = ZTFile::new(resource.filename.clone(), file_buffer.len() as u32, file_buffer)?;
                let data = ztfile_to_raw_resource(&archive_name, resource.filename.clone(), ztfile)?;
                resource.backing = ResourceBacking::LoadedZipFile{archive_name: archive_name.clone(), archive: archive.clone(), data: data.clone()};
                (Some(archive_name), data)
            },
            ResourceBacking::LoadedZipFile{archive_name, archive, data} => (Some(archive_name), data),
            ResourceBacking::Custom{data} => (None, data),
        };

        Ok(Some(ConcreteResource{
            archive_name: archive_name.clone(),
            filename: resource.filename.clone(),
            type_: resource.type_.clone(),
            data: data.clone(),
        }))
    }

    fn loaded_len(&self) -> usize {
        self.map.values().filter(|x| matches!(x.backing, ResourceBacking::LoadedZipFile{..} | ResourceBacking::Custom{..})).count()
    }

    fn not_loaded_len(&self) -> usize {
        self.map.values().filter(|x| matches!(x.backing, ResourceBacking::LazyZipFile{..})).count()
    }

    fn len(&self) -> usize {
        self.map.len()
    }

    fn contains_key(&self, key: &str) -> bool {
        self.map.contains_key(key)
    }

    fn files(&self) -> Box<dyn Iterator<Item = String> + '_> {
        Box::new(self.map.keys().cloned())
    }
}

pub fn check_file(file_name: &str) -> bool {
    let binding = LAZY_RESOURCE_MAP.lock().unwrap();
    binding.contains_key(&file_name.to_lowercase())
}

pub fn get_file_ptr(file_name: &str) -> Option<u32> {
    let mut binding = LAZY_RESOURCE_MAP.lock().unwrap();
    if let Ok(Some(resource)) = binding.get(&file_name.to_lowercase()) {
        Some(resource.data)
    } else {
        None
    }
}

fn load_resources(paths: Vec<String>) {
    use std::time::Instant;
    let now = Instant::now();
    let mut resource_count = 0;

    let mut map = LAZY_RESOURCE_MAP.lock().unwrap();

    paths.iter().rev().for_each(|path| {
        let resources = get_ztd_resources(Path::new(path), false);
        resources.iter().for_each(|resource| {
            info!("Loading resource: {}", resource.display());
            let file_name = resource.to_str().unwrap_or_default().to_lowercase();
            if file_name.ends_with(".ztd") {
                match handle_ztd(&mut map, resource) {
                    Ok(count) => resource_count += count,
                    Err(err) => error!("Error loading ztd: {} -> {}", file_name, err),
                }
            }
        });
    });

    let elapsed = now.elapsed();
    info!(
        "Loaded {} mods and {} ({}) resources in: {:.2?}",
        get_num_mod_ids(),
        map.len(),
        resource_count,
        elapsed
    );

    let now = Instant::now();

    let data_mutex = RESOURCE_HANDLER_ARRAY.lock().unwrap();

    let files = map.files().collect::<Vec<String>>().into_iter();

    // TODO: Don't get files here, get them in handler.handle /WHEN/ we know we need to actually load them
    // Should just get file names here, re-lock the LazyResourceMap later
    for handler in data_mutex.iter() {
        if handler.stage == RunStage::BeforeOpenZTMods {
            files.clone().for_each(|file| {
                handler.handle(&file);
            });
        }
    }

    // apply_patches();

    for handler in data_mutex.iter() {
        if handler.stage == RunStage::AfterOpenZTMods {
            files.clone().for_each(|file| {
                handler.handle(&file);
            });
        }
    }

    let mut filtered_files = Vec::new();

    files.clone().for_each(|file| {
        if file.ends_with(".cfg") {
            let inner_filtered = parse_cfg(&file);
            filtered_files.extend(inner_filtered);
        }
    });

    for handler in data_mutex.iter() {
        if handler.stage == RunStage::AfterFiltering {
            filtered_files.clone().into_iter().for_each(|file| {
               handler.handle(&file);
            });
        }
    }
    
    let elapsed = now.elapsed();
    info!(
        "Extra handling took an extra: {:.2?}",
        elapsed
    );
}

fn handle_ztd(map: &mut LazyResourceMap, resource: &PathBuf) -> anyhow::Result<i32> {
    let mut load_count = 0;
    let file = File::open(resource).with_context(|| format!("Error opening file: {}", resource.display()))?;

    let resource_string = resource
        .clone()
        .into_os_string()
        .into_string()
        .map_err(|e| anyhow::anyhow!("error converting resource path to string: {}", e.to_string_lossy()))?;

    let buf_reader = BufReader::new(file);

    let mut zip = zip::ZipArchive::new(buf_reader).with_context(|| format!("Error reading zip: {}", resource.display()))?;

    let ztd_type = load_open_zt_mod(map, &mut zip)?;

    if ztd_type == mods::ZtdType::Openzt {
        return Ok(0);
    }

    let archive = Arc::new(Mutex::new(zip));

    archive.lock().unwrap().file_names().for_each(|file_name| {
        map.insert_lazy(resource_string.clone(), file_name.to_string(), archive.clone());
        load_count += 1;
    });

    Ok(load_count)
}


fn parse_cfg(file_name: &String) -> Vec<String> {
    let results = Vec::new();

    return results;
}

#[derive(Debug)]
enum LegacyCfgType {
    Ambient,
    Animal,
    Building,
    Fence,
    Filter,
    Food,
    Free,
    Fringe,
    Guest,
    Help,
    Item,
    Path,
    Rubble,
    Scenario,
    Scenery,
    Staff,
    Tile,
    Wall,
    Expansion,
    Show,
    Tank,
    UIInfoImage,
    Economy,
}

#[derive(Debug)]
struct LegacyCfg {
    cfg_type: LegacyCfgType,
    file_name: String,
}

fn map_legacy_cfg_type(file_type_str: &str, file_name: String) -> Result<LegacyCfg, String> {
    match file_type_str {
        "ambient" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::Ambient,
            file_name,
        }),
        "animal" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::Animal,
            file_name,
        }),
        "bldg" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::Building,
            file_name,
        }),
        "fences" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::Fence,
            file_name,
        }),
        "filter" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::Filter,
            file_name,
        }),
        "food" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::Food,
            file_name,
        }),
        "free" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::Free,
            file_name,
        }),
        "fringe" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::Fringe,
            file_name,
        }),
        "guests" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::Guest,
            file_name,
        }),
        "help" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::Help,
            file_name,
        }),
        "items" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::Item,
            file_name,
        }),
        "paths" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::Path,
            file_name,
        }),
        "rubble" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::Rubble,
            file_name,
        }),
        "scenar" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::Scenario,
            file_name,
        }),
        "scener" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::Scenery,
            file_name,
        }),
        "staff" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::Staff,
            file_name,
        }),
        "tile" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::Tile,
            file_name,
        }),
        "twall" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::Wall,
            file_name,
        }),
        "xpac" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::Expansion,
            file_name,
        }),
        "shows" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::Show,
            file_name,
        }),
        "tanks" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::Tank,
            file_name,
        }),
        "ui/infoimg" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::UIInfoImage,
            file_name,
        }),
        "economy" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::Economy,
            file_name,
        }),
        _ => Err(format!("Unknown legacy cfg type: {}", file_type_str)),
    }
}

static LEGACY_CFG_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^((ambient|animal|bldg|fences|filter|food|free|fringe|guests|help|items|paths|rubble|scenar|scener|staff|tile|twall|xpac)[\w\-. ]*?\.cfg)|((shows|tanks|ui\/infoimg|economy)\.cfg)$")
        .unwrap()
});

fn get_legacy_cfg_type(file_name: &String) -> Option<LegacyCfg> {
    let capture = LEGACY_CFG_REGEX.captures(file_name)?;
    match capture.iter().collect::<Vec<_>>().as_slice() {
        [_, Some(file_name), Some(file_type), None, None] => {
            map_legacy_cfg_type(&file_type.as_str(), file_name.as_str().to_string()).ok()
        }
        [_, None, None, Some(file_name), Some(file_type)] => {
            map_legacy_cfg_type(&file_type.as_str(), file_name.as_str().to_string()).ok()
        }
        _ => {
            None
        }
    }
}

// TODO: Make some ZipFile and ZipArchive wrappers and add these functions to them
fn read_file_from_zip(zip: &mut ZipArchive<BufReader<File>>, file_name: &str) -> anyhow::Result<Box<[u8]>> {
    let mut file = zip.by_name(file_name).with_context(|| format!("Error finding file in archive: {}", file_name))?;

    let mut file_buffer = vec![0u8; file.size() as usize].into_boxed_slice();

    file.read_exact(&mut file_buffer).with_context(|| format!("Error reading file: {}", file_name))?;

    Ok(file_buffer)
}

fn read_file_from_zip_to_string(zip: &mut ZipArchive<BufReader<File>>, file_name: &str) -> anyhow::Result<String> {
    let mut buffer = read_file_from_zip(zip, file_name)?;

    Ok(str::from_utf8(&buffer)
        .with_context(|| format!("Error converting file {} to utf8", file_name))?
        .to_string())
}

fn zip_file_to_string(mut zip: ZipFile) -> anyhow::Result<String> {
    let mut buffer = vec![0u8; zip.size() as usize].into_boxed_slice();
    zip.read_exact(&mut buffer).with_context(|| format!("Error reading file: {}", zip.name()))?;

    Ok(str::from_utf8(&buffer)
        .with_context(|| format!("Error converting file {} to utf8", zip.name()))?
        .to_string())
}

fn load_open_zt_mod(map: &mut LazyResourceMap, archive: &mut ZipArchive<BufReader<File>>) -> anyhow::Result<mods::ZtdType> {
    if archive.by_name("meta.toml").is_err() {
        return Ok(mods::ZtdType::Legacy);
    }

    let meta = toml::from_str::<mods::Meta>(&read_file_from_zip_to_string(archive, "meta.toml")?).with_context(|| "Failed to parse meta.toml")?;

    if meta.ztd_type() == &mods::ZtdType::Legacy {
        return Ok(mods::ZtdType::Legacy);
    }

    let mod_id = meta.mod_id().to_string();

    if !add_new_mod_id(&mod_id) {
        return Err(anyhow!("Mod already loaded: {}", mod_id));
    }

    info!("Loading OpenZT mod: {} {}", meta.name(), meta.mod_id());

    let mut file_map: HashMap<String, Box<[u8]>> = HashMap::new();

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            // TODO: Create type that wraps ZipArchive and provide archive name for better error reporting
            .with_context(|| format!("Error reading zip file at index {}", i))?;

        if file.is_dir() {
            continue;
        }
        let file_name = file.name().to_string();

        let mut file_buffer = vec![0; file.size() as usize].into_boxed_slice();
        file.read_exact(&mut file_buffer).with_context(|| format!("Error reading file: {}", file_name))?;

        file_map.insert(file_name, file_buffer);
    }

    let keys = file_map.keys().clone();

    for file_name in keys {
        if file_name.starts_with("defs/") {
            load_def(&mod_id, &file_name, &file_map)?;
        }
    }

    Ok(meta.ztd_type().clone())
}

// Map between the id ZT uses to reference locations/habitats and the string ptr of the animation (icon) resource
static LOCATIONS_HABITATS_RESOURCE_MAP: Lazy<Mutex<HashMap<u32, u32>>> = Lazy::new(|| Mutex::new(HashMap::new()));

// Map between the animation (icon resource) and the id ZT uses to reference location/habitats, this is used to lookup the id needed to add the habitat/location to an animal
static LOCATIONS_HABITATS_ID_MAP: Lazy<Mutex<HashMap<String, u32>>> = Lazy::new(|| Mutex::new(HashMap::new()));

// Used to ensure mod_ids don't clash, a mod will not load if an id is already in this map
static MOD_ID_SET: Lazy<Mutex<HashSet<String>>> = Lazy::new(|| Mutex::new(HashSet::new()));

fn command_list_openzt_mod_ids(_args: Vec<&str>) -> Result<String, CommandError> {
    let binding = MOD_ID_SET.lock().unwrap();
    let mut result_string = String::new();
    for mod_id in binding.iter() {
        result_string.push_str(&format!("{}\n", mod_id));
    }
    Ok(result_string)
}

fn command_list_openzt_locations_habitats(_args: Vec<&str>) -> Result<String, CommandError> {
    let binding = LOCATIONS_HABITATS_RESOURCE_MAP.lock().unwrap();
    let mut result_string = String::new();
    for (id, _) in binding.iter() {
        let name = get_string_from_registry(*id).unwrap_or("<error>".to_string());
        result_string.push_str(&format!("{} {}\n", id, name));
    }
    Ok(result_string)
}

fn add_location_or_habitat(name: &String, icon_resource_id: &String) -> anyhow::Result<()> {
    let mut resource_binding = LOCATIONS_HABITATS_RESOURCE_MAP.lock().unwrap();

    let mut id_binding = LOCATIONS_HABITATS_ID_MAP.lock().unwrap();

    let string_id = add_string_to_registry(name.clone());

    info!("Adding location/habitat: {} {} -> {}", name, icon_resource_id, string_id);

    let icon_resource_id_cstring = CString::new(icon_resource_id.clone())
        .with_context(|| format!("Failed to create cstring for location/habitat {} with icon_resource_id {}", name, icon_resource_id))?;
    resource_binding.insert(string_id, icon_resource_id_cstring.into_raw() as u32);
    id_binding.insert(name.clone(), string_id);

    Ok(())
}

fn get_location_or_habitat_by_id(id: u32) -> Option<u32> {
    let binding = LOCATIONS_HABITATS_RESOURCE_MAP.lock().unwrap();
    binding.get(&id).cloned()
}

fn get_location_or_habitat_by_name(name: &String) -> Option<u32> {
    let binding = LOCATIONS_HABITATS_ID_MAP.lock().unwrap();
    binding.get(name).cloned()
}

// Adds a new mod id to the set, returns false if the mod_id already exists
fn add_new_mod_id(mod_id: &String) -> bool {
    let mut binding = MOD_ID_SET.lock().unwrap();
    binding.insert(mod_id.clone())
}

enum ResourceType {
    Location,
    Habitat,
}

impl Display for ResourceType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ResourceType::Location => write!(f, "location"),
            ResourceType::Habitat => write!(f, "habitat"),
        }
    }
}

enum ZTResourceType {
    Animation,
    Ani,
    Palette,
}

impl Display for ZTResourceType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ZTResourceType::Animation => write!(f, "animation"),
            ZTResourceType::Palette => write!(f, "palette"),
            ZTResourceType::Ani => write!(f, "ani"),
        }
    }
}

fn load_def(mod_id: &String, file_name: &String, file_map: &HashMap<String, Box<[u8]>>) -> anyhow::Result<mods::ModDefinition> {
    info!("Loading defs {} from {}", file_name, mod_id);

    let file = file_map
        .get(file_name)
        .with_context(|| format!("Error finding file {} in resource map for mod {}", file_name, mod_id))?;

    let mut intermediate_string = str::from_utf8(file)
        .with_context(|| format!("Error converting file {} to utf8 for mod {}", file_name, mod_id))?
        .to_string();

    let defs = toml::from_str::<mods::ModDefinition>(&intermediate_string).with_context(|| format!("Error parsing defs from OpenZT mod: {}", file_name))?;

    info!("Loading defs: {}", defs.len());

    // Habitats
    if let Some(habitats) = defs.habitats() {
        for (habitat_name, habitat_def) in habitats.iter() {
            let base_resource_id = openzt_base_resource_id(&mod_id, ResourceType::Habitat, habitat_name);
            let icon_name = load_icon_definition(
                &base_resource_id,
                habitat_def,
                &file_map,
                mod_id,
                include_str!("../resources/include/infoimg-habitat.ani").to_string(),
            )?;
            add_location_or_habitat(&habitat_def.name(), &base_resource_id);
        }
    }

    // Locations
    if let Some(locations) = defs.locations() {
        for (location_name, location_def) in locations.iter() {
            let base_resource_id = openzt_base_resource_id(&mod_id, ResourceType::Location, location_name);
            let icon_name = load_icon_definition(
                &base_resource_id,
                location_def,
                &file_map,
                mod_id,
                include_str!("../resources/include/infoimg-location.ani").to_string(),
            )?;
            add_location_or_habitat(&location_def.name(), &base_resource_id);
        }
    }
    Ok(defs)
}

fn load_icon_definition(
    base_resource_id: &String,
    icon_definition: &mods::IconDefinition,
    file_map: &HashMap<String, Box<[u8]>>,
    mod_id: &String,
    base_config: String,
) -> anyhow::Result<String> {
    let icon_file = file_map.get(icon_definition.icon_path()).with_context(|| {
        format!(
            "Error loading openzt mod {}, cannot find file {} for icon_def {}",
            mod_id,
            icon_definition.icon_path(),
            icon_definition.name()
        )
    })?;

    let icon_file_palette = file_map.get(icon_definition.icon_palette_path()).with_context(|| {
        format!(
            "Error loading openzt mod {}, cannot find file {} for icon_def {}",
            mod_id,
            icon_definition.icon_palette_path(),
            icon_definition.name()
        )
    })?;

    let mut animation = Animation::parse(icon_file);
    animation.set_palette_filename(icon_definition.icon_palette_path().clone());
    let (new_animation_bytes, icon_size) = animation.write();
    let new_icon_file = new_animation_bytes.into_boxed_slice();

    let mut ani_cfg = Ini::new_cs();
    ani_cfg.set_comment_symbols(&[';', '#', ':']);
    ani_cfg.read(base_config).map_err(|s| anyhow!("Error reading ini: {}", s))?;

    if ani_cfg.set("animation", "dir1", Some(openzt_full_resource_id_path(&base_resource_id, ZTResourceType::Animation))) == None {
        return Err(anyhow!("Error setting dir1 for ani"));
    }

    let mut write_options = WriteOptions::default();
    write_options.space_around_delimiters = true;
    write_options.blank_lines_between_sections = 1;
    let new_string = ani_cfg.pretty_writes(&write_options);
    info!("New ani: \n{}", new_string);
    let file_size = new_string.len() as u32;
    let file_name = openzt_full_resource_id_path(&base_resource_id, ZTResourceType::Ani);

    let Ok(new_c_string) = CString::new(new_string) else {
        return Err(anyhow!(
            "Error loading openzt mod {} when converting .ani to CString after modifying {}",
            mod_id,
            file_name
        ));
    };

    let ztfile = ZTFile::new_text(file_name.clone(), file_size, new_c_string)
        .with_context(|| format!("Error loading openzt mod {} when creating ZTFile for .ani after modifying {}", mod_id, file_name))?;

    add_ztfile(Path::new("zip::./openzt.ztd"), file_name, ztfile);

    let animation_file_name = openzt_full_resource_id_path(&base_resource_id, ZTResourceType::Animation);
    let animation_ztfile = ZTFile::new_raw_bytes(animation_file_name.clone(), icon_size as u32, new_icon_file);

    add_ztfile(Path::new("zip::./openzt.ztd"), animation_file_name.clone(), animation_ztfile);

    let palette_file_name = openzt_full_resource_id_path(&base_resource_id, ZTResourceType::Palette);
    let palette_ztfile = ZTFile::new_raw_bytes(palette_file_name.clone(), icon_file_palette.len() as u32, icon_file_palette.clone());
    add_ztfile(Path::new("zip::./openzt.ztd"), palette_file_name, palette_ztfile);

    Ok(animation_file_name)
}

fn openzt_base_resource_id(mod_id: &String, resource_type: ResourceType, resource_name: &String) -> String {
    let resource_type_name = resource_type.to_string();
    format!("openzt.mods.{}.{}.{}", mod_id, resource_type_name, resource_name)
}

fn openzt_full_resource_id_path(base_resource_id: &String, file_type: ZTResourceType) -> String {
    format!("{}.{}", base_resource_id, file_type.to_string())
}

static RESOURCE_HANDLER_ARRAY: Lazy<Mutex<Vec<Handler>>> = Lazy::new(|| Mutex::new(Vec::new()));

pub fn add_handler(handler: Handler) {
    let mut data_mutex = RESOURCE_HANDLER_ARRAY.lock().unwrap();
    data_mutex.push(handler);
}

fn get_handlers() -> Vec<Handler> {
    RESOURCE_HANDLER_ARRAY.lock().unwrap().clone()
}
