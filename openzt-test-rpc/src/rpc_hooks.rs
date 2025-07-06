use lrpc::*;
use tracing::info;

#[fmt_function]
pub fn hello_world(name: String) -> String{
    info!("Hello, {}!", name);
    return format!("Hello, {}!", name);
}

#[fmt_function]
pub fn show_int(num: i32) {
    info!("Received number: {}", num);
}