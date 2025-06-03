use mazer_dbg::{inspect, inspect_ron};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Person {
    name: String,
    age: u32,
    email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Company {
    name: String,
    employees: Vec<Person>,
    founded: u32,
}

fn main() {
    let person1 = Person {
        name: "Alice".to_string(),
        age: 30,
        email: Some("alice@example.com".to_string()),
    };

    let person2 = Person {
        name: "Bob".to_string(),
        age: 25,
        email: None,
    };

    let company = Company {
        name: "Tech Corp".to_string(),
        employees: vec![person1.clone(), person2.clone()],
        founded: 2020,
    };

    let mut scores = HashMap::new();
    scores.insert("Alice", 95);
    scores.insert("Bob", 87);
    scores.insert("Charlie", 92);

    // Regular inspect - uses Debug formatting
    println!("Using regular inspect! macro:");
    inspect!(person1, company, scores);

    // RON inspect - uses RON serialization for better structured output
    println!("Using inspect_ron! macro:");
    inspect_ron!(person2, company, scores);

    println!("Example completed! Check the debug windows that opened.");
}
