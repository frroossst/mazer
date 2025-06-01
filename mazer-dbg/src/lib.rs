use nix::unistd::{fork, ForkResult};
use eframe::egui;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::net::{TcpListener, TcpStream, SocketAddr};
use std::io::{Read, Write, BufReader, BufRead};
use serde::{Serialize, Deserialize};

// Shared state for the GUI
#[derive(Clone, Default)]
struct DebugData {
    entries: Arc<Mutex<Vec<BTreeMap<String, String>>>>,
}

// Message format for communication
#[derive(Serialize, Deserialize, Debug)]
struct InspectMessage {
    data: BTreeMap<String, String>,
}

static mut DEBUG_SERVER_PORT: Option<u16> = None;

pub fn init() {
    match unsafe { fork() } {
        Ok(ForkResult::Parent { child }) => {
            // Give the child process time to start the server
            std::thread::sleep(std::time::Duration::from_millis(100));
            return;
        }
        Ok(ForkResult::Child) => {
            // Child process: run server and GUI
            let debug_data = DebugData::default();
            
            // Start server in background thread
            let server_data = debug_data.clone();
            let port = start_server(server_data);
            
            // Store port globally so inspect! macro can use it
            unsafe {
                DEBUG_SERVER_PORT = Some(port);
            }
            
            // Run GUI on main thread
            make_gui(debug_data);
        }
        Err(err) => {
            panic!("Fork failed: {}", err);
        }
    }
}

fn start_server(debug_data: DebugData) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to address");
    let port = listener.local_addr().unwrap().port();
    
    // Write port to a file so parent process can read it
    std::fs::write("/tmp/debug_server_port", port.to_string())
        .expect("Failed to write port file");
    
    thread::spawn(move || {
        println!("Debug server listening on port {}", port);
        
        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    let debug_data = debug_data.clone();
                    thread::spawn(move || {
                        handle_client(&mut stream, debug_data);
                    });
                }
                Err(e) => {
                    eprintln!("Error accepting connection: {}", e);
                }
            }
        }
    });
    
    port
}

fn handle_client(stream: &mut TcpStream, debug_data: DebugData) {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    
    match reader.read_line(&mut line) {
        Ok(_) => {
            if let Ok(message) = serde_json::from_str::<InspectMessage>(&line.trim()) {
                println!("Received debug data: {:#?}", message.data);
                
                // Add to GUI data
                if let Ok(mut entries) = debug_data.entries.lock() {
                    entries.push(message.data);
                }
            }
        }
        Err(e) => {
            eprintln!("Error reading from client: {}", e);
        }
    }
}

#[derive(Default)]
struct DebugWindow {
    debug_data: DebugData,
}

impl DebugWindow {
    fn new(debug_data: DebugData) -> Self {
        Self { debug_data }
    }
}

impl eframe::App for DebugWindow {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Debug Inspector");
            ui.separator();
            
            // Display debug entries
            if let Ok(entries) = self.debug_data.entries.lock() {
                if entries.is_empty() {
                    ui.label("No debug data received yet...");
                } else {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for (i, entry) in entries.iter().enumerate() {
                            ui.group(|ui| {
                                ui.label(format!("Entry #{}", i + 1));
                                ui.separator();
                                
                                for (key, value) in entry {
                                    ui.horizontal(|ui| {
                                        ui.label(format!("{}:", key));
                                        ui.separator();
                                    });
                                    ui.label(value);
                                    ui.separator();
                                }
                            });
                            ui.add_space(10.0);
                        }
                    });
                }
            }
            
            ui.separator();
            if ui.button("Clear All").clicked() {
                if let Ok(mut entries) = self.debug_data.entries.lock() {
                    entries.clear();
                }
            }
        });
        
        // Request repaint to keep GUI responsive
        ctx.request_repaint();
    }
}

fn make_gui(debug_data: DebugData) {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 400.0])
            .with_title("Debug Inspector"),
        ..Default::default()
    };
    
    eframe::run_native(
        "Debug Inspector",
        options,
        Box::new(move |_cc| Box::new(DebugWindow::new(debug_data))),
    ).unwrap();
}

pub fn send_to_debug_server(data: BTreeMap<String, String>) {
    // Try to read port from file
    let port = match std::fs::read_to_string("/tmp/debug_server_port") {
        Ok(port_str) => match port_str.trim().parse::<u16>() {
            Ok(port) => port,
            Err(_) => {
                eprintln!("Failed to parse port from file");
                return;
            }
        },
        Err(_) => {
            eprintln!("Debug server not available (port file not found)");
            return;
        }
    };
    
    let message = InspectMessage { data };
    let json = match serde_json::to_string(&message) {
        Ok(json) => json,
        Err(e) => {
            eprintln!("Failed to serialize debug data: {}", e);
            return;
        }
    };
    
    match TcpStream::connect(format!("127.0.0.1:{}", port)) {
        Ok(mut stream) => {
            if let Err(e) = writeln!(stream, "{}", json) {
                eprintln!("Failed to send debug data: {}", e);
            }
        }
        Err(e) => {
            eprintln!("Failed to connect to debug server: {}", e);
        }
    }
}

#[macro_export]
macro_rules! inspect {
    ( $( $var:expr ),+ $(,)? ) => {{
        use std::collections::BTreeMap;
        let mut map = BTreeMap::new();
        $(
            map.insert(stringify!($var).to_string(), format!("{:#?}", $var));
        )+
        
        // Print to console as before
        dbg!(&map);
        
        // Send to debug server if available
        $crate::send_to_debug_server(map.clone());
        
        map
    }};
}

// Convenience function to check if debug server is running
pub fn is_debug_server_available() -> bool {
    std::fs::metadata("/tmp/debug_server_port").is_ok()
}
