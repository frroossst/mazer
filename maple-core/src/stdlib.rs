use std::collections::HashMap;

pub struct InBuiltFunctionRegistry {
    registry: Vec<String>,
}

impl InBuiltFunctionRegistry {

    pub fn new() -> Self {
        InBuiltFunctionRegistry {
            registry: vec![
                String::from("integral"),
                String::from("dot"),
                String::from("vec"),
                String::from("matrix"),
                "foo".to_string(),
                "bar".to_string(),
                "qux".to_string(),
            ]
        }
    }

    pub fn is_function(&self, func: &str) -> bool {
        self.registry.contains(&func.to_string())
    }

}
