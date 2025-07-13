#![allow(dead_code)]
#![feature(lazy_cell)]

mod rpc_hooks;

use openzt_detour_macro::detour_mod;
use tracing::{error, info};

#[cfg(target_os = "windows")]
use windows::Win32::System::{SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH, DLL_THREAD_ATTACH, DLL_THREAD_DETACH}, Console::{AllocConsole, FreeConsole}};

#[no_mangle]
extern "system" fn DllMain(_module: u8, reason: u32, _reserved: u8) -> i32 {
    match reason {
        DLL_PROCESS_ATTACH => {
            match init_console() {
                Ok(_) => {
                    let enable_ansi = enable_ansi_support::enable_ansi_support().is_ok();
                    tracing_subscriber::fmt().with_ansi(enable_ansi).init();
                },
                Err(e) => {
                    info!("Failed to initialize console: {}", e);
                    return 0; // Return 0 to indicate failure
                }
            }

            unsafe { detour_zoo_main::init_detours() }.is_err().then(|| {
                error!("Error initialising zoo_main detours");
            });
        }
        DLL_PROCESS_DETACH => {}
        DLL_THREAD_ATTACH => {}
        DLL_THREAD_DETACH => {}
        _ => {}
    }
    1
}


fn init_console() -> windows::core::Result<()> {
        // Free the current console
        unsafe { FreeConsole()? };

        // Allocate a new console
        unsafe { AllocConsole()? };

        Ok(())
}

#[detour_mod]
mod detour_zoo_main {
    use tracing::{error, info};
    use openzt_detour::LOAD_LANG_DLLS;
    use super::init_srv;

    // TODO: Fix this so it works with a crate/mod prefix
    #[detour(LOAD_LANG_DLLS)]
    unsafe extern "thiscall" fn detour_target(this: u32) -> u32 {
        info!("Detour success");

        let _result = unsafe { LOAD_LANG_DLLS_DETOUR.call(this) };

        // Get port from environment variable, default to 9009
        let port = std::env::var("OPENZT_RPC_PORT").unwrap_or_else(|_| "9009".to_string());
        let addr = format!("0.0.0.0:{}", port);
        
        info!("Starting RPC server on {}", addr);
        
        // Try to start the RPC server
        // Note: lrpc::service blocks forever if successful, so we need to handle this differently
        // We'll use a channel to communicate startup status
        let (tx, rx) = std::sync::mpsc::channel();
        let addr_clone = addr.clone();
        
        // Attempt to bind to the port first
        match std::net::TcpListener::bind(&addr_clone) {
            Ok(listener) => {
                // Successfully bound, close the test listener
                drop(listener);
                tx.send(Ok(())).unwrap();
                
                // Now start the actual RPC server (this will block forever)
                lrpc::service(init_srv(), &addr_clone);
            }
            Err(e) => {
                tx.send(Err(e)).unwrap();
            }
        }
        
        // Wait for the startup result
        match rx.recv_timeout(std::time::Duration::from_secs(5)) {
            Ok(Ok(())) => {
                info!("RPC server successfully started on {}", addr);
                // The server is running in the background thread, we can continue
                // Note: The server thread will keep running even after this function returns
            }
            Ok(Err(e)) => {
                error!("Failed to start RPC server on {}: {}", addr, e);
                error!("The port may already be in use or there may be a network configuration issue.");
                error!("You can specify a different port using the OPENZT_RPC_PORT environment variable.");
                error!("Exiting in 30 seconds...");
                
                // Wait 30 seconds before exiting
                std::thread::sleep(std::time::Duration::from_secs(30));
                std::process::exit(1);
            }
            Err(_) => {
                error!("RPC server startup timed out after 5 seconds");
                error!("Exiting in 30 seconds...");
                
                // Wait 30 seconds before exiting
                std::thread::sleep(std::time::Duration::from_secs(30));
                std::process::exit(1);
            }
        }
        
        // Continue with normal execution - don't block on the server thread
        // The RPC server is running in the background
        
        // Return the original result from the detoured function
        _result
    }
}

use crate::rpc_hooks::{show_int, hello_world, show_ivec3, rpc_hooks::{allocate_bftile, deallocate_bftile, allocate_ivec3, deallocate_ivec3}};

fn init_srv() -> lrpc::Fun {
    let mut srv_fun = lrpc::Fun::new();

    srv_fun.regist("show_int", show_int);
    srv_fun.regist("hello_world", hello_world);
    srv_fun.regist("allocate_bftile", allocate_bftile);
    srv_fun.regist("deallocate_bftile", deallocate_bftile);
    srv_fun.regist("allocate_ivec3", allocate_ivec3);
    srv_fun.regist("deallocate_ivec3", deallocate_ivec3);
    srv_fun.regist("show_ivec3", show_ivec3);

    srv_fun
}