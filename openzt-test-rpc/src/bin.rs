use lrpc::*;

// use rpc_hooks::hello_world;
mod rpc_hooks;
use tracing::info;


pub fn main() {
    tracing_subscriber::fmt().init();
    let mut conn = Connection::new("127.0.0.1:9009");
    // info!("Result {}", conn.invoke::<String>(fun!("hello_world", "world".to_string())).unwrap());
    let result: Result<String> = conn.invoke::<String>(fun!("hello_world", "world".to_string()));
    if let Ok(res) = result {
        info!("Result: {}", res);
    } else {
        info!("Failed to invoke hello_world");
    }
    // let result: Result<()> = conn.invoke(fun!("show_int", 42));
}