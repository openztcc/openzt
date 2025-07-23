#[cfg(test)]
mod tests {
    use lrpc::*;

    static GRPC_CONNECTION: LazyLock<Mutex<Connection>> = LazyLock::new(|| Mutex::new(
        {
            let port = std::env::var("OPENZT_RPC_PORT").unwrap_or_else(|_| "9009".to_string());
            let addr = format!("127.0.0.1:{}", port);
    
            info!("Connecting to RPC server at {}", addr);
            Connection::new(&addr)
        }
    ));
   
    macro_rules! rpc_test {
        ($name:ident, $fun:expr) => {
            #[test]
            fn $name() {
                let mut conn = GRPC_CONNECTION.lock().unwrap();
                if let Err(error) = panic::catch_unwind(|| $body) {
                    drop(guard);
                    panic::resume_unwind(err);
                }
            }
        };
    }

}