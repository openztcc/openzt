extern crate winapi;

use std::collections::HashMap;
// use std::error::Error;
// use std::io::{self, Read, Write};
// use std::str;

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

use std::sync::Mutex;
use std::net::SocketAddr;
use once_cell::sync::Lazy;
use tracing::info;

type command_callback = fn(args: Vec<&str>) -> Result<String, &'static str>;

static COMMAND_REGISTRY: Lazy<Mutex<HashMap<String, command_callback>>> = Lazy::new(|| {
    Mutex::new(HashMap::<String, command_callback>::new())
});

static COMMAND_RESULTS: Lazy<Mutex<Vec<String>>> = Lazy::new(|| {
    Mutex::new(Vec::<String>::new())
});

static COMMAND_QUEUE: Lazy<Mutex<Vec::<String>>> = Lazy::new(|| {
    Mutex::new(Vec::<String>::new())
});

pub fn add_to_command_register(command_name: String, command_callback: command_callback) {
    info!("Registring command {} to registry", command_name);
    let mut data_mutex = COMMAND_REGISTRY.lock().unwrap();
    data_mutex.insert(command_name, command_callback);
}

pub fn call_command(command_name: String, args: Vec<&str>) -> Result<String, &'static str> {
    info!("Calling command {} with args {:?}", command_name, args);
    let data_mutex = COMMAND_REGISTRY.lock().unwrap();
    
    let command = data_mutex.get(&command_name).cloned();
    match command {
        Some(command) => command(args),
        None => Err("Command not found")
    }
    // data_mutex.get(&command_name).cloned().unwrap()(args)
}

pub fn call_next_command() { //} -> Result<String, &'static str> {
    let command = get_from_command_queue();
    if command.is_none() {
         return
    } else {
        info!("Calling next command {}", command.as_ref().unwrap());
    }

    let command = command.unwrap();
    let mut command = command.split_whitespace();
    let command_name = command.next().unwrap().to_string();
    let args: Vec<&str> = command.collect();

    match call_command(command_name, args) {
        Ok(result) => {
            let mut data_mutex = COMMAND_RESULTS.lock().unwrap();
            data_mutex.push(result);
        },
        Err(err) => {
            let result = err.to_string();
            let mut data_mutex = COMMAND_RESULTS.lock().unwrap();
            data_mutex.push(result);
        }
    }
}

pub fn get_next_result() -> Option<String> {
    let mut data_mutex = COMMAND_RESULTS.lock().unwrap();
    data_mutex.pop()
}

fn add_to_command_queue(command: String) {
    info!("Adding command {} to queue", command);
    let mut data_mutex = COMMAND_QUEUE.lock().unwrap();
    data_mutex.push(command);
}

pub fn get_from_command_queue() -> Option<String> {
    let mut data_mutex = COMMAND_QUEUE.lock().unwrap();
    data_mutex.pop()
}

fn handle_client(mut stream: TcpStream) {
    let mut buffer = [0; 1024]; // Buffer to store received data

    loop {
        match stream.read(&mut buffer) {
            Ok(size) => {
                if size == 0 {
                    // Connection closed by client
                    break;
                }

                // Print received string
                let received_string = String::from_utf8_lossy(&buffer[0..size]);
                add_to_command_queue(received_string.to_string());
                info!("Received: {}", received_string);

                loop {
                    let result = get_next_result();
                    if !result.is_none() {
                        let result = result.unwrap();
                        // info!("Sending: {}", result);
                        if let Err(err) = stream.write_all(result.as_bytes()) {
                            info!("Error sending data: {}", err);
                        }
                        break;
                    }
                }

                // Send the received string back to the client
                // if let Err(err) = stream.write_all(&buffer[0..size]) {
                //     info!("Error sending data: {}", err);
                //     break;
                // }
            }
            Err(err) => {
                info!("Error reading data: {}", err);
                break;
            }
        }
    }
}

pub fn start_server() {
    let listener = TcpListener::bind("127.0.0.1:8080").expect("Failed to bind socket");

    info!("Listening on 127.0.0.1:8080...");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                // Spawn a new thread for each incoming connection
                thread::spawn(move || {
                    handle_client(stream);
                });
            }
            Err(err) => {
                info!("Error accepting connection: {}", err);
            }
        }
    }
}