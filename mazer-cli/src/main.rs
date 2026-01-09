use std::env;

use mazer_html::document::Document;
use mazer_lisp::{interpreter::Interpreter, environment::EnvironmentExt};
use mazer_types::Environment;
use mazer_parser::Parser;

#[derive(Default)]
struct _Args<'a> {
    verbose: bool,
    filename: Option<&'a str>,
    serve: bool,
    help: bool,
}

fn main() {
    let mut args = env::args();
    let _program = args.next();

    let file = args.next();
    // read file and process
    let content = std::fs::read_to_string(file.unwrap()).expect("Failed to read file");



    let p = Parser::new(&content);
    let r = p.parse().expect("failed to parse");
    let mut d = Document::new(r);
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
