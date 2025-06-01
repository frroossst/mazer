
use nix::unistd::{fork, ForkResult};
use eframe::egui;



pub fn init() {
    match unsafe { fork() } {
        Ok(ForkResult::Parent { child }) => {
            return;
        }
        Ok(ForkResult::Child) => {
            // setup GUI stuff
            // spin up a window GUI

            // spin up GUI in THIS main thread
            make_gui();

        }
        Err(err) => {
            panic!("{}", err);
        }
    }
}

#[derive(Default)]
struct DebugWindow;

impl eframe::App for DebugWindow {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Hello, World!");
            ui.label("This is a simple eGUI application.");
            ui.label("It displays some text in a window.");
            
            ui.separator();
            
            ui.label("You can add more text here:");
            ui.label("• Line 1");
            ui.label("• Line 2");
            ui.label("• Line 3");
        });
    }

}



fn make_gui() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([400.0, 300.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Simple eGUI App",
        options,
        Box::new(|_cc| Box::<DebugWindow>::default()),
    ).unwrap();
}


#[macro_export]
macro_rules! inspect {
    ( $( $var:expr ),+ $(,)? ) => {{
        use std::collections::BTreeMap;
        let mut map = BTreeMap::new();
        $(
            map.insert(stringify!($var), format!("{:#?}", $var));
        )+
        dbg!(&map);
        map
    }};
}

