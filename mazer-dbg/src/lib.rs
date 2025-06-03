//! A GUI-based variable inspection tool for Rust, useful for debugging without print statements.  
//! At runtime, `inspect!(...)` opens a window displaying your variables with pretty formatting.  
//! 
//! This was inspired by Suneido's Inspect tool, which allows you to inspect variables in a GUI.  
//! 
//! This works under the hood by forking the process and using IPC channels to communicate with a GUI server.
//! Only supported on Unix-like systems (Linux, macOS, etc.).
//! 
//! Usage:  
//!     ```
//!         mazer_dbg::inspect!(var1, var2, ...);
//!     ```
//! 

use ipc_channel::ipc;
use nix::unistd::{ForkResult, fork};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::process;
use std::sync::{Arc, Mutex, OnceLock, Once};



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
    variables: BTreeMap<String, VariableDebugFrame>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct DebugResponse {
    continue_execution: bool,
}

// Global channels for bidirectional communication
static DEBUG_SENDER: OnceLock<Arc<Mutex<ipc::IpcSender<DebugMessage>>>> = OnceLock::new();
static RESPONSE_RECEIVER: OnceLock<Arc<Mutex<ipc::IpcReceiver<DebugResponse>>>> = OnceLock::new();

/// #[deprecated(since = "2.0.0", note = "No longer needed; `inspect!` auto-initializes, init() is still called internally!")]
fn init() {
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
    variables: BTreeMap<String, VariableDebugFrame>,
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

fn create_json_from_variables(variables: &BTreeMap<String, VariableDebugFrame>) -> String {
    let mut json_parts = Vec::new();

    for (name, var_frame) in variables {
        let escaped_value = var_frame.value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t");

        // Escape the type name for JSON
        let escaped_type = var_frame.type_name
            .replace('\\', "\\\\")
            .replace('"', "\\\"");

        json_parts.push(format!("  \"{}\": \"{}\"", name, escaped_value));
        json_parts.push(format!("  \"{}_type\": \"{}\"", name, escaped_type));
        
        let size_value = if let Some(size) = var_frame.size_hint {
            size.to_string()
        } else {
            "unknown".to_string()
        };

        // size in bytes
        json_parts.push(format!("  \"{}_size\": \"{}\"", name, size_value));
    }

    format!("{{\n{}\n}}", json_parts.join(",\n"))
}

/// Copy text to clipboard
fn copy_to_clipboard(text: &str) -> Result<(), Box<dyn std::error::Error>> {
    use arboard::Clipboard;
    let mut clipboard = Clipboard::new()?;
    clipboard.set_text(text)?;
    Ok(())
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

    let message = message.clone();
    let _ = eframe::run_simple_native(&window_title, options, move |ctx, _frame| {
        let mut style = (*ctx.style()).clone();
        style.text_styles = [
            (egui::TextStyle::Heading, egui::FontId::proportional(2.0)),
            (egui::TextStyle::Body, egui::FontId::proportional(24.0)),
            (egui::TextStyle::Monospace, egui::FontId::monospace(22.0)),
            (egui::TextStyle::Button, egui::FontId::proportional(24.0)),
            (egui::TextStyle::Small, egui::FontId::proportional(18.0)),
        ]
        .into();
        style.wrap_mode = Some(egui::TextWrapMode::Wrap);
        ctx.set_style(style.clone());

        egui::CentralPanel::default().show(ctx, |ui| {
            // Top panel for copy button
            egui::TopBottomPanel::top("top_panel").show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("📋").clicked() {
                            let json_string = create_json_from_variables(&message.variables);
                            if let Err(e) = copy_to_clipboard(&json_string) {
                                eprintln!("Failed to copy to clipboard: {}", e);
                            }
                        }
                    });
                });
            });

            // Main content area
            egui::CentralPanel::default().show_inside(ui, |ui| {
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
                                        ui.strong("Name (Type, Size)");
                                        ui.strong("Value");
                                        ui.end_row();

                                        // Table rows
                                        use egui_extras::syntax_highlighting::{
                                            CodeTheme, code_view_ui,
                                        };
                                        let theme = CodeTheme::from_style(&style);
                                        for (name, var_frame) in &message.variables {
                                            // Format the name with type and size information
                                            let size_info = if let Some(size) = var_frame.size_hint {
                                                format!("{} bytes", size)
                                            } else {
                                                "unknown".to_string()
                                            };
                                            
                                            let name_with_info = format!("{}\n({}, {})", 
                                                name, 
                                                var_frame.type_name, 
                                                size_info
                                            );
                                            
                                            ui.label(name_with_info);
                                            code_view_ui(ui, &theme, &var_frame.value, "rs");
                                            ui.end_row();
                                        }
                                    });
                            },
                        );
                    });
            });
        });
    });
}

pub fn ensure_init() {
    START.call_once(|| {
        init(); 
    });
}


static START: Once = Once::new();

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VariableDebugFrame {
    pub name: String,
    pub value: String,
    pub type_name: String,
    pub size_hint: Option<usize>,
}

#[macro_export]
macro_rules! inspect {
    ( $( $var:expr ),+ $(,)? ) => {{
        $crate::ensure_init();
        use std::collections::BTreeMap;
        let mut map = BTreeMap::new();
        $(
        let type_name = std::any::type_name_of_val(&$var).to_string();
        let size_hint = std::mem::size_of_val(&$var);

        let vframe = $crate::VariableDebugFrame {
            name: stringify!($var).to_string(),
            value: format!("{:#?}", $var),
            type_name: type_name.clone(),
            size_hint: Some(size_hint),
        };

            map.insert(stringify!($var).to_string(), vframe);
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
