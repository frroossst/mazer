// Cargo.toml dependencies you'll need:
// [dependencies]
// serde = { version = "1.0", features = ["derive"] }
// bincode = "1.3"
// nix = "0.27"
// eframe = "0.24" # For the GUI process
// egui = "0.24"

use std::sync::{Arc, Mutex, OnceLock};
use std::os::unix::net::UnixStream;
use std::io::Write;
use nix::sys::socket::{socketpair, AddressFamily, SockFlag, SockType};
use serde::{Serialize, Deserialize};
use nix::unistd::{fork, ForkResult};



// Message format for IPC communication
#[derive(Serialize, Deserialize, Debug)]
pub struct DebugMessage {
    pub timestamp: std::time::SystemTime,
    pub thread_id: String,
    pub location: String, // file:line
    pub variables: Vec<VariableInfo>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VariableInfo {
    pub name: String,
    pub value: String,
    pub type_name: String,
}

// Global state for the library
static DEBUG_STATE: OnceLock<Arc<Mutex<DebugState>>> = OnceLock::new();

struct DebugState {
    socket: Option<UnixStream>,
    gui_pid: Option<i32>,
}

pub fn init() -> Result<(), Box<dyn std::error::Error>> {
    
    // Create a socket pair for IPC
    let (parent_sock, child_sock) = socketpair(
        AddressFamily::Unix,
        SockType::Stream,
        None,
        SockFlag::empty()
    )?;
    
    match unsafe { fork() }? {
        ForkResult::Parent { child } => {
            // Parent process - the user's application
            drop(child_sock); // Close child end
            
            let state = DebugState {
                socket: Some(UnixStream::from(parent_sock)),
                gui_pid: Some(child.as_raw()),
            };
            
            DEBUG_STATE.set(Arc::new(Mutex::new(state)))
                .map_err(|_| "Failed to initialize debug state")?;
            
            Ok(())
        }
        ForkResult::Child => {
            // Child process - will run the GUI
            drop(parent_sock); // Close parent end
            run_gui_process(UnixStream::from(child_sock));
            std::process::exit(0);
        }
    }
}

fn run_gui_process(socket: UnixStream) {
    use std::sync::mpsc;
    use std::thread;
    use std::io::Read;
    
    let (tx, rx) = mpsc::channel::<DebugMessage>();
    
    // Spawn a thread to read from the socket
    let mut socket = socket;
    thread::spawn(move || {
        let mut buffer = [0u8; 4096];
        loop {
            match socket.read(&mut buffer) {
                Ok(0) => break, // Connection closed
                Ok(n) => {
                    if let Ok(msg) = bincode::deserialize::<DebugMessage>(&buffer[..n]) {
                        let _ = tx.send(msg);
                    }
                }
                Err(_) => break,
            }
        }
    });
    
    // Run the GUI
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_title("Mazer Debug"),
        ..Default::default()
    };
    
    let _ = eframe::run_native(
        "Mazer Debug",
        options,
        Box::new(|_cc| Box::new(DebugApp::new(rx))),
    );
}

// Simple GUI application
struct DebugApp {
    messages: Vec<DebugMessage>,
    receiver: std::sync::mpsc::Receiver<DebugMessage>,
}

impl DebugApp {
    fn new(receiver: std::sync::mpsc::Receiver<DebugMessage>) -> Self {
        Self {
            messages: Vec::new(),
            receiver,
        }
    }
}

impl eframe::App for DebugApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check for new messages
        while let Ok(msg) = self.receiver.try_recv() {
            self.messages.push(msg);
        }
        
        // Keep requesting repaints to check for new messages
        ctx.request_repaint();
        
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Debug Inspector");
            
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (i, msg) in self.messages.iter().enumerate() {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(format!("#{}", i + 1));
                            ui.label(format!("Thread: {}", msg.thread_id));
                            ui.label(format!("Location: {}", msg.location));
                        });
                        
                        for var in &msg.variables {
                            ui.horizontal(|ui| {
                                ui.label(&var.name);
                                ui.label(":");
                                ui.code(&var.value);
                                ui.label(format!("({})", var.type_name));
                            });
                        }
                    });
                    ui.separator();
                }
            });
        });
    }
}

// Function to send debug info to GUI process
pub fn send_debug_message(msg: DebugMessage) {
    if let Some(state) = DEBUG_STATE.get() {
        if let Ok(mut state) = state.lock() {
            if let Some(ref mut socket) = state.socket {
                if let Ok(data) = bincode::serialize(&msg) {
                    let _ = socket.write_all(&data);
                }
            }
        }
    }
}

// Helper function to get current thread ID as string
pub fn get_thread_id() -> String {
    format!("{:?}", std::thread::current().id())
}

// Macro implementation
#[macro_export]
macro_rules! inspect {
    ($($expr:expr),+ $(,)?) => {
        {
            let mut variables = Vec::new();
            $(
                variables.push($crate::VariableInfo {
                    name: stringify!($expr).to_string(),
                    value: format!("{:?}", $expr),
                    type_name: std::any::type_name_of_val(&$expr).to_string(),
                });
            )+
            
            let message = $crate::DebugMessage {
                timestamp: std::time::SystemTime::now(),
                thread_id: $crate::get_thread_id(),
                location: format!("{}:{}", file!(), line!()),
                variables,
            };
            
            $crate::send_debug_message(message);
        }
    };
}

// Cleanup function (optional, for graceful shutdown)
pub fn cleanup() {
    if let Some(state) = DEBUG_STATE.get() {
        if let Ok(mut state) = state.lock() {
            if let Some(pid) = state.gui_pid {
                // Send SIGTERM to GUI process
                let _ = nix::sys::signal::kill(
                    nix::unistd::Pid::from_raw(pid),
                    nix::sys::signal::Signal::SIGTERM
                );
            }
        }
    }
}

// Example usage:
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn example_usage() {
        // Initialize the debug system
        init().expect("Failed to initialize debug system");
        
        // Use the inspect macro
        let x = 42;
        let name = "Alice";
        let data = vec![1, 2, 3, 4, 5];
        
        inspect!(x, name, data);
        
        // From another thread
        tokio::spawn(async {
            let thread_data = "Hello from async";
            inspect!(thread_data);
        }).await.unwrap();
        
        // Keep the program running to see the GUI
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        
        cleanup();
    }
}