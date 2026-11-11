//! Helpers shared by the unit tests.

use mazer_syntax::{Diag, DiagCode, Diags, Document, FileId, Node, markdown, reader};

use crate::pipeline::{Analysis, Options, analyze, compile};
use crate::walk;

/// Parses a note as file 0.
pub fn parse(src: &str) -> (Document, Diags) {
    let mut diags = Diags::new();
    let doc = markdown::parse(src, FileId(0), &mut diags);
    (doc, diags)
}

/// Reads one expression, asserting it reads cleanly.
pub fn read(src: &str) -> Node {
    let mut diags = Diags::new();
    let node = reader::read_str(src, &mut diags);
    assert!(diags.is_empty(), "reading {src:?}: {diags:?}");
    node
}

pub fn codes(diags: &[Diag]) -> Vec<DiagCode> {
    diags.iter().map(|d| d.code).collect()
}

/// Every code form of the resolved tree, printed back as source.
pub fn resolved_forms(analysis: &Analysis) -> Vec<String> {
    let mut out = Vec::new();
    walk::for_each_code(&analysis.resolved, &mut |form, _| out.push(form.to_string()));
    out
}

/// The analysis of `src`, asserting it has no diagnostics.
pub fn clean(src: &str) -> Analysis {
    let analysis = analyze(src, "test.zr");
    assert!(analysis.diags.is_empty(), "unexpected diagnostics for {src:?}: {:?}", analysis.diags);
    analysis
}

/// The HTML body of a note compiled as a fragment, without the wrapper.
pub fn body(src: &str) -> String {
    let out = compile(src, &mut Options::new("t.zr"));
    unwrap_fragment(&out.html).to_owned()
}

/// The part of a fragment inside `<div id="mazer-wasm-container">`.
pub fn unwrap_fragment(html: &str) -> &str {
    let open = "<div id=\"mazer-wasm-container\">";
    let start = html.find(open).map_or(0, |i| i + open.len());
    let inner = &html[start..];
    inner.strip_suffix("</div>").unwrap_or(inner)
}
