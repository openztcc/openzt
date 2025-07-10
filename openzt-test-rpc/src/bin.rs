use lrpc::*;

// use rpc_hooks::hello_world;
mod rpc_hooks;
use tracing::info;


pub fn main() {
    tracing_subscriber::fmt().init();
    
    // Get port from environment variable, default to 9009
    let port = std::env::var("OPENZT_RPC_PORT").unwrap_or_else(|_| "9009".to_string());
    let addr = format!("127.0.0.1:{}", port);
    
    info!("Connecting to RPC server at {}", addr);
    let mut conn = Connection::new(&addr);
    
    // info!("Result {}", conn.invoke::<String>(fun!("hello_world", "world".to_string())).unwrap());
    let result: Result<String> = conn.invoke::<String>(fun!("hello_world", "world".to_string()));
    if let Ok(res) = result {
        info!("Result: {}", res);
    } else {
        info!("Failed to invoke hello_world - make sure the RPC server is running on port {}", port);
    }
    // let result: Result<()> = conn.invoke(fun!("show_int", 42));
}
