use notify::{Error, Event, Watcher};

pub struct State {
    path: String,
    title: String,
    cold_start: bool,
    rx: std::sync::mpsc::Receiver<Result<Event, Error>>,
    _watcher: notify::RecommendedWatcher,
}

impl State {
    pub fn new(path: String, title: String) -> Self {
        let path_t = std::path::PathBuf::from(&path);
        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher = notify::RecommendedWatcher::new(tx, notify::Config::default()).unwrap();
        watcher.watch(&path_t, notify::RecursiveMode::Recursive).unwrap();

        Self { 
            path, 
            title, 
            cold_start: true,
            rx, 
            _watcher: watcher }
    }

    // NOTE: do not call this function directly for debugging
    pub fn has_file_changed(&mut self) -> bool {
        if self.cold_start {
            self.cold_start = false;
            return true;
        }
        let mut has_changed = false;
        while let Ok(event) = self.rx.try_recv() {
            if let Ok(Event { kind: notify::EventKind::Modify(_), .. }) = event {
                has_changed = true;
            }
        }
        has_changed
    }

    pub fn title(&self) -> String {
        self.title.clone()
    }

    pub fn path(&self) -> String {
        self.path.clone()
    }
}
