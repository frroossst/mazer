use ipc_channel::ipc;
use nix::unistd::{fork, ForkResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::process;
use std::sync::{Arc, Mutex, OnceLock};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct DebugMessage {
    timestamp: u64,
    file: String,
    line: u32,
    column: u32,
    variables: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct DebugResponse {
    continue_execution: bool,
}

// Global channels for bidirectional communication
static DEBUG_SENDER: OnceLock<Arc<Mutex<ipc::IpcSender<DebugMessage>>>> = OnceLock::new();
static RESPONSE_RECEIVER: OnceLock<Arc<Mutex<ipc::IpcReceiver<DebugResponse>>>> = OnceLock::new();

/// Initialize the debug IPC system
/// Returns true if initialization was successful
pub fn init() -> bool {
    // Create bidirectional IPC channels
    let (debug_tx, debug_rx) = match ipc::channel() {
        Ok(channel) => channel,
        Err(e) => {
            eprintln!("Failed to create debug IPC channel: {}", e);
            return false;
        }
    };
    
    let (response_tx, response_rx) = match ipc::channel() {
        Ok(channel) => channel,
        Err(e) => {
            eprintln!("Failed to create response IPC channel: {}", e);
            return false;
        }
    };

    // Fork the process
    match unsafe { fork() } {
        Ok(ForkResult::Parent { child }) => {
            // Parent process - store channels globally
            if DEBUG_SENDER.set(Arc::new(Mutex::new(debug_tx))).is_err() {
                eprintln!("Failed to set debug sender");
                return false;
            }
            
            if RESPONSE_RECEIVER.set(Arc::new(Mutex::new(response_rx))).is_err() {
                eprintln!("Failed to set response receiver");
                return false;
            }
            
            println!("Debug server initialized (child PID: {})", child);
            true
        }
        Ok(ForkResult::Child) => {
            // Child process - start debug server with GUI
            debug_server_process(debug_rx, response_tx);
            // This will never return as child exits
            unreachable!()
        }
        Err(e) => {
            eprintln!("Fork failed: {}", e);
            false
        }
    }
}

/// Send debug data to the server and wait for user to close GUI (called by inspect macro)
pub fn send_to_debug_server_and_wait(
    variables: BTreeMap<String, String>,
    file: &str,
    line: u32,
    column: u32,
) {
    if let (Some(sender), Some(receiver)) = (DEBUG_SENDER.get(), RESPONSE_RECEIVER.get()) {
        if let (Ok(sender), Ok(receiver)) = (sender.lock(), receiver.lock()) {
            let message = DebugMessage {
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
                file: file.to_string(),
                line,
                column,
                variables,
            };
            
            // Send debug message
            if let Err(e) = sender.send(message) {
                eprintln!("Failed to send debug message: {}", e);
                return;
            }
            
            // Wait for response (GUI window closed)
            match receiver.recv() {
                Ok(_response) => {
                    // Continue execution
                }
                Err(e) => {
                    eprintln!("Failed to receive response from debug server: {}", e);
                }
            }
        }
    }
}

/// The debug server process that receives debug messages and shows GUI
fn debug_server_process(rx: ipc::IpcReceiver<DebugMessage>, response_tx: ipc::IpcSender<DebugResponse>) {
    println!("Debug server started (PID: {})", process::id());
    
    loop {
        match rx.recv() {
            Ok(message) => {
                println!("\n=== DEBUG BREAKPOINT [{}:{}] ===", message.file, message.line);
                
                // Show GUI window and wait for it to be closed
                show_debug_gui(&message);
                
                // Send response to continue execution
                let response = DebugResponse {
                    continue_execution: true,
                };
                
                if let Err(e) = response_tx.send(response) {
                    eprintln!("Failed to send response: {}", e);
                    break;
                }
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

/// Show GUI window with debug variables (blocking until window is closed)
fn show_debug_gui(message: &DebugMessage) {
    use eframe::egui;
    
    // Extract just the filename from the full path for cleaner display
    let filename = std::path::Path::new(&message.file)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&message.file);
    
    let window_title = format!("Debug Inspector - {}:{}", filename, message.line);
    
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 700.0])
            .with_title(&window_title)
            .with_resizable(true),
        ..Default::default()
    };
    
    let message_clone = message.clone();
    
    let _ = eframe::run_simple_native(
        &window_title,
        options,
        move |ctx, _frame| {
            egui::CentralPanel::default().show(ctx, |ui| {
                // Header with location information
                ui.heading("🔍 Debug Breakpoint");
                ui.separator();
                
                // Location information panel
                egui::Frame::none()
                    .fill(egui::Color32::from_gray(240))
                    .inner_margin(10.0)
                    .rounding(5.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.strong("📂 File:");
                            ui.label(&message_clone.file);
                        });
                        ui.horizontal(|ui| {
                            ui.strong("📍 Line:");
                            ui.label(message_clone.line.to_string());
                            ui.strong("Column:");
                            ui.label(message_clone.column.to_string());
                        });
                        ui.horizontal(|ui| {
                            ui.strong("⏰ Timestamp:");
                            ui.label(format_timestamp(message_clone.timestamp));
                        });
                    });
                
                ui.add_space(10.0);
                ui.strong("Variables:");
                ui.separator();
                
                egui::ScrollArea::vertical().show(ui, |ui| {
                    // Create a table-like display
                    egui::Grid::new("debug_table")
                        .num_columns(2)
                        .spacing([40.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            // Table headers
                            ui.strong("Variable Name");
                            ui.strong("Value");
                            ui.end_row();
                            
                            // Table rows
                            for (name, value) in &message_clone.variables {
                                ui.label(name);
                                ui.label(value);
                                ui.end_row();
                            }
                        });
                });
                
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("▶ Continue Execution").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    ui.label("Close this window to continue program execution");
                });
            });
        },
    );
}

#[macro_export]
macro_rules! inspect {
    ( $( $var:expr ),+ $(,)? ) => {{
        use std::collections::BTreeMap;
        let mut map = BTreeMap::new();
        $(
            map.insert(stringify!($var).to_string(), format!("{:#?}", $var));
        )+
        
        // Send to debug server and wait for GUI to be closed (blocking)
        // Capture file, line, and column information
        $crate::send_to_debug_server_and_wait(
            map.clone(), 
            file!(), 
            line!(), 
            column!()
        );
        
        map
    }};
}
