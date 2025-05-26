use crate::interpreter::{Environment, Interpreter};



#[derive(serde::Serialize, serde::Deserialize)]
#[derive(Debug, Clone)]
pub struct DebugContext {
    version: String,
    file_path: String,
    raw_content: String,
    html_content: String,
    parsed_content: String,
    environment: Environment,
}

impl DebugContext {
    pub fn new(file_path: String, raw_content: String, html_content: String, parsed_content: String, environment: Environment) -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            file_path,
            raw_content,
            html_content,
            parsed_content,
            environment,
        }
    }

    pub fn dump(&self) -> ! {
        let path_buf = std::path::PathBuf::from(&self.file_path);
        let output_path = path_buf.with_extension("bin");


        let ronny = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::new())
            .expect("Failed to serialize DebugContext to RON");

        std::fs::write(&output_path, ronny.clone()).expect("Failed to write DebugContext to file");
        eprintln!("Debug context dumped to: {}", output_path.display());

        // dump to stderr as well
        eprintln!("Debug Context Dump:");
        eprintln!("{}", ronny);

        std::process::exit(1);
    }

}

    

