use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IVec3 {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BFTile {
    pub pos: IVec3,
    pub unknown_byte_2: u8,
}

#[tarpc::service]
pub trait OpenZTRpc {
    async fn show_int(num: i32);
    async fn hello_world(name: String) -> String;
    async fn allocate_bftile(tile: BFTile) -> u32;
    async fn deallocate_bftile(ptr: u32);
    async fn allocate_ivec3(ivec3: IVec3) -> u32;
    async fn deallocate_ivec3(ptr: u32);
    async fn show_ivec3(ptr: u32) -> u32;
    async fn get_local_elevation(tile: u32, ivec3: u32) -> i32;
    async fn test_test_test_test(ivec3: u32) -> i32;
}