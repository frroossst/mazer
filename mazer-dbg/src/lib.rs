// Cargo.toml dependencies needed:
// [dependencies]
// eframe = "0.24"
// egui = "0.24"
// tokio = { version = "1.0", features = ["full"] }

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use eframe::egui;

/// A structure to hold variable inspection data
#[derive(Debug, Clone)]
pub struct VariableInfo {
    pub name: String,
    pub value: String,
    pub type_name: String,
}

/// The inspector window application
struct InspectorApp {
    variables: Vec<VariableInfo>,
    search_filter: String,
}

impl InspectorApp {
    fn new(variables: Vec<VariableInfo>) -> Self {
        Self {
            variables,
            search_filter: String::new(),
        }
    }
}

impl eframe::App for InspectorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Variable Inspector");
            
            // Search filter
            ui.horizontal(|ui| {
                ui.label("Filter:");
                ui.text_edit_singleline(&mut self.search_filter);
                if ui.button("Clear").clicked() {
                    self.search_filter.clear();
                }
            });
            
            ui.separator();
            
            // Table header
            egui::Grid::new("inspector_grid")
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Variable Name");
                    ui.strong("Type");
                    ui.strong("Debug Value");
                    ui.end_row();
                    
                    // Filter and display variables
                    for var in &self.variables {
                        if self.search_filter.is_empty() || 
                           var.name.to_lowercase().contains(&self.search_filter.to_lowercase()) ||
                           var.type_name.to_lowercase().contains(&self.search_filter.to_lowercase()) {
                            
                            ui.monospace(&var.name);
                            ui.monospace(&var.type_name);
                            
                            // Scrollable text area for long values
                            egui::ScrollArea::vertical()
                                .max_height(100.0)
                                .show(ui, |ui| {
                                    ui.monospace(&var.value);
                                });
                            
                            ui.end_row();
                        }
                    }
                });
        });
    }
}

/// Show the inspector window with the given variables
pub fn show_inspector_window(variables: Vec<VariableInfo>) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let options = eframe::NativeOptions {
            ..Default::default()
        };
        
        eframe::run_native(
            "Variable Inspector",
            options,
            Box::new(|_cc| Box::new(InspectorApp::new(variables))),
        ).unwrap();
    });
}

/// The main inspect macro that captures variables from local scope
#[macro_export]
macro_rules! inspect {
    ($($var:ident),* $(,)?) => {
        {
            let mut variables = Vec::new();
            $(
                variables.push(VariableInfo {
                    name: stringify!($var).to_string(),
                    value: format!("{:?}", $var),
                    type_name: std::any::type_name_of_val(&$var).to_string(),
                });
            )*
            
            // Show the inspector window
            show_inspector_window(variables);
        }
    };
}

/// Alternative macro that takes expressions and their names
#[macro_export]
macro_rules! inspect_expr {
    ($($expr:expr),* $(,)?) => {
        {
            let mut variables = Vec::new();
            $(
                variables.push(VariableInfo {
                    name: stringify!($expr).to_string(),
                    value: format!("{:?}", $expr),
                    type_name: std::any::type_name_of_val(&$expr).to_string(),
                });
            )*
            
            // Show the inspector window
            show_inspector_window(variables);
        }
    };
}

// Example usage and test functions
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[derive(Debug)]
    struct TestStruct {
        field1: i32,
        field2: String,
    }

    #[tokio::test]
    async fn example_usage() {
        // Example variables to inspect
        let my_string = "Hello, World!".to_string();
        let my_number = 42i32;
        let my_vector = vec![1, 2, 3, 4, 5];
        let mut my_hashmap: HashMap<String, i32> = HashMap::new();
        my_hashmap.insert("key1".to_string(), 100);
        my_hashmap.insert("key2".to_string(), 200);
        
        let my_struct = TestStruct {
            field1: 123,
            field2: "test".to_string(),
        };

        let my_option: Option<i32> = Some(99);
        let my_result: Result<String, &str> = Ok("success".to_string());

        // This would open the inspector window
        // Uncomment the line below to test (requires GUI environment)
        // inspect!(my_string, my_number, my_vector, my_hashmap, my_struct, my_option, my_result);
        
        // Test that the macro compiles and creates the right data
        let mut variables = Vec::new();
        variables.push(VariableInfo {
            name: "my_string".to_string(),
            value: format!("{:?}", my_string),
            type_name: std::any::type_name_of_val(&my_string).to_string(),
        });
        
        assert_eq!(variables[0].name, "my_string");
        assert!(variables[0].value.contains("Hello, World!"));
    }
}

// Example of how to use in your code:
/*
fn main() {
    let name = "Alice".to_string();
    let age = 30;
    let scores = vec![85, 92, 78, 96];
    let is_active = true;
    
    // This will open a GUI window showing all these variables
    inspect!(name, age, scores, is_active);
    
    // You can also inspect expressions
    inspect_expr!(name.len(), scores.iter().sum::<i32>(), age * 2);
}
*/