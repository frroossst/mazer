#![allow(dead_code)]
use mazer_dbg::inspect;

#[derive(Debug)]
struct User {
    id: u32,
    name: String,
    active: bool,
}

#[derive(Debug)]
struct Database {
    users: Vec<User>,
    connections: u32,
}

fn main() {
    let db = Database {
        users: vec![
            User {
                id: 1,
                name: "Alice".to_string(),
                active: true,
            },
            User {
                id: 2,
                name: "Bob".to_string(),
                active: false,
            },
        ],
        connections: 5,
    };

    inspect!(db);
}
