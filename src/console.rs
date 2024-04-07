extern crate winapi;

use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::Mutex,
    thread,
};

use once_cell::sync::Lazy; //TODO: Use std::sync::LazyCell when it becomes stable
use retour_utils::hook_module;
use tracing::info;

#[hook_module("zoo.exe")]
pub mod zoo_console {
    use super::{add_to_command_register, call_next_command, command_list_commands, start_server};

    #[hook(unsafe extern "thiscall" ZTApp_updateGame, offset = 0x0001a6d1)]
    fn zoo_zt_app_update_game(_this_ptr: u32, param_2: u32) {
        call_next_command();
        unsafe { ZTApp_updateGame.call(_this_ptr, param_2) }
    }

    pub fn init() {
        unsafe { init_detours().unwrap() };
        add_to_command_register("list_commands".to_owned(), command_list_commands);
        std::thread::spawn(|| {
            start_server();
        });
    }
}

type CommandCallback = fn(args: Vec<&str>) -> Result<String, &'static str>;

static COMMAND_REGISTRY: Lazy<Mutex<HashMap<String, CommandCallback>>> =
    Lazy::new(|| Mutex::new(HashMap::<String, CommandCallback>::new()));

static COMMAND_RESULTS: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(Vec::<String>::new()));

static COMMAND_QUEUE: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(Vec::<String>::new()));

pub fn add_to_command_register(command_name: String, command_callback: CommandCallback) {
    info!("Registring command {} to registry", command_name);
    let mut data_mutex = COMMAND_REGISTRY.lock().unwrap();
    data_mutex.insert(command_name, command_callback);
}

fn call_command(command_name: String, args: Vec<&str>) -> Result<String, &'static str> {
    info!("Calling command {} with args {:?}", command_name, args);
    let command = {
        let data_mutex = COMMAND_REGISTRY.lock().unwrap();
        data_mutex.get(&command_name).cloned()
    };
    match command {
        Some(command) => command(args),
        None => Err("Command not found"),
    }
}

pub fn call_next_command() {
    //} -> Result<String, &'static str> {
    let command = get_from_command_queue();
    if command.is_none() {
        return;
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
        }
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

pub fn command_list_commands(_args: Vec<&str>) -> Result<String, &'static str> {
    info!("Getting command list");
    match COMMAND_REGISTRY.lock() {
        Ok(data_mutex) => {
            let mut result = String::new();
            for command_name in data_mutex.keys() {
                info!("Found command {}", command_name);
                result.push_str(&format!("{}\n", command_name));
            }
            Ok(result)
        }
        Err(err) => {
            info!("Error getting command list: {}", err);
            Err("Error getting command list")
        }
    }
}

// fn call_command(command_name: String, args: Vec<&str>) -> Result<String, &'static str> {
//     info!("Calling command {} with args {:?}", command_name, args);
//     let data_mutex = COMMAND_REGISTRY.lock().unwrap();

//     let command = data_mutex.get(&command_name).cloned();
//     match command {
//         Some(command) => command(args),
//         None => Err("Command not found")
//     }
// }

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
