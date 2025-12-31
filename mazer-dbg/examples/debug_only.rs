#![allow(unused_variables)]

use mazer_dbg::{inspect, inspect_when};

fn main() {

    // Try running 
    // cargo run --example debug_only 
    // and
    // cargo run --release --example debug_only 

    let x = 1;
    inspect!(x);

    let s = "you can't see me";
    // this will never show as whenever debug assertions are false 
    // i.e. release mode then the inspect_when! macro will not be compiled
    inspect_when!(!cfg!(debug_assertions), s);


    println!("I finished my inspection!")
}