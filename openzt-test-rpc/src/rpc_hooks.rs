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
        }
    };
}
pub mod rpc_hooks {
    use lrpc::*;
 

    generate_allocate_deallocate_named!(openzt::ztmapview::BFTile, bftile);
    generate_allocate_deallocate_named!(openzt::ztworldmgr::IVec3, ivec3);

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
// pub fn allocate_bftile(tile: openzt::ztmapview::BFTile) -> u32 {
//     Box::into_raw(Box::new(tile)) as u32
// }

// #[fmt_function]
// pub fn deallocate_bftile(ptr: u32) {
//     unsafe {
//         let _ = Box::from_raw(ptr as *mut openzt::ztmapview::BFTile);
//     }
// }

