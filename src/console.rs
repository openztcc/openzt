extern crate winapi;

use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::Mutex,
    thread,
};

use std::error::Error;
use std::fmt;

use once_cell::sync::Lazy; //TODO: Use std::sync::LazyCell when it becomes stable
use retour_utils::hook_module;
use tracing::{info, error};

#[derive(Debug)]
pub struct CommandError {
    message: String,
}

impl CommandError {
    pub fn new(message: String) -> Self {
        CommandError { message }
    }
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "CommandError: {}", self.message)
    }
}

impl Error for CommandError {}

impl From<std::str::ParseBoolError> for CommandError {
    fn from(err: std::str::ParseBoolError) -> Self {
        CommandError {
            message: format!("Failed to parse bool: {}", err),
        }
    }
}

impl From<std::num::ParseIntError> for CommandError {
    fn from(err: std::num::ParseIntError) -> Self {
        CommandError {
            message: format!("Failed to parse int: {}", err),
        }
    }
}

impl From<std::num::ParseFloatError> for CommandError {
    fn from(err: std::num::ParseFloatError) -> Self {
        CommandError {
            message: format!("Failed to parse float: {}", err),
        }
    }
}

impl From<String> for CommandError {
    fn from(err: String) -> Self {
        CommandError { message: err }
    }
}

impl From<&str> for CommandError {
    fn from(err: &str) -> Self {
        CommandError {
            message: err.to_string(),
        }
    }
}

#[hook_module("zoo.exe")]
pub mod zoo_console {
    use super::{add_to_command_register, call_next_command, command_list_commands, start_server};
    use tracing::error;

    #[hook(unsafe extern "thiscall" ZTApp_updateGame, offset = 0x0001a6d1)]
    fn zoo_zt_app_update_game(_this_ptr: u32, param_2: u32) {
        call_next_command();
        unsafe { ZTApp_updateGame.call(_this_ptr, param_2) }
    }

    pub fn init() {
        unsafe { 
            if !init_detours().is_ok() {
                error!("Failed to initialize console detours");
            }
        };
        add_to_command_register("list_commands".to_owned(), command_list_commands);
        std::thread::spawn(|| {
            start_server();
        });
    }
}

type CommandCallback = fn(args: Vec<&str>) -> Result<String, CommandError>;

static COMMAND_REGISTRY: Lazy<Mutex<HashMap<String, CommandCallback>>> =
    Lazy::new(|| Mutex::new(HashMap::<String, CommandCallback>::new()));

static COMMAND_RESULTS: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(Vec::<String>::new()));

static COMMAND_QUEUE: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(Vec::<String>::new()));

pub fn add_to_command_register(command_name: String, command_callback: CommandCallback) {
    info!("Registring command {} to registry", command_name);
    let Ok(mut data_mutex) = COMMAND_REGISTRY.lock() else {
        error!("Failed to lock command registry mutex when adding command {} to registry", command_name);
        return;

    };
    data_mutex.insert(command_name, command_callback);
}

fn call_command(command_name: String, args: Vec<&str>) -> Result<String, CommandError> {
    info!("Calling command {} with args {:?}", command_name, args);
    let command = {
        let Ok(data_mutex) = COMMAND_REGISTRY.lock() else {
            error!("Failed to lock command registry mutex when calling command {}", command_name);
            return Err(Into::into("Failed to lock command registry mutex when calling command"));
        };
        data_mutex.get(&command_name).cloned()
    };
    match command {
        Some(command) => command(args),
        None => Err(Into::into("Command not found")),
    }
}

pub fn call_next_command() {
    let Some(command) = get_from_command_queue() else {
        return;
    };

    info!("Calling next command {}", command.clone());

    let mut command_args = command.split_whitespace();
    let Some(command_name) = command_args.next() else {
        error!("Failed to get command name from command {}", command.clone());
        return;
    };
    let args: Vec<&str> = command_args.collect();

    let Ok(mut result_mutex) = COMMAND_RESULTS.lock() else {
        error!("Failed to lock command results mutex when calling next command {}", command_name);
        return;
    };

    match call_command(command_name.to_string(), args) {
        Ok(result) => {
            result_mutex.push(result);
        }
        Err(err) => {
            let result = err.to_string();
            result_mutex.push(result);
        }
    }
}

pub fn get_next_result() -> Option<String> {
    let Ok(mut data_mutex) = COMMAND_RESULTS.lock() else {
        error!("Failed to lock command results mutex when getting next result");
        return None;
    };
    data_mutex.pop()
}

fn add_to_command_queue(command: String) {
    info!("Adding command {} to queue", command);
    let Ok(mut data_mutex) = COMMAND_QUEUE.lock() else {
        error!("Failed to lock command queue mutex when adding command {} to queue", command);
        return;
    };
    data_mutex.push(command);
}

pub fn get_from_command_queue() -> Option<String> {
    let Ok(mut data_mutex) = COMMAND_QUEUE.lock() else {
        error!("Failed to lock command queue mutex when getting command from queue");
        return None;
    };
    data_mutex.pop()
}

pub fn command_list_commands(_args: Vec<&str>) -> Result<String, CommandError> {
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
            Err(Into::into("Error getting command list"))
        }
    }
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
                    if let Some(result) = get_next_result() {
                        if let Err(err) = stream.write_all(result.as_bytes()) {
                            info!("Error sending data: {}", err);
                        }
                        break;
                    }
                }
            }
            Err(err) => {
                info!("Error reading data: {}", err);
                break;
            }
        }
    }
}

pub fn start_server() {
    let Ok(listener) = TcpListener::bind("127.0.0.1:8080") else {
        error!("Failed to bind socket 127.0.0.1:8080, console will not work");
        return;
    };

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
