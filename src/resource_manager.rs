use core::slice;
use std::{
    collections::{HashMap, HashSet},
    ffi::CString,
    fmt,
    fs::File,
    io,
    io::{BufReader, Read},
    path::{Path, PathBuf},
    sync::Mutex,
};

use bf_configparser::ini::{Ini, WriteOptions};
use once_cell::sync::Lazy;
use retour_utils::hook_module;
use tracing::{error, info};
use walkdir::WalkDir;
use zip::read::ZipFile;

use crate::{
    animation::Animation,
    console::add_to_command_register,
    debug_dll::{get_from_memory, get_string_from_memory, save_to_memory},
    console::CommandError,
};

const GLOBAL_BFRESOURCEMGR_ADDRESS: u32 = 0x006380C0;

#[derive(Debug, Clone)]
pub enum ZTFile {
    Text(CString, ZTFileType, u32),
    Graphics(Box<[u8]>, ZTFileType, u32),
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
    Animation,
    Palette,
    TGA,
    Wav,
    Lle,
    Bmp,
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
            "tga" => ZTFile::Graphics(
                unsafe {
                    Box::from_raw(slice::from_raw_parts_mut(
                        data as *mut _,
                        file_size as usize,
                    ))
                },
                ZTFileType::TGA,
                file_size,
            ),
            "pal" => ZTFile::Graphics(
                unsafe {
                    Box::from_raw(slice::from_raw_parts_mut(
                        data as *mut _,
                        file_size as usize,
                    ))
                },
                ZTFileType::Palette,
                file_size,
            ),
            "wav" => ZTFile::Graphics(
                unsafe {
                    Box::from_raw(slice::from_raw_parts_mut(
                        data as *mut _,
                        file_size as usize,
                    ))
                },
                ZTFileType::Wav,
                file_size,
            ),
            "lle" => ZTFile::Graphics(
                unsafe {
                    Box::from_raw(slice::from_raw_parts_mut(
                        data as *mut _,
                        file_size as usize,
                    ))
                },
                ZTFileType::Lle,
                file_size,
            ),
            "bmp" => ZTFile::Graphics(
                unsafe {
                    Box::from_raw(slice::from_raw_parts_mut(
                        data as *mut _,
                        file_size as usize,
                    ))
                },
                ZTFileType::Bmp,
                file_size,
            ),
            _ => ZTFile::Graphics(
                unsafe {
                    Box::from_raw(slice::from_raw_parts_mut(
                        data as *mut _,
                        file_size as usize,
                    ))
                },
                ZTFileType::Animation,
                file_size,
            ),
        }
    }
}

impl ZTFile {
    pub fn new_text(
        file_name: String,
        file_size: u32,
        data: CString,
    ) -> Result<ZTFile, &'static str> {
        let file_extension = Path::new(&file_name)
            .extension()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default();
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
            _ => Err("Invalid file type"),
        }
    }

    pub fn new_raw_bytes(file_name: String, file_size: u32, data: Box<[u8]>) -> ZTFile {
        let file_extension = Path::new(&file_name)
            .extension()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default();
        match file_extension {
            "tga" => ZTFile::Graphics(data, ZTFileType::TGA, file_size),
            "pal" => ZTFile::Graphics(data, ZTFileType::Palette, file_size),
            "wav" => ZTFile::Graphics(data, ZTFileType::Wav, file_size),
            "lle" => ZTFile::Graphics(data, ZTFileType::Lle, file_size),
            "bmp" => ZTFile::Graphics(data, ZTFileType::Bmp, file_size),
            _ => ZTFile::Graphics(data, ZTFileType::Animation, file_size),
        }
    }
}

pub trait FromZipFile<T> {
    fn from_zip_file(file: &mut ZipFile) -> io::Result<T>;
}

// impl FromZipFile<String> for String {
//     fn from_zip_file(file: &mut ZipFile) -> io::Result<String> {
//         let mut buffer = vec![0; file.size() as usize];
//         file.read(&mut buffer[..])?;
//         Ok(String::from_utf8_lossy(&buffer[..]).to_string())
//     }
// }

// impl FromZipFile<Vec<u8>> for Vec<u8> {
//     fn from_zip_file(file: &mut ZipFile) -> io::Result<Self> {
//         let mut buffer = vec![0; file.size() as usize];
//         file.read(&mut buffer[..])?;
//         Ok(buffer)
//     }
// }

// impl FromZipFile<CString> for CString {
//     fn from_zip_file(file: &mut ZipFile) -> io::Result<Self> {
//         let mut buffer = vec![0; file.size() as usize];
//         file.read(&mut buffer[..])?;
//         Ok(CString::new(
//             String::from_utf8_lossy(&buffer[..]).to_string(),
//         )?)
//     }
// }

fn add_file_to_maps(entry: &Path, file: &mut ZipFile) {
    let lowercase_file_name = file.name().to_lowercase();
    if check_file(&lowercase_file_name) {
        // File already exists, skip loading
        return;
    }
    // TODO: Figure out issues with loading ini, txt and non-text files
    // NOTE: Non-text files seem to work fine when not using mods
    let file_extension = Path::new(&lowercase_file_name)
        .extension()
        .unwrap_or_default()
        .to_str()
        .unwrap_or_default();
    if matches!(
        file_extension,
        "ai" | "ani" | "cfg" | "lyt" | "scn" | "uca" | "ucs" | "ucb"
    ) {
        // | "ini" | "txt") {
        add_txt_file_to_map(entry, file);
        // } else if matches!(file_extension, "tga" | "pal" | "wav" | "lle" | "bmp" | "") {
        // add_raw_bytes_file_to_map(entry, file);
    }
}

pub fn add_txt_file_to_map_with_path_override(entry: &Path, file: &mut ZipFile, path: String) {
    let mut buffer = vec![0; file.size() as usize].into_boxed_slice();
    match file.read_exact(&mut buffer) {
        Ok(bytes_read) => bytes_read,
        Err(e) => {
            error!(
                "Error reading file: {} {} -> {}",
                entry.display(),
                file.name(),
                e
            );
            return;
        }
    };

    let intermediate_string = String::from_utf8_lossy(&buffer).to_string();

    let file_size = intermediate_string.len();
    let file_contents = CString::new(intermediate_string).unwrap();

    add_ztfile(
        entry,
        path.clone(),
        ZTFile::new_text(path, file_size as u32, file_contents).unwrap(),
    );
}

pub fn add_txt_file_to_map(entry: &Path, file: &mut ZipFile) {
    let file_name = file.name().to_string().to_lowercase();

    add_txt_file_to_map_with_path_override(entry, file, file_name)
}

pub fn add_raw_bytes_to_map_with_path_override(entry: &Path, file: &mut ZipFile, path: String) {
    let mut buffer = vec![0; file.size() as usize].into_boxed_slice();
    match file.read_exact(&mut buffer) {
        Ok(_) => {}
        Err(e) => {
            error!(
                "Error reading file: {} {} -> {}",
                entry.display(),
                file.name(),
                e
            );
            return;
        }
    };

    let file_size = file.size() as u32;
    add_ztfile(
        entry,
        path.clone(),
        ZTFile::new_raw_bytes(path, file_size, buffer),
    );
}

pub fn add_raw_bytes_file_to_map(entry: &Path, file: &mut ZipFile) {
    let file_name = file.name().to_string().to_lowercase();
    add_raw_bytes_to_map_with_path_override(entry, file, file_name)
}

static RESOURCE_STRING_TO_PTR_MAP: Lazy<Mutex<HashMap<String, u32>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

static RESOURCE_PTR_PTR_SET: Lazy<Mutex<HashSet<u32>>> = Lazy::new(|| Mutex::new(HashSet::new()));

pub fn add_ptr_ptr(ptr_ptr: u32) {
    RESOURCE_PTR_PTR_SET.lock().unwrap().insert(ptr_ptr);
}

pub fn check_ptr_ptr(ptr_ptr: u32) -> bool {
    RESOURCE_PTR_PTR_SET.lock().unwrap().contains(&ptr_ptr)
}

pub fn check_file(file_name: &str) -> bool {
    RESOURCE_STRING_TO_PTR_MAP
        .lock()
        .unwrap()
        .contains_key(&file_name.to_lowercase())
}

pub fn get_file_ptr(file_name: &str) -> Option<u32> {
    let binding = RESOURCE_STRING_TO_PTR_MAP.lock().unwrap();
    binding.get(&file_name.to_lowercase()).copied()
}

fn add_ztfile(path: &Path, file_name: String, ztfile: ZTFile) {
    let mut ztd_path = path.to_str().unwrap().to_string();
    ztd_path = ztd_path.replace("./", "zip::./").replace('\\', "/");
    let lowercase_filename = file_name.to_lowercase();
    match ztfile {
        ZTFile::Text(data, _, length) => {
            let ptr = data.into_raw() as u32;
            let resource_ptr = Box::into_raw(Box::new(BFResourcePtr {
                num_refs: 100, // We set this very high to prevent the game from unloading the resource
                bf_zip_name_ptr: CString::new(ztd_path).unwrap().into_raw() as u32,
                bf_resource_name_ptr: CString::new(lowercase_filename.clone()).unwrap().into_raw()
                    as u32,
                data_ptr: ptr,
                content_size: length,
            }));

            RESOURCE_STRING_TO_PTR_MAP
                .lock()
                .unwrap()
                .insert(file_name.clone(), resource_ptr as u32);
        }
        ZTFile::Graphics(data, _, length) => {
            let ptr = data.as_ptr() as u32;
            std::mem::forget(data);
            let resource_ptr = Box::into_raw(Box::new(BFResourcePtr {
                num_refs: 100, // We set this very high to prevent the game from unloading the resource
                bf_zip_name_ptr: CString::new(ztd_path).unwrap().into_raw() as u32,
                bf_resource_name_ptr: CString::new(lowercase_filename.clone()).unwrap().into_raw()
                    as u32,
                data_ptr: ptr,
                content_size: length,
            }));

            RESOURCE_STRING_TO_PTR_MAP
                .lock()
                .unwrap()
                .insert(lowercase_filename.clone(), resource_ptr as u32);
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
        let c_string_as_string = c_string.to_str().unwrap().to_string();
        let mut cfg = Ini::new_cs();
        cfg.set_comment_symbols(&[';', '#', ':']);
        if let Err(err) = cfg.read(c_string_as_string.clone()) {
            error!("Error reading ini: {}", err);
            return;
        };

        modifier(&mut cfg);

        let mut write_options = WriteOptions::default();
        write_options.space_around_delimiters = true;
        write_options.blank_lines_between_sections = 1;
        let new_string = cfg.pretty_writes(&write_options);
        file.content_size = new_string.len() as u32;

        let new_c_string = CString::new(new_string).unwrap();
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
        write!(f, "BFResourcePtr {{ num_refs: {:#x}, bf_zip_name: {}, bf_resource_name: {}, data_ptr: {:#x}, content_size: {:#x} }}", self.num_refs, get_string_from_memory(self.bf_zip_name_ptr), get_string_from_memory(self.bf_resource_name_ptr), self.data_ptr, self.content_size)
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
    let mut current_bf_resource_dir =
        get_from_memory::<BFResourceDir>(get_from_memory::<u32>(bf_resource_dir_ptr));
    bf_resource_dir_ptr += 4;

    while bf_resource_dir_ptr < bf_resource_mgr.resource_array_end {
        let class = get_from_memory::<u32>(get_from_memory::<u32>(bf_resource_dir_ptr));
        match class {
            0x630aec => {
                bf_resource_dir_contents.push(BFResourceDirContents {
                    dir: current_bf_resource_dir,
                    zips: bf_resource_zips,
                });
                current_bf_resource_dir =
                    get_from_memory::<BFResourceDir>(get_from_memory::<u32>(bf_resource_dir_ptr));
                bf_resource_zips = Vec::new();
                bf_resource_dir_ptr += 4;
            }
            0x630b0c => {
                bf_resource_zips.push(get_from_memory::<BFResourceZip>(get_from_memory::<u32>(
                    bf_resource_dir_ptr,
                )));
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
        write!(f, "BFResourceMgr {{ resource_array_start: 0x{:X}, resource_array_end: 0x{:X}, resource_array_buffer_end: 0x{:X}, unknown_u32_1: 0x{:X}, unknown_u32_2: 0x{:X}, unknown_u8_1: 0x{:X} }}", self.resource_array_start, self.resource_array_end, self.resource_array_buffer_end, self.unknown_u32_1, self.unknown_u32_2, self.unknown_u8_1)
    }
}

impl fmt::Display for BFResourceDir {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let dir_name_string = get_string_from_memory(self.dir_name_string_start);
        write!(f, "BFResourceDir {{ class: 0x{:X}, unknown_u32_1: 0x{:X}, dir_name: {}, num_bfr_zip: 0x{:X}, unknown_u32_2: 0x{:X} }}", self.class, self.unknown_u32_1, dir_name_string, self.num_child_files, self.unknown_u32_2)
    }
}

impl fmt::Display for BFResourceZip {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let zip_name_string = get_string_from_memory(self.zip_name_string_start);
        write!(f, "BFResourceZip {{ class: 0x{:X}, unknown_u32_1: 0x{:X}, unknown_u32_2: 0x{:X}, unknown_u32_3: 0x{:X}, zip_name: {}, contents_tree: 0x{:X} }}", self.class, self.unknown_u32_1, self.unknown_u32_2, self.unknown_u32_3, zip_name_string, self.contents_tree)
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
            result_string.push_str(&format!(
                "{}\n",
                get_string_from_memory(bf_resource_zip.zip_name_string_start)
            ));
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
    unsafe { zoo_resource_mgr::init_detours().unwrap() };
    add_handler(Handler::new(None, None, add_file_to_maps).unwrap());
}

#[hook_module("zoo.exe")]
pub mod zoo_resource_mgr {
    use bf_configparser::ini::Ini; //TODO: Replace with custom ini parser
    use tracing::info;

    use super::{check_file, get_file_ptr, load_resources, BFResourcePtr};
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
        if bf_resource_inner(this_ptr, file_name) {
            return 1;
        }

        unsafe { BFResource_prepare.call(this_ptr, file_name) }
    }

    fn bf_resource_inner(this_ptr: u32, file_name: u32) -> bool {
        let file_name_string = get_string_from_memory(file_name).to_lowercase();
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

    #[hook(unsafe extern "thiscall" BFResourceMgr_constructor, offset = 0x0012903f)]
    fn zoo_bf_resource_mgr_constructor(this_ptr: u32) -> u32 {
        info!("BFResourceMgr::constructor({:X})", this_ptr);
        let return_value = unsafe { BFResourceMgr_constructor.call(this_ptr) };
        let ini_path = get_ini_path();
        let mut zoo_ini = Ini::new();
        zoo_ini.set_comment_symbols(&['#']);
        zoo_ini.load(ini_path).unwrap();
        if let Some(paths) = zoo_ini.get("resource", "path") {
            // TODO: Readd this when more expansions can be added, expand to add subdirs of mods to ZT path variable
            // let path_vec = paths.split(';').map(|s| s.to_owned()).collect::<Vec<String>>();
            // if !path_vec.clone().into_iter().any(|s| s.trim() == "./mods") {
            //     info!("Adding mods directory to BFResourceMgr");
            //     let add_path: extern "thiscall" fn(u32, u32) -> u32 = unsafe { std::mem::transmute(0x0052870b) };
            //     if let Ok(mods_path) = CString::new("./mods") {
            //         add_path(this_ptr, mods_path.as_ptr() as u32);
            //     }
            // }
            info!("Loading resources from: {}", paths);
            load_resources(paths.split(';').map(|s| s.to_owned()).collect());
            info!("Resources loaded");
        }
        return_value
    }
}

#[derive(Clone)]
pub struct Handler {
    matcher_prefix: Option<String>,
    matcher_suffix: Option<String>,
    handler: HandlerFunction,
}

pub type HandlerFunction = fn(&Path, &mut ZipFile) -> ();

impl Handler {
    pub fn new(
        matcher_prefix: Option<String>,
        matcher_suffix: Option<String>,
        handler: HandlerFunction,
    ) -> Result<Self, &'static str> {
        Ok(Self {
            matcher_prefix,
            matcher_suffix,
            handler,
        })
    }

    fn handle(&self, entry: &Path, file: &mut ZipFile) {
        let file_name = file.name();
        if let Some(prefix) = &self.matcher_prefix {
            if !file_name.starts_with(prefix) {
                return;
            }
        }
        if let Some(file_type) = &self.matcher_suffix {
            if !file_name.ends_with(file_type) {
                return;
            }
        }
        (self.handler)(entry, file);
    }
}

fn get_ztd_resources(dir: &Path, recursive: bool) -> Vec<PathBuf> {
    let mut resources = Vec::new();
    if !dir.is_dir() {
        return resources;
    }
    let walker = WalkDir::new(dir)
        .follow_links(true)
        .max_depth(if recursive { 0 } else { 1 });
    for entry in walker {
        let entry = entry.unwrap();
        if entry
            .file_name()
            .to_str()
            .map(|s| s.to_lowercase().ends_with(".ztd") && !s.starts_with("ztat")) // Exlcuding ztatb*.ztd files until relevant
            .unwrap_or(false)
        {
            resources.push(entry.path().to_path_buf());
        }
    }
    resources
}

fn load_resources(paths: Vec<String>) {
    paths.iter().for_each(|path| {
        let resources = get_ztd_resources(Path::new(path), false);
        resources.iter().for_each(|resource| {
            handle_ztd(resource);
        });
    });
}

fn handle_ztd(resource: &PathBuf) {
    let mut buf_reader = BufReader::new(File::open(resource).unwrap());
    let mut zip = zip::ZipArchive::new(&mut buf_reader).unwrap();
    for i in 0..zip.len() {
        let data_mutex = RESOURCE_HANDLER_ARRAY.lock().unwrap();
        for handler in data_mutex.iter() {
            // ZipFile doesn't provide a .seek() method to set the cursor to the start of the file, so we create new ZipFile for each handler
            let mut file = zip.by_index(i).unwrap();
            if file.is_dir() {
                continue;
            }
            handler.handle(resource, &mut file);
        }
    }
}

static RESOURCE_HANDLER_ARRAY: Lazy<Mutex<Vec<Handler>>> = Lazy::new(|| Mutex::new(Vec::new()));

pub fn add_handler(handler: Handler) {
    let mut data_mutex = RESOURCE_HANDLER_ARRAY.lock().unwrap();
    data_mutex.push(handler);
}

fn get_handlers() -> Vec<Handler> {
    RESOURCE_HANDLER_ARRAY.lock().unwrap().clone()
}
