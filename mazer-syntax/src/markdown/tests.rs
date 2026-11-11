use super::*;
use crate::diag::DiagCode;
use crate::doc::{Block, BlockKind, Inline, InlineKind, ListItem};
use crate::node::{Node, NodeKind};
use crate::span::Span;

fn parse_doc(src: &str) -> (Document, Diags) {
    let mut diags = Diags::new();
    let doc = parse(src, FileId(0), &mut diags);
    (doc, diags)
}

/// Parses and asserts there are no diagnostics.
fn blocks(src: &str) -> Vec<Block> {
    let (doc, diags) = parse_doc(src);
    assert!(diags.is_empty(), "unexpected diagnostics for {src:?}: {diags:?}");
    doc.blocks
}

/// The inlines of a document that is a single paragraph.
fn para(src: &str) -> Vec<Inline> {
    let mut all = blocks(src);
    assert_eq!(all.len(), 1, "expected one block for {src:?}, got {all:?}");
    match all.remove(0).kind {
        BlockKind::Paragraph(inlines) => inlines,
        other => panic!("expected a paragraph for {src:?}, got {other:?}"),
    }
}

fn list(block: &Block) -> (Option<u32>, &[ListItem]) {
    match &block.kind {
        BlockKind::List { ordered, items } => (*ordered, items),
        other => panic!("expected a list, got {other:?}"),
    }
}

/// The inline content of a list item made of one paragraph.
fn item_inlines(item: &ListItem) -> &[Inline] {
    match item.content.as_slice() {
        [Block {
            kind: BlockKind::Paragraph(inlines),
            ..
        }] => inlines,
        other => panic!("expected one paragraph in the item, got {other:?}"),
    }
}

/// Plain text of inlines (styles flattened, code shown as source).
fn flat(inlines: &[Inline]) -> String {
    inlines
        .iter()
        .map(|inline| match &inline.kind {
            InlineKind::Text(t) | InlineKind::InlineCode(t) => t.to_string(),
            InlineKind::Bold(c)
            | InlineKind::Italic(c)
            | InlineKind::Underline(c)
            | InlineKind::Strike(c)
            | InlineKind::Spoiler(c)
            | InlineKind::Link { content: c, .. } => flat(c),
            InlineKind::Image { alt, .. } => alt.to_string(),
            InlineKind::Emoji(sym) => format!(":{sym}:"),
            InlineKind::Code(node) => node.to_string(),
            InlineKind::LineBreak => "\n".into(),
        })
        .collect()
}

fn text(s: &str) -> InlineKind {
    InlineKind::Text(s.into())
}

/// Inline kinds with spans and nested spans dropped, for comparisons.
fn shape(inlines: &[Inline]) -> Vec<InlineKind> {
    inlines.iter().map(|i| strip(&i.kind)).collect()
}

fn strip(kind: &InlineKind) -> InlineKind {
    let all = |c: &[Inline]| c.iter().map(|i| Inline::new(strip(&i.kind), Span::DUMMY)).collect();
    match kind {
        InlineKind::Bold(c) => InlineKind::Bold(all(c)),
        InlineKind::Italic(c) => InlineKind::Italic(all(c)),
        InlineKind::Underline(c) => InlineKind::Underline(all(c)),
        InlineKind::Strike(c) => InlineKind::Strike(all(c)),
        InlineKind::Spoiler(c) => InlineKind::Spoiler(all(c)),
        InlineKind::Link { content, url } => InlineKind::Link {
            content: all(content),
            url: url.clone(),
        },
        other => other.clone(),
    }
}

fn dummy(kind: InlineKind) -> Inline {
    Inline::new(kind, Span::DUMMY)
}

fn code_of(src: &str) -> Node {
    let (doc, diags) = parse_doc(src);
    assert!(diags.is_empty(), "{diags:?}");
    match doc.blocks.as_slice() {
        [Block {
            kind: BlockKind::Code(node),
            ..
        }] => node.clone(),
        other => panic!("expected one code block for {src:?}, got {other:?}"),
    }
}

/// Every code form in the document, in order, with whether it is a block.
fn all_code(blocks: &[Block]) -> Vec<(bool, Node)> {
    fn inl(inlines: &[Inline], out: &mut Vec<(bool, Node)>) {
        for i in inlines {
            match &i.kind {
                InlineKind::Code(n) => out.push((false, n.clone())),
                InlineKind::Bold(c)
                | InlineKind::Italic(c)
                | InlineKind::Underline(c)
                | InlineKind::Strike(c)
                | InlineKind::Spoiler(c)
                | InlineKind::Link { content: c, .. } => inl(c, out),
                _ => {}
            }
        }
    }
    fn blk(blocks: &[Block], out: &mut Vec<(bool, Node)>) {
        for b in blocks {
            match &b.kind {
                BlockKind::Code(n) => out.push((true, n.clone())),
                BlockKind::Heading { content, .. } | BlockKind::Paragraph(content) => inl(content, out),
                BlockKind::List { items, .. } => items.iter().for_each(|it| blk(&it.content, out)),
                BlockKind::BlockQuote(inner) => blk(inner, out),
                BlockKind::CodeBlock { .. } | BlockKind::Image { .. } | BlockKind::Rule => {}
            }
        }
    }
    let mut out = Vec::new();
    blk(blocks, &mut out);
    out
}

// ---------------------------------------------------------------------------
// The old parser's 26 tests, ported to the new tree.
// ---------------------------------------------------------------------------

#[test]
fn test_headers() {
    let doc = blocks("# Header 1\n## Header 2\n### Header 3");
    assert_eq!(doc.len(), 3);
    for (n, block) in doc.iter().enumerate() {
        let BlockKind::Heading { level, content } = &block.kind else {
            panic!("expected heading, got {block:?}");
        };
        assert_eq!(usize::from(*level), n + 1);
        assert_eq!(flat(content), format!("Header {}", n + 1));
    }
}

#[test]
fn test_bullet_points() {
    let doc = blocks("- Item 1\n- Item 2");
    assert_eq!(doc.len(), 1);
    let (ordered, items) = list(&doc[0]);
    assert_eq!(ordered, None);
    assert_eq!(items.len(), 2);
    assert_eq!(shape(item_inlines(&items[0])), vec![text("Item 1")]);
    assert_eq!(shape(item_inlines(&items[1])), vec![text("Item 2")]);
    assert!(items.iter().all(|it| it.checked.is_none()));
}

#[test]
fn test_bullet_point_preserves_inline_code() {
    let doc = blocks("- `a*b` is the product");
    let (_, items) = list(&doc[0]);
    assert_eq!(items.len(), 1);
    assert_eq!(
        shape(item_inlines(&items[0])),
        vec![InlineKind::InlineCode("a*b".into()), text(" is the product")]
    );
}

#[test]
fn test_checkboxes() {
    // Checklist items are list items now; consecutive ones form one list.
    let doc = blocks("-[ ] Unchecked\n-[x] Checked");
    assert_eq!(doc.len(), 1);
    let (ordered, items) = list(&doc[0]);
    assert_eq!(ordered, None);
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].checked, Some(false));
    assert_eq!(items[1].checked, Some(true));
    assert_eq!(flat(item_inlines(&items[0])), "Unchecked");
    assert_eq!(flat(item_inlines(&items[1])), "Checked");
}

#[test]
fn test_spoiler() {
    let inlines = para("This is ||spoiled content|| here");
    assert_eq!(
        shape(&inlines),
        vec![
            text("This is "),
            InlineKind::Spoiler(vec![dummy(text("spoiled content"))]),
            text(" here"),
        ]
    );
}

#[test]
fn test_link() {
    let inlines = para("[Click here](https://example.com)");
    assert_eq!(
        shape(&inlines),
        vec![InlineKind::Link {
            content: vec![dummy(text("Click here"))],
            url: "https://example.com".into(),
        }]
    );
}

#[test]
fn test_code_block() {
    let doc = blocks("```rust\nfn main() {}\n```");
    assert_eq!(
        doc[0].kind,
        BlockKind::CodeBlock {
            lang: Some("rust".into()),
            code: "fn main() {}\n".into(),
        }
    );
}

#[test]
fn test_code_block_no_language() {
    let doc = blocks("```\nsome code\n```");
    assert_eq!(
        doc[0].kind,
        BlockKind::CodeBlock {
            lang: None,
            code: "some code\n".into(),
        }
    );
}

#[test]
fn test_eval_inside_code_block_not_parsed() {
    let doc = blocks("```scheme\n(eval (print \"hello\"))\n```");
    assert_eq!(doc.len(), 1);
    let BlockKind::CodeBlock { code, .. } = &doc[0].kind else {
        panic!("expected a code block, got {doc:?}");
    };
    assert!(code.contains("(eval (print \"hello\"))"));
    assert_eq!(all_code(&doc), vec![]);
}

#[test]
fn test_inline_code() {
    let inlines = para("This is `inline code` here");
    assert_eq!(
        shape(&inlines),
        vec![text("This is "), InlineKind::InlineCode("inline code".into()), text(" here")]
    );
}

#[test]
fn test_inline_code_preserves_special_chars() {
    let cases = [
        ("`*`", "*"),
        ("`a*b`", "a*b"),
        ("`a*b = b*a`", "a*b = b*a"),
        ("`x_i`", "x_i"),
        ("`f(x)`", "f(x)"),
        ("`a[0]`", "a[0]"),
        ("`(show x)`", "(show x)"),
        ("`:sob:`", ":sob:"),
    ];
    for (src, expected) in cases {
        assert_eq!(shape(&para(src)), vec![InlineKind::InlineCode(expected.into())], "{src}");
    }
}

#[test]
fn test_page_separator() {
    assert_eq!(blocks("---")[0].kind, BlockKind::Rule);
}

#[test]
fn test_eval_scheme() {
    let inlines = para("Result: (eval (+ 1 1))");
    assert_eq!(inlines.len(), 2);
    assert_eq!(inlines[0].kind, text("Result: "));
    let InlineKind::Code(node) = &inlines[1].kind else {
        panic!("expected inline code, got {inlines:?}");
    };
    assert_eq!(node.to_string(), "(eval (+ 1 1))");
}

#[test]
fn test_show_scheme() {
    assert_eq!(code_of("(show (matrix (1 1 1)))").to_string(), "(show (matrix (1 1 1)))");
}

#[test]
fn test_normal_parentheses() {
    assert_eq!(shape(&para("This is a (Sentence).")), vec![text("This is a (Sentence).")]);
}

#[test]
fn test_nested_scheme() {
    let node = code_of("(eval (+ 1 1))");
    assert!(node.is_form("eval"));
    assert_eq!(node.args()[0].to_string(), "(+ 1 1)");
}

#[test]
fn test_emoji_in_text() {
    assert_eq!(shape(&para("Hello 👋 World 🌍")), vec![text("Hello 👋 World 🌍")]);
}

#[test]
fn test_emoji_in_header() {
    let doc = blocks("# Header with emoji 🚀");
    assert_eq!(doc.len(), 1);
    let BlockKind::Heading { level: 1, content } = &doc[0].kind else {
        panic!("expected a level-1 heading, got {doc:?}");
    };
    assert_eq!(shape(content), vec![text("Header with emoji 🚀")]);
}

#[test]
fn test_emoji_in_bullet_point() {
    let doc = blocks("- Item with emoji 📝");
    assert_eq!(doc.len(), 1);
    let (_, items) = list(&doc[0]);
    assert_eq!(items.len(), 1);
    assert!(flat(item_inlines(&items[0])).contains("📝"));
}

#[test]
fn test_emoji_in_bold() {
    assert_eq!(
        shape(&para("**Bold text 💪**")),
        vec![InlineKind::Bold(vec![dummy(text("Bold text 💪"))])]
    );
}

#[test]
fn test_multibyte_utf8_characters() {
    let src = "こんにちは世界 مرحبا بالعالم Привет мир";
    assert_eq!(shape(&para(src)), vec![text(src)]);
}

#[test]
fn test_emoji_with_skin_tone_modifier() {
    assert_eq!(shape(&para("👍🏽 thumbs up")), vec![text("👍🏽 thumbs up")]);
}

#[test]
fn test_complex_emoji_sequences() {
    assert_eq!(shape(&para("Family: 👨‍👩‍👧‍👦")), vec![text("Family: 👨‍👩‍👧‍👦")]);
}

#[test]
fn test_mixed_emoji_and_markdown() {
    let doc = blocks("# 🎉 Celebration 🎊\n- First item 🥇\n- Second item 🥈");
    assert_eq!(doc.len(), 2);
    let BlockKind::Heading { content, .. } = &doc[0].kind else {
        panic!("expected a heading");
    };
    assert_eq!(flat(content), "🎉 Celebration 🎊");
    let (_, items) = list(&doc[1]);
    assert_eq!(items.len(), 2);
    assert_eq!(flat(item_inlines(&items[1])), "Second item 🥈");
}

#[test]
fn test_scheme_in_codeblock() {
    let src = "```scheme\n(define (factorial n)\n  (if (= n 0)\n      1\n      (* n (factorial (- n 1)))))\n```\n";
    let doc = blocks(src);
    assert_eq!(doc.len(), 1);
    let BlockKind::CodeBlock { lang, code } = &doc[0].kind else {
        panic!("expected a code block, got {doc:?}");
    };
    assert_eq!(lang.as_deref(), Some("scheme"));
    assert!(code.contains("factorial"));
}

#[test]
fn inline_eval_and_fenced_code_are_distinguished() {
    // Only the inline `(eval (+ 1 1))` is code; the forms inside the
    // ```scheme fence are plain text.
    const SRC: &str = "\
# Foo

look ma! live reload

(show (string f((pow Delta 2))))

```scheme
(define (factorial n)
  (if (= n 0)
      1
      (* n (factorial (- n 1)))))

(eval
    (print (string 42))
)

(show (pow x 2))
```

I like math such that (show (+ 1 1)) equals to (eval (+ 1 1)).

(show
    (ellipsis)
)

- Item A

(show
    (integral
        (exp (- (pow x 2)))
        x
    )
)
";
    let doc = blocks(SRC);
    let found = all_code(&doc);
    let evals: Vec<&(bool, Node)> = found.iter().filter(|(_, n)| n.is_form("eval")).collect();
    assert_eq!(evals.len(), 1, "{evals:?}");
    assert!(!evals[0].0, "the eval is inside a sentence");
    assert_eq!(evals[0].1.to_string(), "(eval (+ 1 1))");
    assert!(found.iter().all(|(_, n)| !n.is_form("define")), "{found:?}");
    // Every `(show` outside fences became code (one of the five is fenced).
    let expected = SRC.matches("(show").count() - 1;
    assert_eq!(found.iter().filter(|(_, n)| n.is_form("show")).count(), expected);
    let standalone: Vec<String> = found.iter().filter(|(block, _)| *block).map(|(_, n)| n.to_string()).collect();
    assert_eq!(
        standalone,
        vec![
            "(show (string f ((pow Delta 2))))",
            "(show (ellipsis))",
            "(show (integral (exp (- (pow x 2))) x))",
        ]
    );
}

#[test]
fn fence_show_eval_block_sequence() {
    const SRC: &str = "\
```scheme
(show (add 1 2 3))
```
(show (add 1 2 3))

```scheme
(show (add 11 12))
```
(show (add 11 12))

(eval
    (define a 5)
    (define b 10)
    (print (add a b))
)

(show (add a b))
";
    let doc = blocks(SRC);
    let kinds: Vec<&str> = doc
        .iter()
        .map(|b| match &b.kind {
            BlockKind::CodeBlock { .. } => "fence",
            BlockKind::Code(n) if n.is_form("show") => "show",
            BlockKind::Code(n) if n.is_form("eval") => "eval",
            other => panic!("unexpected block {other:?}"),
        })
        .collect();
    assert_eq!(kinds, vec!["fence", "show", "fence", "show", "eval", "show"]);
    let BlockKind::Code(eval) = &doc[4].kind else {
        panic!()
    };
    assert_eq!(eval.args().len(), 3);
}

/// Byte offsets of every unescaped `(show` outside fenced code and inline
/// code, found with a simple scan independent of the parser.
fn show_offsets(text: &str) -> Vec<usize> {
    let mut found = Vec::new();
    let mut in_fence = false;
    let mut line_start = 0;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start_matches([' ', '\t', '>']).trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
        } else if !in_fence {
            for (at, _) in line.match_indices("(show") {
                let after = line[at + 5..].chars().next();
                let is_form = after.is_some_and(|c| c.is_ascii_whitespace() || "()".contains(c));
                let in_inline_code = line[..at].matches('`').count() % 2 == 1;
                let escaped = line[..at].ends_with('\\');
                if is_form && !in_inline_code && !escaped {
                    found.push(line_start + at);
                }
            }
        }
        line_start += line.len();
    }
    found
}

/// Loose check on the live examples: their contents are free to change, but
/// they must parse cleanly, have headings, and turn every `(show` into code.
#[test]
fn live_examples_parse_cleanly() {
    for (name, src) in [
        ("markdown.md", include_str!("../../../examples/markdown.md")),
        ("docs.md", include_str!("../../../examples/docs.md")),
    ] {
        let (doc, diags) = parse_doc(src);
        assert!(diags.is_empty(), "{name}: {diags:?}");
        assert!(
            doc.blocks.iter().any(|b| matches!(b.kind, BlockKind::Heading { .. })),
            "{name}: no heading"
        );
        let spans: Vec<Span> = all_code(&doc.blocks).iter().map(|(_, n)| n.span).collect();
        for at in show_offsets(src) {
            assert!(
                spans.iter().any(|s| s.start.to_usize() <= at && at < s.end.to_usize()),
                "{name}: `(show` at byte {at} did not become code"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Blocks
// ---------------------------------------------------------------------------

#[test]
fn heading_levels_and_rules() {
    let doc = blocks("###### six\n####### seven\n#nospace\n# closed ##\n#\n***\n- - -\n___");
    let BlockKind::Heading { level: 6, .. } = doc[0].kind else {
        panic!("{doc:?}")
    };
    // `####### seven` and `#nospace` are paragraph text.
    let BlockKind::Paragraph(p) = &doc[1].kind else {
        panic!("{doc:?}")
    };
    assert_eq!(flat(p), "####### seven\n#nospace");
    let BlockKind::Heading { level: 1, content } = &doc[2].kind else {
        panic!("{doc:?}")
    };
    assert_eq!(flat(content), "closed");
    let BlockKind::Heading { level: 1, content } = &doc[3].kind else {
        panic!("{doc:?}")
    };
    assert_eq!(content, &vec![]);
    assert!(doc[4..].iter().all(|b| b.kind == BlockKind::Rule));
    assert_eq!(doc.len(), 7);
}

#[test]
fn headings_hold_rich_content() {
    let doc = blocks("## The **big** (show (frac 1 2)) [idea](u) :fire:");
    let BlockKind::Heading { level: 2, content } = &doc[0].kind else {
        panic!("{doc:?}")
    };
    let kinds = shape(content);
    assert_eq!(kinds[1], InlineKind::Bold(vec![dummy(text("big"))]));
    assert!(matches!(&kinds[3], InlineKind::Code(n) if n.is_form("show")));
    assert!(matches!(&kinds[5], InlineKind::Link { .. }));
    assert_eq!(kinds[7], InlineKind::Emoji(Sym::new("fire")));
}

#[test]
fn paragraphs_split_on_blank_lines() {
    let doc = blocks("one\ntwo\n\n  three  \n");
    assert_eq!(doc.len(), 2);
    let BlockKind::Paragraph(first) = &doc[0].kind else {
        panic!()
    };
    // A soft break stays a newline inside the text.
    assert_eq!(shape(first), vec![text("one\ntwo")]);
    let BlockKind::Paragraph(second) = &doc[1].kind else {
        panic!()
    };
    assert_eq!(shape(second), vec![text("three")]);
}

#[test]
fn escaped_backslash_at_end_of_line_is_not_a_break() {
    // Two backslashes: a literal backslash, then a soft break.
    assert_eq!(shape(&para("foo\\\\\nbar")), vec![text("foo\\\nbar")]);
    // Three: a literal backslash, then a hard break.
    assert_eq!(
        shape(&para("foo\\\\\\\nbar")),
        vec![text("foo\\"), InlineKind::LineBreak, text("bar")]
    );
}

#[test]
fn hard_line_breaks() {
    let inlines = para("two spaces  \nbackslash\\\none space \nend  ");
    assert_eq!(
        shape(&inlines),
        vec![
            text("two spaces"),
            InlineKind::LineBreak,
            text("backslash"),
            InlineKind::LineBreak,
            text("one space\nend"),
        ]
    );
}

#[test]
fn ordered_lists_keep_their_start() {
    let doc = blocks("3. three\n4. four\n10) ten");
    let (ordered, items) = list(&doc[0]);
    assert_eq!(ordered, Some(3));
    assert_eq!(items.len(), 3);
    assert_eq!(flat(item_inlines(&items[2])), "ten");
    let doc = blocks("1. a\n\n2. b");
    let (ordered, items) = list(&doc[0]);
    assert_eq!((ordered, items.len()), (Some(1), 2));
}

#[test]
fn bullets_and_numbers_are_different_lists() {
    let doc = blocks("- a\n1. b\n* c\n+ d");
    assert_eq!(doc.len(), 3);
    assert_eq!(list(&doc[0]).0, None);
    assert_eq!(list(&doc[1]).0, Some(1));
    assert_eq!(list(&doc[2]).1.len(), 2);
}

#[test]
fn nested_lists_by_indentation() {
    let src = "- a\n  - b\n    - c\n  - d\n- e\n\n1. one\n   - inner\n2. two";
    let doc = blocks(src);
    assert_eq!(doc.len(), 2);
    let (_, items) = list(&doc[0]);
    assert_eq!(items.len(), 2);
    let a = &items[0].content;
    assert_eq!(a.len(), 2);
    let (_, inner) = list(&a[1]);
    assert_eq!(inner.len(), 2);
    let (_, innermost) = list(&inner[0].content[1]);
    assert_eq!(flat(item_inlines(&innermost[0])), "c");
    assert_eq!(flat(item_inlines(&inner[1])), "d");
    assert_eq!(flat(item_inlines(&items[1])), "e");

    let (ordered, items) = list(&doc[1]);
    assert_eq!(ordered, Some(1));
    assert_eq!(items.len(), 2);
    assert_eq!(list(&items[0].content[1]).1.len(), 1);
    // Item spans start at the marker.
    assert_eq!(&src[items[1].span.range()], "2. two");
}

#[test]
fn shallow_indent_is_a_sibling_not_a_child() {
    // `- ` has content column 2; one space is not enough to nest.
    let doc = blocks("- a\n - b");
    let (_, items) = list(&doc[0]);
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].content.len(), 1);
}

#[test]
fn list_items_hold_several_blocks() {
    let doc = blocks("- first\n\n  second paragraph\n\n  ```\n  code\n  ```\n- next");
    let (_, items) = list(&doc[0]);
    assert_eq!(items.len(), 2);
    let content = &items[0].content;
    assert_eq!(content.len(), 3);
    assert_eq!(
        content[2].kind,
        BlockKind::CodeBlock {
            lang: None,
            code: "code\n".into()
        }
    );
}

#[test]
fn no_lazy_continuation_after_a_list() {
    let doc = blocks("- a claim\nAn answer to it.");
    assert_eq!(doc.len(), 2);
    assert_eq!(list(&doc[0]).1.len(), 1);
    assert!(matches!(&doc[1].kind, BlockKind::Paragraph(p) if flat(p) == "An answer to it."));
}

#[test]
fn lists_interrupt_paragraphs_but_only_numbered_from_one() {
    let doc = blocks("intro\n- a\n- b");
    assert_eq!(doc.len(), 2);
    let doc = blocks("in the year\n2003. was fine");
    assert_eq!(doc.len(), 1);
    let doc = blocks("steps\n1. first");
    assert_eq!(doc.len(), 2);
}

#[test]
fn checklists_in_both_spellings() {
    let doc = blocks("-[ ] old\n-[X]old two\n- [x] github\n- [ ] github two\n- plain");
    let (_, items) = list(&doc[0]);
    let states: Vec<Option<bool>> = items.iter().map(|i| i.checked).collect();
    assert_eq!(states, vec![Some(false), Some(true), Some(true), Some(false), None]);
    assert_eq!(flat(item_inlines(&items[1])), "old two");
    assert_eq!(flat(item_inlines(&items[2])), "github");
    // `- [link](u)` is not a checkbox.
    let doc = blocks("- [a](u)");
    assert_eq!(list(&doc[0]).1[0].checked, None);
}

#[test]
fn block_quotes_contain_blocks() {
    let doc = blocks("> # Title\n> - a\n> - b\n>\n> text\nafter");
    assert_eq!(doc.len(), 2);
    let BlockKind::BlockQuote(inner) = &doc[0].kind else {
        panic!("{doc:?}")
    };
    assert_eq!(inner.len(), 3);
    assert!(matches!(inner[0].kind, BlockKind::Heading { level: 1, .. }));
    assert_eq!(list(&inner[1]).1.len(), 2);
    assert!(matches!(&inner[2].kind, BlockKind::Paragraph(p) if flat(p) == "text"));
    assert!(matches!(&doc[1].kind, BlockKind::Paragraph(p) if flat(p) == "after"));
}

#[test]
fn separate_quotes_and_nested_quotes() {
    let doc = blocks("> one\n\n> two\n>> deep");
    assert_eq!(doc.len(), 2);
    let BlockKind::BlockQuote(second) = &doc[1].kind else {
        panic!()
    };
    assert!(matches!(&second[1].kind, BlockKind::BlockQuote(_)));
}

#[test]
fn fences_are_verbatim() {
    let src = "~~~\n# not a heading\n**not bold** (show x)\n- no list\n~~~\n```\nunclosed\n\n> still code";
    let doc = blocks(src);
    assert_eq!(doc.len(), 2);
    assert_eq!(
        doc[0].kind,
        BlockKind::CodeBlock {
            lang: None,
            code: "# not a heading\n**not bold** (show x)\n- no list\n".into()
        }
    );
    assert_eq!(
        doc[1].kind,
        BlockKind::CodeBlock {
            lang: None,
            code: "unclosed\n\n> still code\n".into()
        }
    );
    assert_eq!(all_code(&doc), vec![]);
}

#[test]
fn fence_inside_quote_and_indented_fence() {
    let doc = blocks("> ```py\n> x = 1\n> ```\n\n  ```\n  a\n    b\n  ```");
    let BlockKind::BlockQuote(inner) = &doc[0].kind else {
        panic!()
    };
    assert_eq!(
        inner[0].kind,
        BlockKind::CodeBlock {
            lang: Some("py".into()),
            code: "x = 1\n".into()
        }
    );
    assert_eq!(
        doc[1].kind,
        BlockKind::CodeBlock {
            lang: None,
            code: "a\n  b\n".into()
        }
    );
}

#[test]
fn images_alone_on_a_line_are_blocks() {
    let src = "text\n![a cat](cat.png)\n![x](y) and more";
    let doc = blocks(src);
    assert_eq!(doc.len(), 3);
    assert_eq!(
        doc[1].kind,
        BlockKind::Image {
            alt: "a cat".into(),
            url: "cat.png".into()
        }
    );
    assert_eq!(&src[doc[1].span.range()], "![a cat](cat.png)");
    let BlockKind::Paragraph(p) = &doc[2].kind else {
        panic!()
    };
    assert_eq!(
        p[0].kind,
        InlineKind::Image {
            alt: "x".into(),
            url: "y".into()
        }
    );
}

#[test]
fn crlf_line_endings() {
    let doc = blocks("# T\r\n\r\n- a\r\n- b\r\n```\r\nx\r\n```\r\n");
    assert_eq!(doc.len(), 3);
    assert_eq!(list(&doc[1]).1.len(), 2);
    assert_eq!(
        doc[2].kind,
        BlockKind::CodeBlock {
            lang: None,
            code: "x\n".into()
        }
    );
}

// ---------------------------------------------------------------------------
// Inlines
// ---------------------------------------------------------------------------

#[test]
fn the_five_styles() {
    assert_eq!(
        shape(&para("**b** *i* _u_ ~s~ ~~s2~~ ||p||")),
        vec![
            InlineKind::Bold(vec![dummy(text("b"))]),
            text(" "),
            InlineKind::Italic(vec![dummy(text("i"))]),
            text(" "),
            InlineKind::Underline(vec![dummy(text("u"))]),
            text(" "),
            InlineKind::Strike(vec![dummy(text("s"))]),
            text(" "),
            InlineKind::Strike(vec![dummy(text("s2"))]),
            text(" "),
            InlineKind::Spoiler(vec![dummy(text("p"))]),
        ]
    );
}

#[test]
fn styles_nest() {
    let inlines = para("**bold *both* and (show x) and [l](u)**");
    let [Inline {
        kind: InlineKind::Bold(inner),
        ..
    }] = inlines.as_slice()
    else {
        panic!("{inlines:?}")
    };
    let kinds = shape(inner);
    assert_eq!(kinds[0], text("bold "));
    assert_eq!(kinds[1], InlineKind::Italic(vec![dummy(text("both"))]));
    assert!(matches!(&kinds[3], InlineKind::Code(n) if n.to_string() == "(show x)"));
    assert!(matches!(&kinds[5], InlineKind::Link { url, .. } if &**url == "u"));

    assert_eq!(
        shape(&para("***x***")),
        vec![InlineKind::Bold(vec![dummy(InlineKind::Italic(vec![dummy(text("x"))]))])]
    );
    assert_eq!(
        shape(&para("||_a ~b~_||")),
        vec![InlineKind::Spoiler(vec![dummy(InlineKind::Underline(vec![
            dummy(text("a ")),
            dummy(InlineKind::Strike(vec![dummy(text("b"))])),
        ]))])]
    );
    // A link holding styles.
    let inlines = para("[a **b**](u)");
    let InlineKind::Link { content, .. } = &inlines[0].kind else {
        panic!()
    };
    assert_eq!(shape(content)[1], InlineKind::Bold(vec![dummy(text("b"))]));
}

#[test]
fn star_after_a_letter_or_digit_is_literal() {
    assert_eq!(shape(&para("lambda*I and 2*3*4")), vec![text("lambda*I and 2*3*4")]);
    assert_eq!(shape(&para("snake_case_name")), vec![text("snake_case_name")]);
    // A closing star after a letter is still a closer.
    assert_eq!(
        shape(&para("not *yet* done")),
        vec![text("not "), InlineKind::Italic(vec![dummy(text("yet"))]), text(" done")]
    );
}

#[test]
fn double_underscores_are_text() {
    for src in ["__init__", "__a__b", "a__b__c", "x = __", "a ____ b"] {
        assert_eq!(shape(&para(src)), vec![text(src)], "{src}");
    }
    assert_eq!(
        shape(&para("call __init__ and _u_")),
        vec![text("call __init__ and "), InlineKind::Underline(vec![dummy(text("u"))])]
    );
    // A single underline cannot close on a run either.
    assert_eq!(shape(&para("_a__")), vec![text("_a__")]);
    // Doubled `*` and `~` keep their meaning.
    assert_eq!(shape(&para("**init**")), vec![InlineKind::Bold(vec![dummy(text("init"))])]);
    assert_eq!(shape(&para("~~init~~")), vec![InlineKind::Strike(vec![dummy(text("init"))])]);
}

#[test]
fn unmatched_markers_are_text() {
    for src in [
        "a * b * c",
        "**open",
        "x ~5 and ~10",
        "a || b",
        "one _two",
        "a ~~~ three",
        "a ****",
        "`unclosed",
        "[no link]",
        "[bad](has space)",
        "![alt]",
    ] {
        assert_eq!(shape(&para(src)), vec![text(src)], "{src}");
    }
}

#[test]
fn styles_span_lines_but_not_paragraphs() {
    let inlines = para("**one\ntwo**");
    assert_eq!(inlines.len(), 1);
    assert_eq!(flat(&inlines), "one\ntwo");
    let doc = blocks("**one\n\ntwo**");
    assert_eq!(doc.len(), 2);
}

#[test]
fn links_and_urls() {
    let inlines = para("see [Magma](https://en.wikipedia.org/wiki/Magma_(algebra)) and [x](a_b~c*d)");
    let urls: Vec<String> = inlines
        .iter()
        .filter_map(|i| match &i.kind {
            InlineKind::Link { url, .. } => Some(url.to_string()),
            _ => None,
        })
        .collect();
    assert_eq!(urls, vec!["https://en.wikipedia.org/wiki/Magma_(algebra)", "a_b~c*d"]);
    assert_eq!(flat(&inlines), "see Magma and x");
    // Nested parens balance; an unbalanced URL or one with a space is text.
    assert_eq!(
        shape(&para("[x](a(b)c) ![y](p(q))")),
        vec![
            InlineKind::Link {
                content: vec![dummy(text("x"))],
                url: "a(b)c".into()
            },
            text(" "),
            InlineKind::Image {
                alt: "y".into(),
                url: "p(q)".into()
            },
        ]
    );
    for src in ["[x](a(b c)", "[x](a(b", "[x](a\nb)", "[x]((a)"] {
        assert!(para(src).iter().all(|i| matches!(i.kind, InlineKind::Text(_))), "{src}");
    }
}

#[test]
fn a_form_running_past_link_text_means_no_link() {
    // Code binds tighter than link brackets, as code spans do in CommonMark.
    let inlines = para("[see (show x](u) y)");
    assert_eq!(flat(&inlines), "[see (show x] (u) y)");
    assert!(matches!(&inlines[1].kind, InlineKind::Code(n) if n.is_form("show")));
    // A form closed inside the link text is part of it.
    let inlines = para("[see (show x)](u)");
    let InlineKind::Link { content, url } = &inlines[0].kind else {
        panic!("{inlines:?}")
    };
    assert_eq!(&**url, "u");
    assert!(matches!(&content[1].kind, InlineKind::Code(n) if n.to_string() == "(show x)"));
}

#[test]
fn inline_code_spans() {
    assert_eq!(
        shape(&para("``a`b`` and ` padded ` and `x\ny`")),
        vec![
            InlineKind::InlineCode("a`b".into()),
            text(" and "),
            InlineKind::InlineCode("padded".into()),
            text(" and "),
            InlineKind::InlineCode("x y".into()),
        ]
    );
}

#[test]
fn escapes() {
    assert_eq!(shape(&para(r"\*not italic\* \(show x) \a")), vec![text(r"*not italic* (show x) \a")]);
}

#[test]
fn emoji_known_and_unknown() {
    assert_eq!(
        shape(&para(":sob: :+1: :-1: :nope: 10:30:45 :fire:")),
        vec![
            InlineKind::Emoji(Sym::new("sob")),
            text(" "),
            InlineKind::Emoji(Sym::new("+1")),
            text(" "),
            InlineKind::Emoji(Sym::new("-1")),
            text(" :nope: 10:30:45 "),
            InlineKind::Emoji(Sym::new("fire")),
        ]
    );
    let src = "a :rocket: b";
    let inlines = para(src);
    assert_eq!(&src[inlines[1].span.range()], ":rocket:");
}

// ---------------------------------------------------------------------------
// Code vs prose
// ---------------------------------------------------------------------------

#[test]
fn prose_parentheses_stay_prose() {
    for src in [
        "(usually this works)",
        "(the show must go on)",
        "a (showing) b",
        "(tag x)",
        "(shows x)",
        "f(x) = (x + 1)",
        "unbalanced ( and ) and )(",
        "(show",
    ] {
        assert_eq!(shape(&para(src)), vec![text(src)], "{src}");
    }
}

#[test]
fn every_reserved_head_is_code() {
    for head in crate::RESERVED_HEADS {
        for sep in [" x", "\tx", "(x)", ")", "\nx", "\r\nx"] {
            let src = format!("a ({head}{sep}) b");
            let inlines = para(&src);
            assert!(
                matches!(&inlines[1].kind, InlineKind::Code(n) if n.is_form(head)),
                "{src:?} gave {inlines:?}"
            );
        }
    }
}

#[test]
fn reserved_head_glued_to_a_quote_or_string_is_prose() {
    // A head counts only when followed by whitespace, `(` or `)`.
    for src in ["(let's see)", "(show's over)", "(eval\"x\")", "(show'x)", "a (let's go) b", "(show;x)"] {
        assert_eq!(shape(&para(src)), vec![text(src)], "{src}");
    }
}

#[test]
fn escaped_paren_keeps_a_reserved_word_prose() {
    // Unescaped, this is read as code ...
    let doc = blocks("(show that f is continuous)");
    assert!(matches!(&doc[0].kind, BlockKind::Code(n) if n.is_form("show")));
    // ... and `\(` keeps it prose, alone or inside a sentence.
    assert_eq!(shape(&para(r"\(show that f is continuous)")), vec![text("(show that f is continuous)")]);
    assert_eq!(
        shape(&para(r"We \(show that f is continuous) now.")),
        vec![text("We (show that f is continuous) now.")]
    );
    // An escaped head does not hide a form nested inside it.
    let inlines = para(r"\(show (show x))");
    assert_eq!(flat(&inlines), "(show (show x))");
    assert!(matches!(&inlines[1].kind, InlineKind::Code(n) if n.to_string() == "(show x)"));
}

#[test]
fn ref_alone_is_never_a_block() {
    let doc = blocks("See\n\n(ref laplace)\n\nend");
    assert_eq!(doc.len(), 3);
    let BlockKind::Paragraph(p) = &doc[1].kind else {
        panic!("{doc:?}")
    };
    assert_eq!(p.len(), 1);
    assert!(matches!(&p[0].kind, InlineKind::Code(n) if n.to_string() == "(ref laplace)"));
    // The other heads still make blocks.
    for head in ["show", "eval", "let", "alias", "macro"] {
        let src = format!("({head} x)");
        assert!(matches!(&blocks(&src)[0].kind, BlockKind::Code(n) if n.is_form(head)), "{src}");
    }
}

#[test]
fn block_vs_inline_code() {
    // Alone in its paragraph: a block.
    let doc = blocks("  (show (= a b))  \n\nText (show a) here.\n\n(let x 2) (let y 3)\n\n(show x).");
    assert!(matches!(&doc[0].kind, BlockKind::Code(n) if n.is_form("show")));
    assert!(matches!(&doc[1].kind, BlockKind::Paragraph(_)));
    // Two forms, or a form plus punctuation, are inline.
    assert!(matches!(&doc[2].kind, BlockKind::Paragraph(p) if p.len() == 3));
    assert!(matches!(&doc[3].kind, BlockKind::Paragraph(p) if p.len() == 2));
    // In lists, quotes and headings a lone form is inline.
    let doc = blocks("- (show a)\n\n> (show b)\n\n# (show c)");
    let (_, items) = list(&doc[0]);
    assert!(matches!(item_inlines(&items[0])[0].kind, InlineKind::Code(_)));
    let BlockKind::BlockQuote(q) = &doc[1].kind else {
        panic!()
    };
    assert!(matches!(&q[0].kind, BlockKind::Paragraph(p) if matches!(p[0].kind, InlineKind::Code(_))));
    assert!(matches!(&doc[2].kind, BlockKind::Heading { content, .. } if matches!(content[0].kind, InlineKind::Code(_))));
}

#[test]
fn multi_line_show_blocks() {
    // A form spans consecutive non-blank lines, whatever they look like.
    let src = "Intro line\n\n(show\n    (=\n        (jux a b)\n        # not a heading\n- not a list\n    )\n)\n\nAfter.";
    let doc = blocks(src);
    assert_eq!(doc.len(), 3, "{doc:?}");
    let BlockKind::Code(node) = &doc[1].kind else {
        panic!("{doc:?}")
    };
    assert_eq!(node.to_string(), "(show (= (jux a b) # not a heading - not a list))");
    assert_eq!(&src[node.span.range()], &src[src.find("(show").unwrap()..=src.rfind(")\n").unwrap()]);
    assert!(matches!(&doc[2].kind, BlockKind::Paragraph(p) if flat(p) == "After."));
}

#[test]
fn a_form_never_crosses_a_blank_line() {
    let src = "(show\n    (=\n        (jux a b)\n\n        c)\n)\n\nAfter.";
    let (doc, diags) = parse_doc(src);
    assert_eq!(diags.iter().map(|d| d.code).collect::<Vec<_>>(), vec![DiagCode::UnclosedParen]);
    assert_eq!(diags.as_slice()[0].span.range(), 0..1);
    let BlockKind::Code(node) = &doc.blocks[0].kind else {
        panic!("{doc:?}")
    };
    assert_eq!(node.to_string(), "(show (= (jux a b) <error>))");
    // The rest is read as ordinary paragraphs again.
    assert_eq!(doc.blocks.len(), 3, "{doc:?}");
    assert!(matches!(&doc.blocks[1].kind, BlockKind::Paragraph(p) if flat(p) == "c)\n)"));
}

#[test]
fn unclosed_form_does_not_swallow_later_blocks() {
    let src = "(show (+ a b)\n\nSome prose.\n\n# Heading\n\n- item one\n- item two\n\ntext with a smiley :) end";
    let (doc, diags) = parse_doc(src);
    assert_eq!(diags.iter().map(|d| d.code).collect::<Vec<_>>(), vec![DiagCode::UnclosedParen]);
    assert_eq!(diags.as_slice()[0].span.range(), 0..1);
    let b = &doc.blocks;
    assert_eq!(b.len(), 5, "{b:?}");
    assert!(matches!(&b[0].kind, BlockKind::Code(n) if n.to_string() == "(show (+ a b) <error>)"));
    assert!(matches!(&b[1].kind, BlockKind::Paragraph(p) if flat(p) == "Some prose."));
    assert!(matches!(&b[2].kind, BlockKind::Heading { level: 1, content } if flat(content) == "Heading"));
    assert_eq!(list(&b[3]).1.len(), 2);
    // The stray `)` of the smiley is prose.
    let BlockKind::Paragraph(last) = &b[4].kind else {
        panic!("{b:?}")
    };
    assert_eq!(shape(last), vec![text("text with a smiley :) end")]);

    let src = "Intro (show (+ a b) and more\n\n## Section\n\n1) first\n2) second";
    let (doc, diags) = parse_doc(src);
    assert_eq!(diags.iter().map(|d| d.code).collect::<Vec<_>>(), vec![DiagCode::UnclosedParen]);
    assert_eq!(diags.as_slice()[0].span.start.to_usize(), src.find("(show").unwrap());
    let b = &doc.blocks;
    assert_eq!(b.len(), 3, "{b:?}");
    let BlockKind::Paragraph(p) = &b[0].kind else {
        panic!("{b:?}")
    };
    assert_eq!(flat(p), "Intro (show (+ a b) and more <error>)");
    assert!(matches!(&b[1].kind, BlockKind::Heading { level: 2, content } if flat(content) == "Section"));
    let (ordered, items) = list(&b[2]);
    assert_eq!(ordered, Some(1));
    assert_eq!(items.len(), 2);
    assert_eq!(flat(item_inlines(&items[0])), "first");
}

#[test]
fn stray_close_paren_after_an_unclosed_form_is_prose() {
    let (doc, diags) = parse_doc("see (show (a\n\nthen b) and c)");
    assert_eq!(diags.len(), 1);
    assert_eq!(doc.blocks.len(), 2);
    let BlockKind::Paragraph(p) = &doc.blocks[1].kind else {
        panic!("{doc:?}")
    };
    assert_eq!(shape(p), vec![text("then b) and c)")]);
}

#[test]
fn unclosed_form_in_a_list_item_or_quote_stays_inside_it() {
    // The next item is not swallowed.
    let (doc, diags) = parse_doc("- one (show (a\n- two\n- three");
    assert_eq!(diags.iter().map(|d| d.code).collect::<Vec<_>>(), vec![DiagCode::UnclosedParen]);
    assert_eq!(doc.blocks.len(), 1, "{doc:?}");
    let (_, items) = list(&doc.blocks[0]);
    assert_eq!(items.len(), 3);
    assert_eq!(flat(item_inlines(&items[0])), "one (show (a <error>))");
    assert_eq!(flat(item_inlines(&items[1])), "two");
    assert_eq!(flat(item_inlines(&items[2])), "three");
    // Nor is an unindented line after it: an unclosed form does not stretch
    // the item the way a closed one does.
    let (doc, diags) = parse_doc("- one (show (a\nlazy) line\n- two");
    assert_eq!(diags.len(), 1);
    assert_eq!(doc.blocks.len(), 3, "{doc:?}");
    assert!(matches!(&doc.blocks[1].kind, BlockKind::Paragraph(p) if flat(p) == "lazy) line"));
    // In a nested list too.
    let (doc, diags) = parse_doc("- a\n  - b (show (x\n  - c\n- d");
    assert_eq!(diags.len(), 1);
    let (_, outer) = list(&doc.blocks[0]);
    assert_eq!(outer.len(), 2);
    let (_, inner) = list(&outer[0].content[1]);
    assert_eq!(flat(item_inlines(&inner[1])), "c");

    // The text after a quote is not swallowed.
    let (doc, diags) = parse_doc("> see (show (a\n> more\nafter the quote");
    assert_eq!(diags.iter().map(|d| d.code).collect::<Vec<_>>(), vec![DiagCode::UnclosedParen]);
    assert_eq!(doc.blocks.len(), 2, "{doc:?}");
    let BlockKind::BlockQuote(q) = &doc.blocks[0].kind else {
        panic!("{doc:?}")
    };
    assert!(matches!(&q[0].kind, BlockKind::Paragraph(p) if flat(p) == "see (show (a more <error>))"));
    assert!(matches!(&doc.blocks[1].kind, BlockKind::Paragraph(p) if flat(p) == "after the quote"));
    // A blank `>` line ends the form but not the quote.
    let (doc, diags) = parse_doc("> (show (a\n>\n> next");
    assert_eq!(diags.len(), 1);
    let BlockKind::BlockQuote(q) = &doc.blocks[0].kind else {
        panic!("{doc:?}")
    };
    assert_eq!(q.len(), 2);
    assert!(matches!(&q[1].kind, BlockKind::Paragraph(p) if flat(p) == "next"));
}

#[test]
fn crlf_in_and_around_forms() {
    // A form over several CRLF lines.
    let doc = blocks("(show (= a\r\n    b))\r\n\r\nnext\r\n");
    assert_eq!(doc.len(), 2);
    assert!(matches!(&doc[0].kind, BlockKind::Code(n) if n.to_string() == "(show (= a b))"));
    assert!(matches!(&doc[1].kind, BlockKind::Paragraph(p) if flat(p) == "next"));
    // A CRLF blank line ends an unclosed form.
    let (doc, diags) = parse_doc("(show (a\r\n  b\r\n\r\n# H\r\n");
    assert_eq!(diags.iter().map(|d| d.code).collect::<Vec<_>>(), vec![DiagCode::UnclosedParen]);
    assert_eq!(doc.blocks.len(), 2, "{doc:?}");
    assert!(matches!(&doc.blocks[0].kind, BlockKind::Code(n) if n.to_string() == "(show (a b <error>))"));
    assert!(matches!(&doc.blocks[1].kind, BlockKind::Heading { content, .. } if flat(content) == "H"));
    // Whitespace-only lines count as blank too.
    let (doc, diags) = parse_doc("(show (a\r\n \t\r\nb)");
    assert_eq!(diags.len(), 1);
    assert_eq!(doc.blocks.len(), 2);
}

#[test]
fn semicolon_in_a_form_starts_a_comment() {
    let doc = blocks("(show (a;c\n b))");
    assert!(matches!(&doc[0].kind, BlockKind::Code(n) if n.to_string() == "(show (a b))"));
}

#[test]
fn multi_line_show_inside_a_sentence_and_list() {
    let src = "The transform (show\n    (= a\n        b)\n) for (show c) is\n- item (show (frac\n1 2)) end\n- next";
    let doc = blocks(src);
    assert_eq!(doc.len(), 2);
    let BlockKind::Paragraph(p) = &doc[0].kind else {
        panic!()
    };
    assert_eq!(flat(p), "The transform (show (= a b)) for (show c) is");
    let (_, items) = list(&doc[1]);
    assert_eq!(items.len(), 2);
    assert_eq!(flat(item_inlines(&items[0])), "item (show (frac 1 2)) end");
}

#[test]
fn multi_line_show_inside_a_quote() {
    let src = "> see (show (a\n> b))\n> done";
    let doc = blocks(src);
    let BlockKind::BlockQuote(q) = &doc[0].kind else {
        panic!()
    };
    let BlockKind::Paragraph(p) = &q[0].kind else {
        panic!()
    };
    assert_eq!(flat(p), "see (show (a b))\ndone");
}

#[test]
fn nothing_inside_code_is_code() {
    let doc = blocks("`(show x)` and\n```\n(show y)\n```");
    assert_eq!(all_code(&doc), vec![]);
}

#[test]
fn unclosed_paren_recovers_within_the_paragraph() {
    let src = "Text (show (frac a\nb\n\n# Next heading\n\nmore (show ok)";
    let (doc, diags) = parse_doc(src);
    assert_eq!(doc.blocks.len(), 3, "{doc:?}");
    assert!(matches!(doc.blocks[1].kind, BlockKind::Heading { level: 1, .. }));
    let codes: Vec<_> = diags.iter().map(|d| d.code).collect();
    // One diagnostic, on the outermost unclosed paren: exactly that `(`.
    assert_eq!(codes, vec![DiagCode::UnclosedParen]);
    assert_eq!(diags.as_slice()[0].span.start.to_usize(), src.find("(show").unwrap());
    assert!(diags.iter().all(|d| &src[d.span.range()] == "("));
    let found = all_code(&doc.blocks);
    assert_eq!(found.len(), 2);
    assert_eq!(found[0].1.to_string(), "(show (frac a b <error>))");
}

#[test]
fn unterminated_string_in_a_note() {
    let (doc, diags) = parse_doc("(show (text \"oops))\n\nnext");
    assert_eq!(diags.iter().map(|d| d.code).collect::<Vec<_>>().first(), Some(&DiagCode::UnterminatedString));
    assert_eq!(doc.blocks.len(), 2);
}

#[test]
fn diagnostics_are_not_duplicated_by_backtracking() {
    // The italic attempt fails and the content is parsed again; the broken
    // form must still report once (one unterminated string, one unclosed form).
    let (_, diags) = parse_doc("*a (show (x \"s) b");
    assert_eq!(diags.len(), 2, "{diags:?}");
}

// ---------------------------------------------------------------------------
// Spans
// ---------------------------------------------------------------------------

#[test]
fn spans_with_multibyte_text() {
    let src = "# τ **αβ** 😭\n\nκ (show (+ τ 1)) _λ_ [π](u) `ω` :heart:\n\n- ✓ item\n\n> «quote»";
    let doc = blocks(src);
    let check = |span: Span| {
        assert!(src.is_char_boundary(span.start.to_usize()) && src.is_char_boundary(span.end.to_usize()));
        assert!(span.start <= span.end && span.end.to_usize() <= src.len());
        &src[span.range()]
    };
    assert_eq!(check(doc[0].span), "# τ **αβ** 😭");
    let BlockKind::Heading { content, .. } = &doc[0].kind else {
        panic!()
    };
    assert_eq!(check(content[1].span), "**αβ**");
    let BlockKind::Paragraph(p) = &doc[1].kind else {
        panic!()
    };
    let texts: Vec<&str> = p.iter().map(|i| check(i.span)).collect();
    assert_eq!(
        texts,
        vec!["κ ", "(show (+ τ 1))", " ", "_λ_", " ", "[π](u)", " ", "`ω`", " ", ":heart:"]
    );
    let InlineKind::Code(node) = &p[1].kind else {
        panic!()
    };
    assert_eq!(check(node.args()[0].args()[0].span), "τ");
    let (_, items) = list(&doc[2]);
    assert_eq!(check(items[0].span), "- ✓ item");
    assert_eq!(check(doc[3].span), "> «quote»");
    let BlockKind::BlockQuote(q) = &doc[3].kind else {
        panic!()
    };
    assert_eq!(check(q[0].span), "«quote»");
}

#[test]
fn text_spans_cover_soft_breaks() {
    let src = "ab\n  cd";
    let inlines = para(src);
    assert_eq!(inlines[0].span.range(), 0..src.len());
}

// ---------------------------------------------------------------------------
// Robustness
// ---------------------------------------------------------------------------

#[test]
fn never_panics_on_odd_input() {
    let nasty = [
        "",
        "\n\n\n",
        "\u{feff}# bom",
        "(",
        "(show",
        "(show ",
        "(show (",
        "(show \"",
        "(show '",
        ")",
        "[",
        "![",
        "[a](",
        "`",
        "```",
        "-",
        "- ",
        "-[",
        "-[ ",
        "-[ ]",
        "1.",
        "1)",
        "#",
        ">",
        "> >",
        ">\n>",
        "*",
        "**",
        "_",
        "~",
        "||",
        ":",
        "::",
        ":+",
        "\\",
        "a\\",
        "a  ",
        "\t- a\n\t\t- b",
        "- a\n\t- b\n  \t  - c",
        "😭(show 😭)😭",
        "(show (x)) (show (y\n\n",
        "> (show (a\n\nb))",
        "- (show (a\n\n  b))\n- c",
        "\r",
        "\r\n\r\n",
        "a\rb",
    ];
    for src in nasty {
        let (doc, _) = parse_doc(src);
        check_spans(src, &doc.blocks);
    }
}

#[test]
fn deep_nesting_is_bounded() {
    let n = 10_000;
    let quotes = ">".repeat(n) + " x";
    let lists: String = (0..200).flat_map(|i| ["  ".repeat(i), "- x\n".to_owned()]).collect();
    let stars = "*a ".repeat(n);
    let links = "[".repeat(n) + &"](u)".repeat(n);
    let parens = format!("(show {}", "(".repeat(n));
    let mixed = "**_~||".repeat(n);
    for src in [quotes, lists, stars, links, parens, mixed] {
        let (doc, _) = parse_doc(&src);
        check_spans(&src, &doc.blocks);
    }
}

/// Inputs that used to take quadratic time (or read to the end of the note
/// for every unclosed form) parse quickly.
#[test]
fn pathological_inputs_scale_linearly() {
    let cases = [
        ("unclosed forms between blank lines", "(show x\n\n".repeat(5_000)),
        ("unclosed forms in one paragraph", "(show x\n".repeat(5_000)),
        ("unclosed forms in list items", "- (show x\n".repeat(5_000)),
        ("unclosed forms in nested items", "- a\n  - (show x\n".repeat(2_500)),
        ("unclosed forms in a quote", "> (show x\n".repeat(5_000)),
        ("unclosed forms in items with lazy lines", "- (show (x\nlazy\n".repeat(5_000)),
        ("unclosed forms in quotes with lazy lines", "> (show (x\nlazy\n".repeat(5_000)),
        ("forms inside link text", format!("[{}](u)", "(show x ".repeat(5_000))),
        ("links without a closing paren", "[a](".repeat(20_000)),
        ("images without a closing paren", "![a](".repeat(20_000)),
        ("link openers before text", format!("{} x)", "[a](b".repeat(10_000))),
        ("many unclosed parens", format!("(show {}\n\n", "(a ".repeat(200)).repeat(200)),
    ];
    for (what, src) in cases {
        let started = std::time::Instant::now();
        let (doc, _) = parse_doc(&src);
        let took = started.elapsed();
        check_spans(&src, &doc.blocks);
        assert!(took < std::time::Duration::from_secs(1), "{what}: {took:?}");
    }
    // One diagnostic per unclosed form, not one per paren or per re-read.
    let (_, diags) = parse_doc(&"(show x\n\n".repeat(5_000));
    assert_eq!(diags.len(), 5_000);
    let (_, diags) = parse_doc(&format!("(show {}", "(a ".repeat(200)));
    assert_eq!(diags.len(), 1);
}

fn check_spans(src: &str, blocks: &[Block]) {
    fn ok(src: &str, span: Span) {
        let (a, b) = (span.start.to_usize(), span.end.to_usize());
        assert!(a <= b && b <= src.len(), "bad span {span:?} in {src:?}");
        assert!(src.is_char_boundary(a) && src.is_char_boundary(b), "span off a char boundary in {src:?}");
    }
    fn node(src: &str, n: &Node) {
        ok(src, n.span);
        if let NodeKind::List(items) = &n.kind {
            for item in items {
                node(src, item);
            }
        }
    }
    fn inl(src: &str, inlines: &[Inline]) {
        for i in inlines {
            ok(src, i.span);
            match &i.kind {
                InlineKind::Code(n) => node(src, n),
                InlineKind::Bold(c)
                | InlineKind::Italic(c)
                | InlineKind::Underline(c)
                | InlineKind::Strike(c)
                | InlineKind::Spoiler(c)
                | InlineKind::Link { content: c, .. } => inl(src, c),
                _ => {}
            }
        }
    }
    for b in blocks {
        ok(src, b.span);
        match &b.kind {
            BlockKind::Code(n) => node(src, n),
            BlockKind::Heading { content, .. } | BlockKind::Paragraph(content) => inl(src, content),
            BlockKind::List { items, .. } => {
                for item in items {
                    ok(src, item.span);
                    check_spans(src, &item.content);
                }
            }
            BlockKind::BlockQuote(inner) => check_spans(src, inner),
            BlockKind::CodeBlock { .. } | BlockKind::Image { .. } | BlockKind::Rule => {}
        }
    }
}

use crate::sym::Sym;
