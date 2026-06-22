use mazer_html::document::{Document, Metadata, DocOutputType};
use mazer_lisp::{environment::EnvironmentExt, interpreter::Interpreter};
use mazer_parser::Parser;
use mazer_types::Environment;


use wasm_bindgen::prelude::*;


/// Minimal HTML escaping for embedding untrusted text (e.g. error messages)
/// inside a `<pre>` block without breaking the surrounding markup.
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// The input is a string that we will parse, and return rendered HTML
/// Upto the caller to actually set the returned content as innerHTML or other
/// appropriate property.
///
/// This function is **total**: parse or evaluation failures return an error
/// snippet string rather than panicking. This matters in wasm, where a Rust
/// panic aborts the module instance and would poison every subsequent call on
/// the page — so one malformed `.zr` block must not break the others.
#[wasm_bindgen]
pub fn run_mazer(content: &str, window_name: &str) -> String {
    // An empty/whitespace-only note renders to nothing rather than an error.
    if content.trim().is_empty() {
        return String::new();
    }

    let p = Parser::new(content);
    let r = match p.parse() {
        Ok(r) => r,
        Err(e) => {
            return format!(
                "<pre class=\"mazer-error\">mazer parse error: {}</pre>",
                escape_html(&e.to_string())
            );
        }
    };

    let mut d = Document::new(r).dockind(DocOutputType::InnerHtml);
    d.meta(Metadata {
        source: window_name,
        version: env!("CARGO_PKG_VERSION"),
    });
    d.build();

    let ctx = Environment::new().with_native().with_prelude();
    let frg = d.fragments();
    let mut interp = Interpreter::new(frg, ctx);
    if let Err(e) = interp.run() {
        return format!(
            "<pre class=\"mazer-error\">mazer eval error: {}</pre>",
            escape_html(&e.to_string())
        );
    }
    let rst = interp.results();
    d.inject(rst);
    d.fmt(interp.env());

    d.output()
}
