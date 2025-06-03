use mazer_dbg::inspect;

fn main() {
    let name = "Alice";
    let age = 30;
    let hobbies = vec!["reading", "coding", "hiking"];
    
    inspect!(name, age, hobbies);
}

