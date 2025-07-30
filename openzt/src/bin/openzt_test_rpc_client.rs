#![cfg(feature = "test-rpc")]

use tarpc::{client, context};
use tracing::info;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use openztlib::test_rpc::service::{OpenZTRpcClient, IVec3};


#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().init();
    
    // Get port from environment variable, default to 9009
    let port = std::env::var("OPENZT_RPC_PORT").unwrap_or_else(|_| "9009".to_string());
    let socket_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port.parse()?);
    
    info!("Connecting to RPC server at {}", socket_addr);
    
    let transport = tarpc::serde_transport::tcp::connect(socket_addr, tarpc::tokio_serde::formats::Json::default).await?;
    let client = OpenZTRpcClient::new(client::Config::default(), transport).spawn();
    
    let vec = IVec3 { x: 1, y: 2, z: 3 };
    let vec_ptr = client.allocate_ivec3(context::current(), vec).await?;
    info!("Allocated IVec3 at pointer: {}", vec_ptr);
    
    let vec_ptr2 = client.show_ivec3(context::current(), vec_ptr).await?;
    info!("Reallocated IVec3 at pointer: {}", vec_ptr2);
    
    client.show_ivec3(context::current(), vec_ptr2).await?;
    
    // Test hello_world
    let result = client.hello_world(context::current(), "world".to_string()).await?;
    info!("Result: {}", result);
    
    // Test show_int
    client.show_int(context::current(), 42).await?;
    
    Ok(())
}
