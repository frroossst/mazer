use std::{env, sync::Arc};

use mazer_html::document::{Document, Metadata};
use mazer_lisp::{interpreter::Interpreter, environment::EnvironmentExt};
use mazer_types::Environment;
use mazer_parser::Parser;

#[derive(Default)]
struct Args {
    filename: Option<String>,
    serve: bool,
    open: bool,
    verbose: bool,
    help: bool,
    help_topic: Option<String>,
}

fn parse() -> Args {
    let mut args = env::args();
    args.next(); // program name

    let mut result = Args::default();
    let mut seen_file = false;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--serve" | "-s" => result.serve = true,
            "--open" | "-o" => result.open = true,
            "--verbose" | "-v" => result.verbose = true,
            "--help" | "-h" => {
                result.help = true;
                result.help_topic = args.next();
                break;
            }
            val if val.starts_with('-') => {
                eprintln!("Unknown flag: {val}");
            }
            val => {
                if !seen_file {
                    result.filename = Some(val.to_string());
                    seen_file = true;
                } else if result.help && result.help_topic.is_none() {
                    result.help_topic = Some(val.to_string());
                } else {
                    eprintln!("Ignoring extra positional argument: {val}");
                }
            }
        }
    }

    result
}

fn print_help_message() {
    println!("Usage: mazer-cli <input-file> [options]");
    println!("Options:");
    println!("  --serve, -s       Serve the output via a local web server");
    println!("  --open, -o        Open the output in the default web browser");
    println!("  --verbose, -v     Enable verbose logging");
    println!("  --help, -h        Show this help message");
}

fn main() {
    let args = parse();
    let file_name = args.filename.unwrap_or_else(|| {
        eprintln!("No input file specified.");
        print_help_message();
        std::process::exit(1);
    });

    let content = std::fs::read_to_string(&file_name).expect("Failed to read file");

    let p = Parser::new(&content);
    let r = p.parse().expect("failed to parse");
    let mut d = Document::new(r);
    d.meta(Metadata { 
        source: &file_name,
        version: env!("CARGO_PKG_VERSION"),
    });
    d.build();

    let ctx = Environment::new().with_native().with_prelude();
    let frg = d.fragments();
    let mut interp = Interpreter::new(frg, ctx);
    interp.run().expect("inter no pret");
    let rst = interp.results();
    d.inject(rst);
    d.fmt(interp.env());
    
    let o = d.output();

    // write to /tmp/output.html
    std::fs::write("/tmp/output.html", o).expect("Failed to write output");

}
