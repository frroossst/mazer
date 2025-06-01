use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};
use std::thread;
use eframe::egui::{self, CentralPanel, Label};
use eframe::{App, CreationContext, NativeOptions};


use std::sync::mpsc::{self, Sender, Receiver};
use once_cell::sync::OnceCell;

pub static GUI_SENDER: OnceCell<Sender<HashMap<String, String>>> = OnceCell::new();

pub fn init_inspector() {
    let (tx, rx): (Sender<HashMap<String, String>>, Receiver<HashMap<String, String>>) = mpsc::channel();
    GUI_SENDER.set(tx).expect("Inspector already initialized");

    std::thread::spawn(move || {
        loop {
            if let Ok(data) = rx.recv() {
                run_gui_blocking(data); // blocking GUI
            }
        }
    });
}

fn run_gui_blocking(data: HashMap<String, String>) {
    let app_data = std::sync::Arc::new(std::sync::Mutex::new(data));
    let options = eframe::NativeOptions::default();

    let _ = eframe::run_native(
        "Inspector",
        options,
        Box::new(|_cc| Box::new(InspectApp { data: app_data })),
    );
}



struct InspectApp {
    data: Arc<Mutex<HashMap<String, String>>>,
}

impl App for InspectApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {
            ui.heading("Inspect Variables");
            let map = self.data.lock().unwrap();
            for (k, v) in map.iter() {
                ui.horizontal(|ui| {
                    ui.label(k);
                    ui.label(" = ");
                    ui.add(Label::new(v).wrap(false));
                });
            }
        });
    }
}

pub fn spawn_gui(data: HashMap<String, String>) {
    let shared = Arc::new(Mutex::new(data));
    let app_data = shared.clone();

    let _ = thread::spawn(move || {
        let native_options = NativeOptions::default();
        let _ = eframe::run_native(
            "Inspector",
            native_options,
            Box::new(|_cc: &CreationContext| Box::new(InspectApp { data: app_data })),
        );
    }).join(); // block until window closed
}

// #[macro_export]
// macro_rules! inspect {
//     ( $($val:expr),+ $(,)? ) => {{
//         use std::collections::HashMap;
//         let mut map: HashMap<String, String> = HashMap::new();
//         $(
//             map.insert(stringify!($val).to_string(), format!("{:?}", $val));
//         )+
//         $crate::spawn_gui(map);
//     }};
// }

#[macro_export]
macro_rules! inspect {
    ( $($val:expr),+ $(,)? ) => {{
        let mut map = std::collections::HashMap::new();
        $(
            map.insert(stringify!($val).to_string(), format!("{:?}", $val));
        )+

        if let Some(sender) = $crate::GUI_SENDER.get() {
            sender.send(map).expect("Failed to send inspect data");
        } else {
            eprintln!("inspect! called before init_inspector()");
        }
    }};
}


