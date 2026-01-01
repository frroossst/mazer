use mazer_dbg::inspect;

fn main() {
    let name = "Alice";
    let age = 30;
    let bio = String::from("extrovert, leo");
    let hobbies = vec!["reading", "coding", "hiking"];

    inspect!(name, age, bio, hobbies);
}
