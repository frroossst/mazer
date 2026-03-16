use std::env;
use std::sync::LazyLock;

use mazer_atog::Atog;
use mazer_html::document::{DocOutputType, Document, Metadata};
use mazer_lisp::{environment::EnvironmentExt, interpreter::Interpreter};
use mazer_parser::Parser;
use mazer_types::Environment;
use mazer_types::implfuncs::ShowFunc;

#[derive(Debug, Default)]
struct Args {
    filename: Option<String>,
    open: bool,
    verbose: bool,
    help: bool,
    help_topic: Option<String>,
    doc_query: Option<String>,
}

// Global singleton for parsed arguments - initialized once on first access
static PARSED_ARGS: LazyLock<Args> = LazyLock::new(parse);

fn parse() -> Args {
    let mut args = env::args();
    args.next(); // program name

    let mut result = Args::default();
    let mut seen_file = false;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--query" | "-q" => {
                let query = args.collect::<Vec<_>>().join(" ");
                result.doc_query = Some(query);
                return result;
            }
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
    println!();
    println!("Options:");
    println!("  --open, -o             Open the output in the default web browser");
    println!("  --verbose, -v          Enable verbose logging");
    println!("  --query, -q <search>   Search functions and symbols (e.g. `mazer -q real`)");
    println!("  --help, -h             Show this help message");
}

/// Search functions and symbols, printing matches to stdout.
fn run_doc_search(query: &str) {
    let query_lower = query.to_lowercase();
    // Collect all results into a unified list sorted by name
    enum DocResult {
        Func { name: String, doc: String, aliases: Vec<String> },
        Symbol { name: String, symbol: String, doc: String },
    }

    let mut results: Vec<DocResult> = Vec::new();

    // Gather matching functions
    let functions = ShowFunc::all_functions();
    for f in &functions {
        let dominated = query.is_empty()
            || f.names.iter().any(|n| n.to_lowercase().contains(&query_lower))
            || f.doc.to_lowercase().contains(&query_lower)
            || f.variant_name.to_lowercase().contains(&query_lower);
        if dominated {
            results.push(DocResult::Func {
                name: f.canonical_name().to_string(),
                doc: f.doc.to_string(),
                aliases: f.names.iter().skip(1).map(|s| s.to_string()).collect(),
            });
        }
    }

    // Gather matching symbols
    for (name, entry) in Atog::iter() {
        let matches = query.is_empty()
            || name.to_lowercase().contains(&query_lower)
            || entry.doc.to_lowercase().contains(&query_lower)
            || entry.symbol.contains(&query_lower);
        if matches {
            results.push(DocResult::Symbol {
                name: name.to_string(),
                symbol: entry.symbol.to_string(),
                doc: entry.doc.to_string(),
            });
        }
    }

    results.sort_by(|a, b| {
        let name_a = match a {
            DocResult::Func { name, .. } | DocResult::Symbol { name, .. } => name,
        };
        let name_b = match b {
            DocResult::Func { name, .. } | DocResult::Symbol { name, .. } => name,
        };
        name_a.to_lowercase().cmp(&name_b.to_lowercase())
    });

    if results.is_empty() {
        eprintln!("No matches for '{query}'");
        std::process::exit(1);
    }

    for result in &results {
        match result {
            DocResult::Func { name, doc, aliases } => {
                println!("{}", name);
                println!("  {}", doc);
                if !aliases.is_empty() {
                    println!("  Aliases: {}", aliases.join(", "));
                }
            }
            DocResult::Symbol { name, symbol, doc } => {
                println!("{} → {}", name, symbol);
                println!("  {}", doc);
            }
        }
        println!();
    }
}

fn main() {
    // Access the global parsed args (initialized on first access)
    let args = &*PARSED_ARGS;

    // Handle `doc` subcommand
    if let Some(ref query) = args.doc_query {
        run_doc_search(query);
        return;
    }

    let file_name = args.filename.as_deref().map(|s| s).unwrap_or_else(|| {
        eprintln!("No input file specified.");
        print_help_message();
        std::process::exit(1);
    });

    let content = std::fs::read_to_string(file_name).expect("Failed to read file");

    let o = compile(&content, file_name);

    // write to /tmp/output.html
    std::fs::write("/tmp/output.html", o).expect("Failed to write output");
}

fn compile(content: &str, file_name: &str) -> String {
    let p = Parser::new(content);
    let r = p.parse().expect("failed to parse");
    let mut d = Document::new(r).dockind(DocOutputType::FullBody);
    d.meta(Metadata {
        source: file_name,
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

    d.output()
}

