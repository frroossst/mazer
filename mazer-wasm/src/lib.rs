use mazer_html::document::{Document, Metadata, DocOutputType};
use mazer_lisp::{environment::EnvironmentExt, interpreter::Interpreter};
use mazer_parser::Parser;
use mazer_types::Environment;


use wasm_bindgen::prelude::*;



/// The input is a string that we will parse, and return rendered HTML
/// Upto the caller to actually set the returned content as innerHTML or other
/// appropriate property
#[wasm_bindgen]
pub fn run_mazer(content: &str, window_name: &str) -> String {
    let p = Parser::new(content);
    let r = p.parse().expect("failed to parse");
    let mut d = Document::new(r).dockind(DocOutputType::InnerHtml);
    d.meta(Metadata {
        source: window_name,
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



