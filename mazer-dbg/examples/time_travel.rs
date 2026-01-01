use mazer_dbg::inspect;

fn main() {
    // First debug point
    let mut x = 42;
    let name = "First checkpoint";
    inspect!(x, name);

    // Second debug point
    x += 10;
    let y = vec![1, 2, 3, 4, 5];
    let status = "Second checkpoint";
    inspect!(x, y, status);

    // Third debug point
    x += 5;
    let z = format!("Hello from frame {}", 3);
    let data = std::collections::HashMap::from([("key1", "value1"), ("key2", "value2")]);
    inspect!(x, y, z, data);

    // Fourth debug point
    x *= 2;
    let final_value = x * 2 + y.len();
    let message = "Final checkpoint - you can navigate back to see previous values";
    inspect!(x, final_value, message);
}
