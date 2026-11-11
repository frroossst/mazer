//! `textDocument/semanticTokens/full`: code coloured by what it is.
//!
//! Only code is tokenized; prose is left to the editor's own highlighting,
//! so code stands out from it. Token types:
//!
//! | type         | what                                                     |
//! |--------------|----------------------------------------------------------|
//! | `keyword`    | reserved words (`show`, `let`, ...) and `tag`           |
//! | `function`   | keyword calls and bare keywords (`defaultLibrary`), heads of function applications |
//! | `macro`      | macro and alias uses and definitions                     |
//! | `enumMember` | display names (`tau`, `alpha`, `->`)                     |
//! | `variable`   | `let` variables                                          |
//! | `parameter`  | macro parameters and every other name                    |
//! | `number`     | numbers                                                  |
//! | `string`     | string literals (split per line)                         |
//! | `comment`    | `;` comments, found in the text since the reader drops them |
//! | `operator`   | the `'` of `'x` and the `-` of `-x`                      |
//! | `label`      | the names in `(tag name)` and `(ref name)` (custom type) |
//!
//! Modifiers: `declaration` where a name is defined, `defaultLibrary` on
//! built-in keywords.
//!
//! Tokens are sorted, non-overlapping, single-line, with UTF-16 positions and
//! lengths, delta-encoded as the protocol requires.

use lsp_types::{
    SemanticToken, SemanticTokenModifier, SemanticTokenType, SemanticTokens, SemanticTokensLegend,
};
use mazer_doc::syntax::NodeKind;
use mazer_doc::{Analysis, ByteOffset, Span};

use crate::convert::{self, NOTE, note_text};
use crate::tree::{self, Role};

/// Token type indices into [`legend`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u32)]
pub enum Kind {
    Keyword = 0,
    Function,
    Macro,
    EnumMember,
    Variable,
    Parameter,
    Number,
    String,
    Comment,
    Operator,
    Label,
}

/// Modifier bits.
pub const DECLARATION: u32 = 1;
pub const DEFAULT_LIBRARY: u32 = 2;

/// The legend declared in `initialize`. The order matches [`Kind`].
#[must_use]
pub fn legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: vec![
            SemanticTokenType::KEYWORD,
            SemanticTokenType::FUNCTION,
            SemanticTokenType::MACRO,
            SemanticTokenType::ENUM_MEMBER,
            SemanticTokenType::VARIABLE,
            SemanticTokenType::PARAMETER,
            SemanticTokenType::NUMBER,
            SemanticTokenType::STRING,
            SemanticTokenType::COMMENT,
            SemanticTokenType::OPERATOR,
            SemanticTokenType::new("label"),
        ],
        token_modifiers: vec![SemanticTokenModifier::DECLARATION, SemanticTokenModifier::DEFAULT_LIBRARY],
    }
}

/// One token before encoding: a byte range of the note.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Raw {
    pub start: usize,
    pub end: usize,
    pub kind: Kind,
    pub modifiers: u32,
}

/// The tokens of the note, sorted and non-overlapping, each on one line.
#[must_use]
pub fn raw_tokens(analysis: &Analysis) -> Vec<Raw> {
    let text = note_text(&analysis.sources);
    let mut raw = Vec::new();
    let mut push = |span: Span, kind: Kind, modifiers: u32| {
        if span.file == NOTE && span.start < span.end && span.end.to_usize() <= text.len() {
            raw.push(Raw {
                start: span.start.to_usize(),
                end: span.end.to_usize(),
                kind,
                modifiers,
            });
        }
    };
    for (form, _) in mazer_doc::code_forms(analysis) {
        tree::visit(text, form, &mut |node, ctx| match &node.kind {
            NodeKind::Ident(_) => {
                let (kind, modifiers) = ident_kind(tree::classify(analysis, node, ctx));
                push(node.span, kind, modifiers);
            }
            NodeKind::Number(_) => push(node.span, Kind::Number, 0),
            NodeKind::Str(_) => push(node.span, Kind::String, 0),
            NodeKind::List(_) | NodeKind::Error => {}
        });
    }
    for span in mazer_doc::comment_spans(analysis) {
        push(span, Kind::Comment, 0);
    }
    raw.sort_by_key(|t| (t.start, t.end));
    let mut out: Vec<Raw> = Vec::with_capacity(raw.len());
    for token in raw {
        if out.last().is_some_and(|prev| token.start < prev.end) {
            continue;
        }
        // Split at line breaks: a token never spans lines.
        let mut start = token.start;
        for (i, b) in text.as_bytes()[token.start..token.end].iter().enumerate() {
            if *b == b'\n' {
                let mut end = token.start + i;
                if end > start && text.as_bytes()[end - 1] == b'\r' {
                    end -= 1;
                }
                if end > start {
                    out.push(Raw { start, end, ..token });
                }
                start = token.start + i + 1;
            }
        }
        if token.end > start {
            out.push(Raw { start, ..token });
        }
    }
    out
}

const fn ident_kind(role: Role<'_>) -> (Kind, u32) {
    match role {
        Role::Mark => (Kind::Operator, 0),
        Role::Reserved(_) => (Kind::Keyword, 0),
        Role::Keyword(_) => (Kind::Function, DEFAULT_LIBRARY),
        Role::MacroUse(_) => (Kind::Macro, 0),
        Role::MacroName(_) => (Kind::Macro, DECLARATION),
        Role::MacroParam | Role::Free => (Kind::Parameter, 0),
        Role::LetName(_) => (Kind::Variable, DECLARATION),
        Role::LetUse(_) => (Kind::Variable, 0),
        Role::TagName(..) => (Kind::Label, DECLARATION),
        Role::RefName(..) => (Kind::Label, 0),
        Role::Display(_) => (Kind::EnumMember, 0),
        Role::Function => (Kind::Function, 0),
    }
}

/// The semantic tokens of the note, encoded for the protocol.
#[must_use]
pub fn semantic_tokens(analysis: &Analysis) -> SemanticTokens {
    let sources = &analysis.sources;
    let text = note_text(sources);
    let mut data = Vec::new();
    let (mut prev_line, mut prev_col) = (0u32, 0u32);
    for token in raw_tokens(analysis) {
        let pos = convert::position(sources, NOTE, ByteOffset::from_usize(token.start));
        let length = convert::utf16_len(&text[token.start..token.end]);
        let delta_line = pos.line - prev_line;
        let delta_start = if delta_line == 0 { pos.character - prev_col } else { pos.character };
        data.push(SemanticToken {
            delta_line,
            delta_start,
            length,
            token_type: token.kind as u32,
            token_modifiers_bitset: token.modifiers,
        });
        prev_line = pos.line;
        prev_col = pos.character;
    }
    SemanticTokens { result_id: None, data }
}

/// Decodes protocol tokens back to `(line, column, length, type, modifiers)`
/// with absolute UTF-16 positions. For tests and checks.
#[must_use]
pub fn decode(tokens: &SemanticTokens) -> Vec<(u32, u32, u32, u32, u32)> {
    let (mut line, mut col) = (0u32, 0u32);
    tokens
        .data
        .iter()
        .map(|t| {
            if t.delta_line > 0 {
                line += t.delta_line;
                col = t.delta_start;
            } else {
                col += t.delta_start;
            }
            (line, col, t.length, t.token_type, t.token_modifiers_bitset)
        })
        .collect()
}

/// Checks that encoded tokens are well formed for `text`: sorted, not
/// overlapping, each within one line, with valid types. Returns the first
/// problem found.
///
/// # Errors
/// A description of the first malformed token.
pub fn check_well_formed(text: &str, tokens: &SemanticTokens) -> Result<(), String> {
    let lines: Vec<&str> = text.split('\n').collect();
    let types = u32::try_from(legend().token_types.len()).unwrap_or(u32::MAX);
    let mut prev: Option<(u32, u32, u32)> = None;
    for (i, (line, col, len, ty, _)) in decode(tokens).into_iter().enumerate() {
        let Some(text_line) = lines.get(line as usize) else {
            return Err(format!("token {i} is on line {line}, past the end"));
        };
        let width = convert::utf16_len(text_line.strip_suffix('\r').unwrap_or(text_line));
        if len == 0 || col + len > width {
            return Err(format!("token {i} at {line}:{col} (length {len}) does not fit its line of {width} units"));
        }
        if ty >= types {
            return Err(format!("token {i} has unknown type {ty}"));
        }
        if let Some((pl, pc, plen)) = prev
            && (line < pl || (line == pl && col < pc + plen))
        {
            return Err(format!("token {i} at {line}:{col} overlaps or precedes {pl}:{pc}"));
        }
        prev = Some((line, col, len));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use mazer_doc::analyze;

    use super::*;

    /// `(text, kind)` of every token.
    fn kinds(src: &str) -> Vec<(String, Kind)> {
        let analysis = analyze(src, "t.zr");
        raw_tokens(&analysis)
            .into_iter()
            .map(|t| (src[t.start..t.end].to_owned(), t.kind))
            .collect()
    }

    #[test]
    fn what_each_name_is() {
        use Kind::*;
        let src = "(alias R (bb R))\n\n(let x 2)\n\n(show (tag e) (+ (frac x tau) R 'y (f -z) 1.5 \"s\") ; note\n)\n\nSee (ref e).";
        let got = kinds(src);
        let want: Vec<(std::string::String, Kind)> = [
            ("alias", Keyword),
            ("R", Macro),
            ("bb", Function),
            ("R", Parameter),
            ("let", Keyword),
            ("x", Variable),
            ("2", Number),
            ("show", Keyword),
            ("tag", Keyword),
            ("e", Label),
            ("+", Function),
            ("frac", Function),
            ("x", Variable),
            ("tau", EnumMember),
            ("R", Macro),
            ("'", Operator),
            ("y", Parameter),
            ("f", Function),
            ("-", Operator),
            ("z", Parameter),
            ("1.5", Number),
            ("\"s\"", String),
            ("; note", Comment),
            ("ref", Keyword),
            ("e", Label),
        ]
        .into_iter()
        .map(|(t, k)| (t.to_owned(), k))
        .collect();
        assert_eq!(got, want);
    }

    #[test]
    fn utf16_positions_and_lengths() {
        // Multibyte text before and inside code.
        let src = "τ😭 text (show (+ α β 😭x \"ü😭\") ; ü😭\n)\n";
        let analysis = analyze(src, "t.zr");
        let tokens = semantic_tokens(&analysis);
        check_well_formed(src, &tokens).unwrap();
        let decoded = decode(&tokens);
        // `show` starts after τ(1) 😭(2) " text (" (7) = column 10.
        assert_eq!(decoded[0], (0, 10, 4, Kind::Keyword as u32, 0));
        // α: one unit; 😭x: three; the string "ü😭": five with its quotes.
        let lens: Vec<u32> = decoded.iter().map(|t| t.2).collect();
        assert_eq!(lens, vec![4, 1, 1, 1, 3, 5, 5], "the comment `; ü😭` is five units");
    }

    #[test]
    fn multi_line_strings_and_comments_split() {
        let src = "(macro (f a)\r\n  \"doc\r\n  more\" ; c1\r\n  (g a)) ; c2";
        let analysis = analyze(src, "t.zr");
        let tokens = semantic_tokens(&analysis);
        check_well_formed(src, &tokens).unwrap();
        let got = kinds(src);
        assert!(got.contains(&("\"doc".to_owned(), Kind::String)), "{got:?}");
        assert!(got.contains(&("  more\"".to_owned(), Kind::String)), "{got:?}");
        assert!(got.contains(&("; c1".to_owned(), Kind::Comment)), "{got:?}");
        // A comment after the form is prose, not code.
        assert!(!got.iter().any(|(t, _)| t == "; c2"), "{got:?}");
    }

    #[test]
    fn unclosed_and_empty() {
        for src in ["(show (frac a", "", "(", "(show \"open", "(show (+ a ; c"] {
            let analysis = analyze(src, "t.zr");
            check_well_formed(src, &semantic_tokens(&analysis)).unwrap();
        }
    }
}
