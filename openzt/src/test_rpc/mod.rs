#![allow(dead_code)]

mod rpc_hooks;
pub mod service;

use futures::prelude::*;
use tarpc::{context, server::{self, Channel}};
use std::net::SocketAddr;

#[cfg(target_os = "windows")]
use crate::detour_mod;
use tracing::{error, info};

#[cfg(target_os = "windows")]
use windows::Win32::System::{Console::{AllocConsole, FreeConsole}};

pub fn init() {
    #[cfg(target_os = "windows")]
    {
        match init_console() {
            Ok(_) => {
                let enable_ansi = enable_ansi_support::enable_ansi_support().is_ok();
                tracing_subscriber::fmt().with_ansi(enable_ansi).init();
            },
            Err(e) => {
                info!("Failed to initialize console: {}", e);
            }
        }

        unsafe { detour_zoo_main::init_detours() }.is_err().then(|| {
            error!("Error initialising zoo_main detours");
        });
    }
}


#[cfg(target_os = "windows")]
fn init_console() -> windows::core::Result<()> {
        // Free the current console
        unsafe { FreeConsole()? };

        // Allocate a new console
        unsafe { AllocConsole()? };

        Ok(())
}

#[cfg(target_os = "windows")]
#[detour_mod]
mod detour_zoo_main {
    use tracing::{error, info};
    #[cfg(target_os = "windows")]
    use openzt_detour::LOAD_LANG_DLLS;
    use std::net::SocketAddr;
    use super::spawn_server;

    // TODO: Fix this so it works with a crate/mod prefix
    #[detour(LOAD_LANG_DLLS)]
    unsafe extern "thiscall" fn detour_target(this: u32) -> u32 {
        info!("Detour success");

        let _result = unsafe { LOAD_LANG_DLLS_DETOUR.call(this) };

        // Get port from environment variable, default to 9009
        let port = std::env::var("OPENZT_RPC_PORT").unwrap_or_else(|_| "9009".to_string());
        let addr = format!("0.0.0.0:{}", port);
        
        info!("Starting RPC server on {}", addr);
        
        let addr_clone = addr.clone();
        
                
            // Now start the actual RPC server in a background thread
        let socket_addr: SocketAddr = addr_clone.parse().expect("Invalid address");
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            if let Err(e) = spawn_server(socket_addr).await {
                error!("RPC server error: {}", e);
            }
        });
        
        _result
    }
}

use crate::test_rpc::rpc_hooks::{show_int, hello_world, rpc_hooks::{allocate_bftile, deallocate_bftile, allocate_ivec3, deallocate_ivec3, show_ivec3, bftile_get_local_elevation, bftile_get_local_elevation_2}};
use crate::test_rpc::service::{OpenZTRpc, IVec3, BFTile};

#[derive(Clone)]
struct OpenZTRpcImpl;

impl OpenZTRpc for OpenZTRpcImpl {
    async fn show_int(self, _: context::Context, num: i32) {
        show_int(num);
    }

    async fn hello_world(self, _: context::Context, name: String) -> String {
        hello_world(name)
    }

    async fn allocate_bftile(self, _: context::Context, tile: BFTile) -> u32 {
        // Convert from service::BFTile to ztmapview::BFTile
        let pos = crate::ztworldmgr::IVec3 { x: tile.pos.x, y: tile.pos.y, z: tile.pos.z };
        let tile = crate::ztmapview::BFTile::new(pos, tile.unknown_byte_2);
        allocate_bftile(tile)
    }

    async fn deallocate_bftile(self, _: context::Context, ptr: u32) {
        deallocate_bftile(ptr);
    }

    async fn allocate_ivec3(self, _: context::Context, ivec3: IVec3) -> u32 {
        // Convert from service::IVec3 to ztworldmgr::IVec3
        let ivec3 = crate::ztworldmgr::IVec3 { x: ivec3.x, y: ivec3.y, z: ivec3.z };
        allocate_ivec3(ivec3)
    }

    async fn deallocate_ivec3(self, _: context::Context, ptr: u32) {
        deallocate_ivec3(ptr);
    }

    async fn show_ivec3(self, _: context::Context, ptr: u32) -> u32 {
        show_ivec3(ptr)
    }

    async fn get_local_elevation(self, _: context::Context, tile: u32, ivec3: u32) -> i32 {
        bftile_get_local_elevation(tile, ivec3)
    }

    async fn test_test_test_test(self, _: context::Context, ivec3: u32) -> i32 {
        bftile_get_local_elevation_2(ivec3)
    }
}

async fn spawn_server(addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
    let mut listener = tarpc::serde_transport::tcp::listen(&addr, tarpc::tokio_serde::formats::Json::default).await?;
    listener.config_mut().max_frame_length(usize::MAX);
    
    loop {
        let Some(transport) = listener.next().await else {
            break;
        };
        let transport = transport?;
        let handler = OpenZTRpcImpl;
        tokio::spawn(
            server::BaseChannel::with_defaults(transport)
                .execute(handler.serve())
                .for_each(|resp| async move {
                    tokio::spawn(resp);
                })
        );
    }
    Ok(())
}