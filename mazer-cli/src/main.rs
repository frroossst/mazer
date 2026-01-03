use std::env;

use mazer_html::document::Document;
use mazer_lisp::environment::Environment;
use mazer_parser::Parser;

struct Args<'a> {
    verbose: bool,
    filename: Option<&'a str>,
    serve: bool,
    help: bool,
}

impl Default for Args<'_> {
    fn default() -> Self {
        Args {
            verbose: false,
            filename: None,
            serve: false,
            help: true,
        }
    }
}

fn main() {
    let mut args = env::args();
    let _program = args.next();

    let file = args.next();
    // read file and process
    let content = std::fs::read_to_string(file.unwrap()).expect("Failed to read file");
    let p = Parser::new(&content);
    let r = p.parse().expect("failed to parse");

    dbg!(&r);

    let mut d = Document::new(r);
    d.build();

    let ctx = Environment::with_stdlib();
    let frg = d.lisp_fragments();

    dbg!(frg);

    let o = d.output();

    dbg!(&o);

    // write to /tmp/output.html
    std::fs::write("/tmp/output.html", o).expect("Failed to write output");

}
