use ipc_channel::ipc;
use nix::unistd::{ForkResult, fork};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::process;
use std::sync::{Arc, Mutex, OnceLock};



#[cfg(test)]
mod tests;
#[cfg(not(unix))]
compile_error!("This crate is only supported on Unix-like systems (Linux, macOS, etc.)");



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

pub fn init() {
    let (debug_tx, debug_rx) = match ipc::channel() {
        Ok(channel) => channel,
        Err(e) => {
            panic!("Failed to create debug IPC channel: {}", e);
        }
    };

    let (response_tx, response_rx) = match ipc::channel() {
        Ok(channel) => channel,
        Err(e) => {
            panic!("Failed to create response IPC channel: {}", e);
        }
    };

    match unsafe { fork() } {
        Ok(ForkResult::Parent { child: _ }) => {
            if DEBUG_SENDER.set(Arc::new(Mutex::new(debug_tx))).is_err() {
                panic!("Failed to set debug sender");
            }

            if RESPONSE_RECEIVER
                .set(Arc::new(Mutex::new(response_rx)))
                .is_err()
            {
                panic!("Failed to set response receiver");
            }
        }
        Ok(ForkResult::Child) => {
            debug_server_process(debug_rx, response_tx);
            unreachable!()
        }
        Err(e) => {
            panic!("Fork failed: {}", e);
        }
    }
}

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

            if let Err(e) = sender.send(message) {
                eprintln!("Failed to send debug message: {}", e);
                return;
            }

            // Wait for response (GUI window closed)
            match receiver.recv() {
                Ok(_response) => {
                    // continue execution
                }
                Err(e) => {
                    eprintln!("Failed to receive response from debug server: {}", e);
                }
            }
        }
    }
}

/// The debug server process that receives debug messages and shows GUI
fn debug_server_process(
    rx: ipc::IpcReceiver<DebugMessage>,
    response_tx: ipc::IpcSender<DebugResponse>,
) {
    loop {
        match rx.recv() {
            Ok(message) => {
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
                eprintln!("Debug server: Channel error: {}", e);
                break;
            }
        }
    }
    process::exit(0);
}

/// Show GUI window with debug variables (blocking until window is closed)
fn show_debug_gui(message: &DebugMessage) {
    use eframe::egui;

    let filename = std::path::Path::new(&message.file)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&message.file);

    let window_title = format!("[Mazer Debug] - {}:{}", filename, message.line);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 700.0])
            .with_title(&window_title)
            .with_resizable(true),
        ..Default::default()
    };

    let message_clone = message.clone();

    let _ = eframe::run_simple_native(&window_title, options, move |ctx, _frame| {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Header with location information
            ui.heading("🔍 Debug Breakpoint");
            ui.separator();

            egui::ScrollArea::vertical()
                .max_width(f32::INFINITY)
                .auto_shrink([false; 2])
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                .show(ui, |ui| {
                    ui.allocate_ui_with_layout(
                        ui.available_size(),
                        egui::Layout::left_to_right(egui::Align::Min),
                        |ui| {
                            egui::Grid::new("debug_table")
                                .num_columns(2)
                                .spacing([40.0, 4.0])
                                .striped(true)
                                .min_col_width(ui.available_width() / 2.0)
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
                        },
                    );
                });

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("▶ Continue Execution").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                ui.label("Close this window to continue program execution");
            });
        });
    });
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