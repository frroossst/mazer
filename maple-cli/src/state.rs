use notify::{Error, Event, Watcher};

pub struct State {
    path: String,
    title: String,
    rx: std::sync::mpsc::Receiver<Result<Event, Error>>,
    _watcher: notify::RecommendedWatcher,
}

impl State {
    pub fn new(path: String, title: String) -> Self {
        let path_t = std::path::PathBuf::from(&path);
        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher = notify::RecommendedWatcher::new(tx, notify::Config::default()).unwrap();
        watcher.watch(&path_t, notify::RecursiveMode::Recursive).unwrap();

        Self { path, title, rx, _watcher: watcher }
    }

    pub fn has_file_changed(&mut self) -> bool {
        let mut has_changed = false;
        while let Ok(event) = self.rx.try_recv() {
            match event {
                Ok(Event { kind: notify::EventKind::Modify(_), .. }) => {
                    has_changed = true;
                },
                _ => {},
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
