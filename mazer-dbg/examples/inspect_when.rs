use mazer_dbg::inspect_when;

fn main() {
    let number = 42;

    inspect_when!(number % 2 == 0, "The number is even", number);

    let number = 11;

    // you won't see this
    inspect_when!(number % 2 == 0, "The number is NOT even", number);
}
