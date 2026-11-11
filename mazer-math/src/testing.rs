//! Helpers shared by this crate's tests.

use mazer_syntax::{Diags, Node};

/// Reads `src` as one form. Every test that turns source text into a [`Node`]
/// goes through here, so swapping in the real reader is a one-line change.
pub fn read(src: &str) -> Node {
    let mut diags = Diags::new();
    let node = mazer_syntax::reader::read_str(src, &mut diags);
    assert!(diags.is_empty(), "reading {src:?} gave diagnostics: {diags:?}");
    node
}

/// Renders `src`, returning the output and the diagnostics.
pub fn render(src: &str) -> (String, Diags) {
    let node = read(src);
    let mut diags = Diags::new();
    let out = crate::to_mathml(&node, &mut diags);
    (out, diags)
}

/// Renders `src` and asserts it produced no diagnostics.
pub fn mathml(src: &str) -> String {
    let (out, diags) = render(src);
    assert!(diags.is_empty(), "{src:?} gave diagnostics: {diags:?}");
    out
}

/// How many elements sit at the top level of `xml`. Text outside any element
/// counts too, so a well-formed single expression gives exactly 1.
pub fn top_level_elements(xml: &str) -> usize {
    let mut depth = 0usize;
    let mut count = 0;
    let mut rest = xml;
    while let Some(start) = rest.find('<') {
        if depth == 0 && !rest[..start].trim().is_empty() {
            count += 1;
        }
        let end = start + rest[start..].find('>').expect("unclosed tag");
        let tag = &rest[start..=end];
        if tag.starts_with("</") {
            depth -= 1;
        } else {
            if depth == 0 {
                count += 1;
            }
            if !tag.ends_with("/>") {
                depth += 1;
            }
        }
        rest = &rest[end + 1..];
    }
    if !rest.trim().is_empty() {
        count += 1;
    }
    assert_eq!(depth, 0, "unbalanced tags in {xml}");
    count
}
