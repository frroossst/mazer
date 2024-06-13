use std::collections::HashMap;

#[derive(Debug)]
pub enum ByteCode {
}

#[derive(Debug)]
pub struct VirtualMachine {
    stack: Vec<f64>,
    // bytecode for inbuilt functions
    lib: HashMap<String, Vec<ByteCode>>,
    ip: usize,
    sp: usize,
}

impl VirtualMachine {
    pub fn new() -> Self {
        VirtualMachine {
            stack: Vec::new(),
            lib: HashMap::new(),
            ip: 0,
            sp: 0,
        }
    }
}

