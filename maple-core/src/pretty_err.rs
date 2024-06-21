#[derive(Debug)]
pub struct DebugContext;

impl DebugContext {

    pub fn new(file_title: &str) -> Self {
        DebugContext
    }

    pub fn panic(&self, msg: &str) -> ! {
        eprintln!("Panic: {}", msg);
        panic!("DebugContext::panic() called")
    }
}