use mazer_dbg::inspect;

fn divide(a: f64, b: f64) -> Result<f64, String> {
    if b == 0.0 {
        let error = "Cannot divide by zero";
        inspect!(a, b, error);
        return Err(error.to_string());
    }
    
    let result = a / b;
    inspect!(a, b, result);
    Ok(result)
}

fn main() {
    let _ = divide(22.0, 7.0);
    let _ = divide(10.0, 0.0);
    let _ = divide(100.0, -12.43);
}
