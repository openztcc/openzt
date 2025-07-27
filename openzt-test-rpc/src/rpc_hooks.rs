use tracing::info;

macro_rules! generate_allocate_deallocate_named {
    ($type:ty, $name:ident) => {
        paste::paste! {
            pub fn [<allocate_ $name>](item: $type) -> u32 {
                let ptr = Box::into_raw(Box::new(item)) as u32;
                info!("Allocated {} at {}", stringify!($name), ptr);
                ptr
            }

            pub fn [<deallocate_ $name>](ptr: u32) {
                unsafe {
                    let _ = Box::from_raw(ptr as *mut $type);
                }
            }

            // TODO: Leak rather than into_raw and store the reference in a HashMap
            pub fn [<show_ $name>](ptr: u32) -> u32 {
                let item = unsafe {Box::from_raw(ptr as *mut $type)};
                info!("Received {}: {:?}", stringify!($name), item);
                let ptr = Box::into_raw(Box::new(item)) as u32;
                info!("Allocated {} at {}", stringify!($name), ptr);
                ptr
            }
        }
    };
}

pub mod rpc_hooks {
    use tracing::info;
 

    generate_allocate_deallocate_named!(openztlib::ztmapview::BFTile, bftile);
    generate_allocate_deallocate_named!(openztlib::ztworldmgr::IVec3, ivec3);


    pub fn bftile_get_local_elevation(tile: u32, ivec3: u32) -> i32 {
        unsafe { openzt_detour::BFTILE_GET_LOCAL_ELEVATION.original()(tile, ivec3) }
    }

    pub fn bftile_get_local_elevation_2(ivec3: u32) -> i32 {
        info!("Getting local elevation for with IVec3 at {}", ivec3);
        // unsafe { openzt_detour::BFTILE_GET_LOCAL_ELEVATION.original()(tile, ivec3) }
        0
    }

    // pub fn show_string(s: String) {
    //     info!("Received string: {}", s);
    // }

    // #[fmt_function]
    // pub fn show_bool(b: bool) {
    //     info!("Received boolean: {}", b);
    // }

    // #[fmt_function]
    // pub fn show_float(f: f32) {
    //     info!("Received float: {}", f);
    // }    

}

pub fn hello_world(name: String) -> String{
    info!("Hello, {}!", name);
    return format!("Hello, {}!", name);
}

pub fn show_int(num: i32) {
    info!("Received number: {}", num);
}

// #[fmt_function]
// pub fn show_ivec3(vec_ptr: u32) {
//     unsafe {
//         let vec = Box::from_raw(vec_ptr as *mut openztlib::ztworldmgr::IVec3);
//         info!("Received IVec3: ({}, {}, {})", vec.x, vec.y, vec.z);
//     }
// }

// #[fmt_function]
// pub fn allocate_bftile(tile: openzt::ztmapview::BFTile) -> u32 {
//     Box::into_raw(Box::new(tile)) as u32
// }

// #[fmt_function]
// pub fn deallocate_bftile(ptr: u32) {
//     unsafe {
//         let _ = Box::from_raw(ptr as *mut openzt::ztmapview::BFTile);
//     }
// }

