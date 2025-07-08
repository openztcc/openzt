use winresource::WindowsResource;

fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    
    if target_os == "windows" {
        WindowsResource::new()
            .compile()
            .unwrap();
    }
}
