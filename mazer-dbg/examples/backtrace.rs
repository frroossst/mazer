use mazer_dbg::inspect;

fn does_panic(result: u32) {
    inspect!(result);
    panic!("oops!");
}

fn fibonacci(n: u32) -> u32 {
    if n <= 1 {
        return n;
    }
    fibonacci(n - 1) + fibonacci(n - 2)
}

fn main() {
    let n = 10;
    let r = fibonacci(n);

    does_panic(r);
}
