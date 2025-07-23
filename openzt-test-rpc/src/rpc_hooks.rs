use lrpc::*;
use tracing::info;

macro_rules! generate_allocate_deallocate_named {
    ($type:ty, $name:ident) => {
        paste::paste! {
            #[fmt_function]
            pub fn [<allocate_ $name>](item: $type) -> u32 {
                Box::into_raw(Box::new(item)) as u32
            }

            #[fmt_function]
            pub fn [<deallocate_ $name>](ptr: u32) {
                unsafe {
                    let _ = Box::from_raw(ptr as *mut $type);
                }
            }

            #[fmt_function]
            pub fn [<show_ $name>](ptr: u32) -> u32 {
                unsafe {
                    let item = Box::from_raw(ptr as *mut $type);
                    info!("Received {}: {:?}", stringify!($name), item);
                    [<show_ $name>](item)
                }
            }
        }
    };
}
pub mod rpc_hooks {
    use lrpc::*;
 

    generate_allocate_deallocate_named!(openztlib::ztmapview::BFTile, bftile);
    generate_allocate_deallocate_named!(openztlib::ztworldmgr::IVec3, ivec3);

}

#[fmt_function]
pub fn hello_world(name: String) -> String{
    info!("Hello, {}!", name);
    return format!("Hello, {}!", name);
}

#[fmt_function]
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

