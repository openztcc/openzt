use std::net::TcpListener;

use clap::Parser;
use tracing::{error, info};
use tracing_subscriber::filter::LevelFilter;

use {
    dll_syringe::process::{OwnedProcess, Process},
    dll_syringe::Syringe,
    std::os::windows::process::CommandExt,
    std::process::Command,
    winapi::shared::minwindef::DWORD,
    winapi::um::handleapi::CloseHandle,
    winapi::um::processthreadsapi::{OpenThread, ResumeThread},
    winapi::um::tlhelp32::{CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, Thread32First, Thread32Next, THREADENTRY32},
    winapi::um::winbase::{CREATE_SUSPENDED, DETACHED_PROCESS},
    winapi::um::winnt::{THREAD_QUERY_INFORMATION, THREAD_SUSPEND_RESUME},
};

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, default_value = "false")]
    resume: bool,
    #[arg(short, long, default_value = "false")]
    listen: bool,
    #[arg(short, long, default_value = "target/i686-pc-windows-msvc/release/deps/openzt.dll")]
    dll_path: String,
}

fn main() {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(LevelFilter::DEBUG)
        .finish();

    tracing::subscriber::set_global_default(subscriber).unwrap_or_else(|error| {
        panic!("Failed to init tracing: {error}")
    });

    let args = Args::parse();

    info!("Starting OpenZT Loader with args: {:?}", args);

    const CREATE_FLAGS: u32 = CREATE_SUSPENDED | DETACHED_PROCESS;
    const ZOO_PATH: &str = "C:\\Program Files (x86)\\Microsoft Games\\Zoo Tycoon\\zoo.exe";
    let command: OwnedProcess = match Command::new(ZOO_PATH).creation_flags(CREATE_FLAGS).spawn() {
        Ok(command) => command.into(),
        Err(e) => panic!("Failed to spawn process: {e}"),
    };

    info!("Process spawned");

    let listener = TcpListener::bind("127.0.0.1:1492");

    if listener.is_err() {
        error!("Failed to bind to port 1492: Log stream disabled");
    }

    let syringe = Syringe::for_process(command);
    match syringe.inject(args.dll_path) {
        Ok(_) => (),
        Err(e) => panic!("Failed to inject dll: {e}")
    }

    info!("Dll Injected");

    if args.resume {
        // let Ok(process_pid) = syringe.process().pid() else {

        // }
        match syringe.process().pid() {
            Ok(pid) => resume_threads(pid.into()),
            Err(err) => error!("Failed to get process pid: {}", err)
        }
        info!("Thread Resumed");
    }

    if args.listen && listener.is_ok() {
        let mut stream = match listener.unwrap().accept() {
            Ok((stream, addr)) => {
                info!(%addr, "Accepted connection from");
                stream
            }
            Err(error) => panic!("Log stream failed to connect: {error}")
        };
        match std::io::copy(&mut stream, &mut std::io::stdout()) {
            Ok(_) => (),
            Err(e) => info!("Logging Stream Closed: {e}")
        };
    }
}


fn resume_threads(process_id: u32) {
    // Take a snapshot of the threads in the system
    let snap_handle = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
    if snap_handle.is_null() {
        panic!("Failed to create snapshot: {}", std::io::Error::last_os_error());
    }

    // Enumerate the threads of the process using Thread32First and Thread32Next
    let mut thread_entry: THREADENTRY32 = unsafe { std::mem::zeroed() };
    thread_entry.dwSize = std::mem::size_of::<THREADENTRY32>() as u32;
    let result = unsafe { Thread32First(snap_handle, &mut thread_entry) };
    while result != 0 {
        if thread_entry.th32OwnerProcessID == process_id {
            // Open the thread with THREAD_QUERY_INFORMATION and THREAD_SUSPEND_RESUME permissions
            let thread_handle = unsafe {
                OpenThread(
                    THREAD_QUERY_INFORMATION | THREAD_SUSPEND_RESUME,
                    winapi::shared::minwindef::FALSE,
                    thread_entry.th32ThreadID,
                )
            };
            if thread_handle.is_null() {
                error!("Failed to open thread: {}", std::io::Error::last_os_error());
                continue;
            }

            // Resume the thread with ResumeThread
            let result = unsafe { ResumeThread(thread_handle) };
            if result == DWORD::max_value() {
                error!("Failed to resume thread: {}", std::io::Error::last_os_error());
            } else {
                info!("Resumed thread: {}", thread_entry.th32ThreadID);
            }

            // Close the thread handle
            let result = unsafe { CloseHandle(thread_handle) };
            if result == 0 {
                error!("Failed to close thread handle: {}", std::io::Error::last_os_error());
            }
        }

        // Get the next thread in the snapshot
        let result = unsafe { Thread32Next(snap_handle, &mut thread_entry) };
        if result == 0 {
            break;
        }
    }

    // Close the snapshot handle
    let result = unsafe { CloseHandle(snap_handle) };
    if result == 0 {
        panic!("Failed to close snapshot handle: {}", std::io::Error::last_os_error());
    }
}
