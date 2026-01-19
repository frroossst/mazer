//! A GUI-based variable inspection tool for Rust, useful for debugging without print statements.  
//! At runtime, `inspect!(...)` opens a window displaying your variables with pretty formatting.  
//!
//! This was inspired by Suneido's Inspect tool, which allows you to inspect variables in a GUI.  
//!
//! This works under the hood by forking the process and using IPC channels to communicate with a GUI server.
//! Only supported on Unix-like systems (Linux, macOS, etc.).
//!
//! This also allows you to "time travel" through your debug frames, letting you go back and forth through previous variable states.
//! NOTE: This does not change the execution of your program, only debug's inspect history.
//!
//! Usage:  
//!     ```
//!         mazer_dbg::inspect!(var1, var2, ...);
//!         mazer_dbg::inspect_when!(condition, var1, var2, ...);
//!     ```
//!
//! inspect!() will automatically initialize the debug server if it hasn't been done yet.
//!
//! The library wraps in #[cfg(debug_assertions)] so it only compiles in debug builds.
//! No runtime cost in release builds, as all inspect!() calls are optimized out.
//!

#![allow(dead_code)]
#![allow(unused_imports)]

use ipc_channel::ipc;
use nix::unistd::{ForkResult, fork};
use serde::{Deserialize, Serialize};
use smartstring::alias::String;
use std::collections::{BTreeMap, VecDeque};
use std::process;
use std::sync::{Arc, Mutex, Once, OnceLock};

#[cfg(not(unix))]
compile_error!("This crate is only supported on Unix-like systems (Linux, macOS, etc.)");

#[derive(Serialize, Deserialize, Debug, Clone)]
struct DebugMessage {
    timestamp: u64,
    file: String,
    line: u32,
    column: u32,
    variables: BTreeMap<String, VariableDebugFrame>,
    backtrace: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct DebugResponse {
    continue_execution: bool,
}

/// Debug frame history manager for time traveling
///
/// Stores all debug frames without limit to allow comprehensive debugging sessions.
/// Memory usage grows with the number of inspect!() calls, but this provides maximum
/// flexibility for debugging complex applications.
#[derive(Debug, Clone)]
struct DebugFrameHistory {
    frames: VecDeque<DebugMessage>,
    current_index: usize,
}

impl DebugFrameHistory {
    fn new() -> Self {
        Self {
            frames: VecDeque::new(),
            current_index: 0,
        }
    }

    fn add_frame(&mut self, frame: DebugMessage) {
        self.frames.push_back(frame);
        self.current_index = self.frames.len().saturating_sub(1);
    }

    fn go_backward(&mut self) -> bool {
        if self.current_index > 0 {
            self.current_index -= 1;
            true
        } else {
            false
        }
    }

    fn go_forward(&mut self) -> bool {
        if self.current_index + 1 < self.frames.len() {
            self.current_index += 1;
            true
        } else {
            false
        }
    }

    fn get_current_frame(&self) -> Option<&DebugMessage> {
        self.frames.get(self.current_index)
    }

    fn can_go_backward(&self) -> bool {
        self.current_index > 0
    }

    fn can_go_forward(&self) -> bool {
        self.current_index + 1 < self.frames.len()
    }

    fn get_position_info(&self) -> (usize, usize) {
        (self.current_index + 1, self.frames.len())
    }
}

// Global channels for bidirectional communication
static DEBUG_SENDER: OnceLock<Arc<Mutex<ipc::IpcSender<DebugMessage>>>> = OnceLock::new();
static RESPONSE_RECEIVER: OnceLock<Arc<Mutex<ipc::IpcReceiver<DebugResponse>>>> = OnceLock::new();

/// #[deprecated(since = "2.0.0", note = "No longer needed; `inspect!` auto-initializes, init() is still called internally!")]
#[cfg(debug_assertions)]
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
    backtrace: String,
) {
    if let (Some(sender), Some(receiver)) = (DEBUG_SENDER.get(), RESPONSE_RECEIVER.get())
        && let (Ok(sender), Ok(receiver)) = (sender.lock(), receiver.lock())
    {
        let message = DebugMessage {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            file: file.into(),
            line,
            column,
            variables,
            backtrace,
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

/// The debug server process that receives debug messages and shows GUI
fn debug_server_process(
    rx: ipc::IpcReceiver<DebugMessage>,
    response_tx: ipc::IpcSender<DebugResponse>,
) {
    let mut frame_history = DebugFrameHistory::new();

    loop {
        match rx.recv() {
            Ok(message) => {
                frame_history.add_frame(message);
                show_debug_gui_with_history(&mut frame_history);

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
        let escaped_value = var_frame
            .value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t");

        // Escape the type name for JSON
        let escaped_type = var_frame
            .type_name
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

    format!("{{\n{}\n}}", json_parts.join(",\n")).into()
}

/// Copy text to clipboard
fn copy_to_clipboard(text: &str) -> Result<(), Box<dyn std::error::Error>> {
    use arboard::Clipboard;
    let mut clipboard = Clipboard::new()?;
    clipboard.set_text(text)?;
    Ok(())
}

/// Show GUI window with debug variables and time traveling functionality
fn show_debug_gui_with_history(frame_history: &mut DebugFrameHistory) {
    use eframe::egui;
    use std::sync::{Arc, Mutex};

    if let Some(current_message) = frame_history.get_current_frame() {
        let filename = std::path::Path::new(&*current_message.file)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(&current_message.file);

        let window_title = format!("[Mazer Debug] - {}:{}", filename, current_message.line);

        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([900.0, 700.0])
                .with_title(&window_title)
                .with_resizable(true),
            ..Default::default()
        };

        let frame_history_arc = Arc::new(Mutex::new(frame_history.clone()));
        let frame_history_gui = frame_history_arc.clone();
        let show_backtrace = Arc::new(Mutex::new(false));
        let show_backtrace_gui = show_backtrace.clone();

        let _ = eframe::run_simple_native(&window_title, options, move |ctx, _frame| {
            let mut style = (*ctx.style()).clone();
            style.text_styles = [
                (egui::TextStyle::Heading, egui::FontId::proportional(20.0)),
                (egui::TextStyle::Body, egui::FontId::proportional(24.0)),
                (egui::TextStyle::Monospace, egui::FontId::monospace(22.0)),
                (egui::TextStyle::Button, egui::FontId::proportional(24.0)),
                (egui::TextStyle::Small, egui::FontId::proportional(18.0)),
            ]
            .into();
            style.wrap_mode = Some(egui::TextWrapMode::Wrap);
            if style.visuals.dark_mode {
                style.visuals.override_text_color = Some(egui::Color32::WHITE);
            }
            ctx.set_style(style.clone());

            egui::CentralPanel::default().show(ctx, |ui| {
                let mut frame_history_guard = frame_history_gui.lock().unwrap();

                // Top panel for navigation and copy button
                egui::TopBottomPanel::top("top_panel").show_inside(ui, |ui| {
                    ui.horizontal(|ui| {
                        // Time traveling controls on the left
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            let can_go_back = frame_history_guard.can_go_backward();
                            let can_go_forward = frame_history_guard.can_go_forward();

                            ui.add_enabled_ui(can_go_back, |ui| {
                                if ui.button("⬅ Previous").clicked() {
                                    frame_history_guard.go_backward();
                                }
                            });

                            ui.add_enabled_ui(can_go_forward, |ui| {
                                if ui.button("Next ➡").clicked() {
                                    frame_history_guard.go_forward();
                                }
                            });

                            let (current_pos, total_frames) =
                                frame_history_guard.get_position_info();
                            ui.label(format!("Frame {}/{}", current_pos, total_frames));
                        });

                        // Copy button and backtrace button on the right
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if let Some(current_frame) = frame_history_guard.get_current_frame() {
                                if ui.button("📋").clicked() {
                                    let json_string =
                                        create_json_from_variables(&current_frame.variables);
                                    if let Err(e) = copy_to_clipboard(&json_string) {
                                        eprintln!("Failed to copy to clipboard: {}", e);
                                    }
                                }

                                if ui.button("📋 Backtrace").clicked() {
                                    *show_backtrace_gui.lock().unwrap() = true;
                                }
                            }
                        });
                    });
                });

                // Main content area
                egui::CentralPanel::default().show_inside(ui, |ui| {
                    if let Some(current_frame) = frame_history_guard.get_current_frame() {
                        // Show current frame info
                        ui.horizontal(|ui| {
                            ui.strong("File:");
                            ui.label(&*current_frame.file);
                            ui.strong("Line:");
                            ui.label(current_frame.line.to_string());
                            ui.strong("Column:");
                            ui.label(current_frame.column.to_string());
                        });
                        ui.separator();

                        egui::ScrollArea::vertical()
                            .max_width(f32::INFINITY)
                            .auto_shrink([false; 2])
                            .scroll_bar_visibility(
                                egui::scroll_area::ScrollBarVisibility::AlwaysVisible,
                            )
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
                                                for (name, var_frame) in &current_frame.variables {
                                                    // Format the name with type and size information
                                                    let size_info =
                                                        if let Some(size) = var_frame.size_hint {
                                                            format!("{} bytes", size)
                                                        } else {
                                                            "unknown".to_string()
                                                        };

                                                    let name_with_info = format!(
                                                        "{}\n\n\t{}\n\t{}\n",
                                                        name, size_info, var_frame.type_name,
                                                    );

                                                    ui.label(name_with_info);
                                                    code_view_ui(
                                                        ui,
                                                        &theme,
                                                        &var_frame.value,
                                                        "rs",
                                                    );
                                                    ui.end_row();
                                                }
                                            });
                                    },
                                );
                            });
                    }
                });
            });

            // Backtrace window
            let mut show_backtrace_state = show_backtrace_gui.lock().unwrap();
            if *show_backtrace_state {
                let mut open = true;
                egui::Window::new("🔍 Call Stack / Backtrace")
                    .open(&mut open)
                    .default_width(800.0)
                    .default_height(600.0)
                    .resizable(true)
                    .collapsible(false)
                    .show(ctx, |ui| {
                        let frame_history_guard = frame_history_gui.lock().unwrap();
                        if let Some(current_frame) = frame_history_guard.get_current_frame() {
                            ui.horizontal(|ui| {
                                ui.strong("Backtrace for:");
                                ui.label(format!("{}:{}", current_frame.file, current_frame.line));
                                ui.separator();
                                if ui.button("📋 Copy").clicked()
                                    && let Err(e) = copy_to_clipboard(&current_frame.backtrace) {
                                        eprintln!("Failed to copy backtrace to clipboard: {}", e);
                                    }
                            });
                            ui.separator();

                            egui::ScrollArea::vertical()
                                .max_width(f32::INFINITY)
                                .auto_shrink([false; 2])
                                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                                .show(ui, |ui| {
                                    // Format and display the backtrace nicely
                                    let backtrace_lines: Vec<&str> = current_frame.backtrace.lines().collect();
                                    
                                    for line in backtrace_lines.iter() {
                                        let line = line.trim();
                                        if line.is_empty() {
                                            continue;
                                        }

                                        ui.horizontal(|ui| {
                                            // Determine line color and styling based on content
                                            if line.contains("::") && (line.contains(".rs:") || line.contains("src/")) {
                                                // This looks like a Rust function call with file info
                                                ui.colored_label(egui::Color32::LIGHT_BLUE, line);
                                            } else if line.starts_with("at ") {
                                                // File location line
                                                ui.colored_label(egui::Color32::LIGHT_GREEN, line);
                                            } else if line.contains("libstd") || line.contains("libcore") || line.contains("liballoc") {
                                                // Standard library calls
                                                ui.colored_label(egui::Color32::GRAY, line);
                                            } else {
                                                // Default formatting
                                                ui.label(line);
                                            }
                                        });
                                    }
                                    
                                    // If backtrace is empty or doesn't contain useful info
                                    if backtrace_lines.is_empty() || backtrace_lines.len() < 2 {
                                        ui.colored_label(egui::Color32::YELLOW, "⚠ Backtrace information not available or limited");
                                        ui.label("Try enabling debug symbols or running in debug mode for more detailed backtraces.");
                                    }
                                });
                        }
                    });

                if !open {
                    *show_backtrace_state = false;
                }
            }
        });

        // Update the original frame_history with any navigation changes
        if let Ok(updated_history) = frame_history_arc.lock() {
            *frame_history = updated_history.clone();
        }
    }
}

#[cfg(debug_assertions)]
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
        #[cfg(debug_assertions)]
        {
            $crate::ensure_init();
            use std::collections::BTreeMap;
            use smartstring::{SmartString, LazyCompact};
            use $crate::VariableDebugFrame;
            let mut map: BTreeMap<SmartString<LazyCompact>, VariableDebugFrame> = BTreeMap::new();
            $(
            let var_name: String = stringify!($var).into();

            let type_name = std::any::type_name_of_val(&$var).to_string();
            let size_hint = std::mem::size_of_val(&$var);

            let vframe = $crate::VariableDebugFrame {
                name: var_name.clone().into(),
                value: format!("{:#?}", $var).into(),
                type_name: type_name.clone().into(),
                size_hint: Some(size_hint),
            };

                map.insert(var_name.into(), vframe);
            )+

            let bt = std::backtrace::Backtrace::force_capture();

            // Send to debug server and wait for GUI to be closed (blocking)
            $crate::send_to_debug_server_and_wait(
                map.clone().into(),
                file!(),
                line!(),
                column!(),
                format!("{}", bt).into(),
            );

            map
        }
    }};
}

#[macro_export]
/// A conditional version of inspect! that only inspects if the condition is true
/// Does have the runtime cost of evaluating the condition, but avoids inspecting
/// variables when not needed
macro_rules! inspect_when {
    ($condition:expr, $( $var:expr ),+ $(,)? ) => {{
        #[cfg(debug_assertions)]
        {
            if $condition {
                $crate::inspect!($($var),+);
            }
        }
    }};
}
