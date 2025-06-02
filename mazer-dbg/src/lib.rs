use ipc_channel::ipc;
use nix::unistd::{fork, ForkResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::process;
use std::sync::{Arc, Mutex, OnceLock};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct DebugMessage {
    timestamp: u64,
    variables: BTreeMap<String, String>,
}

// Global sender for the inspect macro to use
static DEBUG_SENDER: OnceLock<Arc<Mutex<ipc::IpcSender<DebugMessage>>>> = OnceLock::new();

/// Initialize the debug IPC system
/// Returns true if initialization was successful
pub fn init() -> bool {
    // Create an IPC channel
    let (tx, rx) = match ipc::channel() {
        Ok(channel) => channel,
        Err(e) => {
            eprintln!("Failed to create IPC channel: {}", e);
            return false;
        }
    };

    // Fork the process
    match unsafe { fork() } {
        Ok(ForkResult::Parent { child }) => {
            // Parent process - store sender globally
            if DEBUG_SENDER.set(Arc::new(Mutex::new(tx))).is_err() {
                eprintln!("Failed to set debug sender");
                return false;
            }
            println!("Debug server initialized (child PID: {})", child);
            true
        }
        Ok(ForkResult::Child) => {
            // Child process - start debug server
            debug_server_process(rx);
            // This will never return as child exits
            unreachable!()
        }
        Err(e) => {
            eprintln!("Fork failed: {}", e);
            false
        }
    }
}

/// Send debug data to the server (called by inspect macro)
pub fn send_to_debug_server(variables: BTreeMap<String, String>) {
    if let Some(sender) = DEBUG_SENDER.get() {
        if let Ok(sender) = sender.lock() {
            let message = DebugMessage {
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
                variables,
            };

            if let Err(e) = sender.send(message) {
                eprintln!("Failed to send debug message: {}", e);
            }
        }
    }
}

/// The debug server process that receives and processes debug messages
fn debug_server_process(rx: ipc::IpcReceiver<DebugMessage>) {
    println!("Debug server started (PID: {})", process::id());
    
    loop {
        match rx.recv() {
            Ok(message) => {
                println!("\n=== DEBUG MESSAGE [{}] ===", message.timestamp);
                for (var_name, var_value) in &message.variables {
                    println!("{}: {}", var_name, var_value);
                }
                println!("================================\n");
            }
            Err(e) => {
                println!("Debug server: Channel error: {}", e);
                break;
            }
        }
    }
    
    println!("Debug server shutting down");
    process::exit(0);
}


#[macro_export]
macro_rules! inspect {
    ( $( $var:expr ),+ $(,)? ) => {{
        use std::collections::BTreeMap;

        let mut map = BTreeMap::new();
        $(
            map.insert(stringify!($var).to_string(), format!("{:#?}", $var));
        )+

        // Send to debug server if available
        $crate::send_to_debug_server(map.clone());
        
        map
    }};
}
