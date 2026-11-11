//! Helpers shared by this crate's tests.

use mazer_syntax::{Diags, Node};

/// Reads `src` as one form. Every test that turns source text into a [`Node`]
/// goes through here, so swapping in the real reader is a one-line change.
pub fn read(src: &str) -> Node {
    let mut diags = Diags::new();
    // TODO(after step 1 merge): replace with mazer_syntax::reader::read_str
    let node = mini_reader::read_str(src, &mut diags);
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

/// A deliberately tiny S-expression reader, only good enough for tests: parens,
/// names, numbers, `"strings"` (escapes `\"` and `\\`), `'x` -> `(quote x)`
/// and `-name` -> `(- name)`. It panics on malformed input instead of
/// recovering. Spans are byte offsets with `FileId(0)`, as in the real reader.
// TODO(after step 1 merge): delete this module once `read` uses mazer_syntax::reader::read_str.
mod mini_reader {
    use mazer_syntax::{ByteOffset, Diags, FileId, Node, Span};

    pub fn read_str(src: &str, _diags: &mut Diags) -> Node {
        let mut reader = Reader { src, pos: 0 };
        reader.skip_space();
        let node = reader.form();
        reader.skip_space();
        assert_eq!(reader.pos, src.len(), "mini reader: text after the form in {src:?}");
        node
    }

    struct Reader<'a> {
        src: &'a str,
        pos: usize,
    }

    impl Reader<'_> {
        fn peek(&self) -> Option<char> {
            self.src[self.pos..].chars().next()
        }

        fn span(&self, start: usize) -> Span {
            Span::new(FileId(0), ByteOffset::from_usize(start), ByteOffset::from_usize(self.pos))
        }

        fn skip_space(&mut self) {
            while self.peek().is_some_and(char::is_whitespace) {
                self.pos += self.peek().map_or(0, char::len_utf8);
            }
        }

        fn form(&mut self) -> Node {
            let start = self.pos;
            match self.peek().expect("mini reader: unexpected end of input") {
                '(' => {
                    self.pos += 1;
                    let mut items = Vec::new();
                    loop {
                        self.skip_space();
                        match self.peek().expect("mini reader: unclosed paren") {
                            ')' => break,
                            _ => items.push(self.form()),
                        }
                    }
                    self.pos += 1;
                    Node::list(items, self.span(start))
                }
                ')' => panic!("mini reader: stray `)` in {:?}", self.src),
                '\'' => {
                    self.pos += 1;
                    let quote = Node::ident("quote", self.span(start));
                    let quoted = self.form();
                    Node::list(vec![quote, quoted], self.span(start))
                }
                '"' => {
                    self.pos += 1;
                    let mut text = String::new();
                    loop {
                        let ch = self.peek().expect("mini reader: unterminated string");
                        self.pos += ch.len_utf8();
                        match ch {
                            '"' => break,
                            '\\' => {
                                let escaped = self.peek().expect("mini reader: unterminated string");
                                self.pos += escaped.len_utf8();
                                text.push(escaped);
                            }
                            _ => text.push(ch),
                        }
                    }
                    Node::string(&text, self.span(start))
                }
                _ => {
                    let len = self.src[start..]
                        .find(|c: char| c.is_whitespace() || c == '(' || c == ')')
                        .unwrap_or(self.src.len() - start);
                    self.pos = start + len;
                    let atom = &self.src[start..self.pos];
                    let mut chars = atom.chars();
                    let first = chars.next().unwrap_or(' ');
                    let second = chars.next();
                    if first.is_ascii_digit() || (first == '-' && second.is_some_and(|c| c.is_ascii_digit())) {
                        Node::number(atom, self.span(start))
                    } else if first == '-' && second.is_some() {
                        let minus = Span::new(FileId(0), ByteOffset::from_usize(start), ByteOffset::from_usize(start + 1));
                        let name = Span::new(FileId(0), ByteOffset::from_usize(start + 1), ByteOffset::from_usize(self.pos));
                        Node::list(vec![Node::ident("-", minus), Node::ident(&atom[1..], name)], self.span(start))
                    } else {
                        Node::ident(atom, self.span(start))
                    }
                }
            }
        }
    }

    #[test]
    fn reads_the_test_syntax() {
        let node = read_str("(f 'x -y -2 \"a \\\"b\\\"\" (g))", &mut Diags::new());
        assert_eq!(node.to_string(), "(f (quote x) (- y) -2 \"a \\\"b\\\"\" (g))");
        assert_eq!(node.span.range(), 0..26);
        assert_eq!(node.args()[0].span.range(), 3..5);
    }
}
