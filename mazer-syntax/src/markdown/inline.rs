//! Inline content: text, styles, links, inline code, emoji and code forms
//! within the lines of one paragraph, heading or similar.
//!
//! The lines are flattened into a vector of [`Unit`]s (one per character,
//! plus one per line break) that remember their byte offsets, so every inline
//! gets an exact span while the parser itself can look ahead freely.

use std::collections::HashMap;

use super::{is_form_start, read_code};
use crate::diag::Diags;
use crate::doc::{Inline, InlineKind};
use crate::emoji;
use crate::node::Node;
use crate::reader::Seg;
use crate::span::{ByteOffset, FileId, Span};
use crate::sym::Sym;

/// How deeply styles and links may nest before markers are read as text.
const MAX_DEPTH: usize = 32;

/// Parses the inline content of `segs`, the lines of one block. The first
/// segment starts where the content starts (after a heading's `#`, a list
/// marker and so on). Diagnostics from code forms go to `diags`.
pub(super) fn parse(text: &str, file: FileId, segs: &[Seg], diags: &mut Diags) -> Vec<Inline> {
    let mut parser = Parser::new(text, file, segs);
    let len = parser.units.len();
    let content = parser.seq(0, len, None, 0).map_or_default(|(content, _)| content);
    parser.emit_diags(&content, diags);
    content
}

#[derive(Clone, Copy, Debug)]
struct Unit {
    start: usize,
    end: usize,
    /// The character, or `'\n'` for the break between two lines.
    ch: char,
    /// For a line break: whether it is a hard break.
    hard: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Delim {
    Bold,
    Italic,
    Underline,
    Strike1,
    Strike2,
    Spoiler,
}

impl Delim {
    const ALL: [Self; 6] = [
        Self::Bold,
        Self::Italic,
        Self::Underline,
        Self::Strike1,
        Self::Strike2,
        Self::Spoiler,
    ];

    const fn len(self) -> usize {
        match self {
            Self::Italic | Self::Underline | Self::Strike1 => 1,
            Self::Bold | Self::Strike2 | Self::Spoiler => 2,
        }
    }

    const fn ch(self) -> char {
        match self {
            Self::Bold | Self::Italic => '*',
            Self::Underline => '_',
            Self::Strike1 | Self::Strike2 => '~',
            Self::Spoiler => '|',
        }
    }

    const fn index(self) -> usize {
        self as usize
    }

    const fn wrap(self, content: Vec<Inline>) -> InlineKind {
        match self {
            Self::Bold => InlineKind::Bold(content),
            Self::Italic => InlineKind::Italic(content),
            Self::Underline => InlineKind::Underline(content),
            Self::Strike1 | Self::Strike2 => InlineKind::Strike(content),
            Self::Spoiler => InlineKind::Spoiler(content),
        }
    }
}

/// Plain text being gathered until the next non-text inline.
#[derive(Default)]
struct TextBuf {
    text: String,
    start: usize,
    end: usize,
}

impl TextBuf {
    fn push(&mut self, ch: char, start: usize, end: usize) {
        if self.text.is_empty() {
            self.start = start;
        }
        self.text.push(ch);
        self.end = end;
    }

    fn flush(&mut self, file: FileId, out: &mut Vec<Inline>) {
        if !self.text.is_empty() {
            let span = Span::new(file, ByteOffset::from_usize(self.start), ByteOffset::from_usize(self.end));
            out.push(Inline::new(InlineKind::Text(std::mem::take(&mut self.text).into()), span));
        }
    }
}

type Parsed = Option<(Vec<Inline>, usize)>;

struct Parser<'a> {
    text: &'a str,
    file: FileId,
    /// The block's lines, untrimmed: what the reader reads code forms from.
    segs: &'a [Seg],
    units: Vec<Unit>,
    /// For each [`Delim`], the units where a closer of that kind may stand.
    closers: [Vec<usize>; 6],
    /// For each `[`, the unit of its matching `]`.
    brackets: HashMap<usize, usize>,
    /// For each `(`, the unit of the `)` that balances it without crossing
    /// whitespace: where a link's URL can end.
    parens: HashMap<usize, usize>,
    /// Results of trying a style opener: `(unit, end, delim)` to the content
    /// and the unit after the closer, or `None` when it did not close.
    attempts: HashMap<(usize, usize, Delim), Parsed>,
    /// Code forms already read: unit of the `(` to the node and the next unit.
    codes: HashMap<usize, (Node, usize)>,
    /// Diagnostics of each code form, by the byte offset of its `(`. Emitted
    /// only for forms that end up in the final tree, each exactly once.
    code_diags: HashMap<usize, Diags>,
    /// Set when a code form inside link text runs past the closing `]`. The
    /// link then fails and the content parse stops at once, so the forms
    /// nested inside that form are not each read again.
    overrun: bool,
}

impl<'a> Parser<'a> {
    fn new(text: &'a str, file: FileId, segs: &'a [Seg]) -> Self {
        let units = units(text, segs);
        let mut parser = Self {
            text,
            file,
            segs,
            units,
            closers: Default::default(),
            brackets: HashMap::new(),
            parens: HashMap::new(),
            attempts: HashMap::new(),
            codes: HashMap::new(),
            code_diags: HashMap::new(),
            overrun: false,
        };
        for i in 0..parser.units.len() {
            for delim in Delim::ALL {
                if parser.is_closer(i, delim) {
                    parser.closers[delim.index()].push(i);
                }
            }
        }
        parser.brackets = match_brackets(&parser.units);
        parser.parens = match_url_parens(&parser.units);
        parser
    }

    fn span(&self, first: usize, last: usize) -> Span {
        Span::new(
            self.file,
            ByteOffset::from_usize(self.units[first].start),
            ByteOffset::from_usize(self.units[last].end),
        )
    }

    fn ch(&self, i: usize) -> Option<char> {
        self.units.get(i).map(|u| u.ch)
    }

    /// Whitespace, a line break, or outside the content.
    fn is_space(&self, i: Option<usize>) -> bool {
        i.and_then(|i| self.ch(i)).is_none_or(char::is_whitespace)
    }

    fn is_alnum(&self, i: Option<usize>) -> bool {
        i.and_then(|i| self.ch(i)).is_some_and(char::is_alphanumeric)
    }

    /// Parses units `start..end`. With a `closer`, stops after the matching
    /// closing marker and returns `None` if there is none.
    fn seq(&mut self, start: usize, end: usize, closer: Option<Delim>, depth: usize) -> Parsed {
        let mut out = Vec::new();
        let mut buf = TextBuf::default();
        let mut i = start;
        while i < end {
            if self.overrun {
                return None;
            }
            if let Some(delim) = closer
                && i > start
                && i + delim.len() <= end
                && self.is_closer(i, delim)
            {
                buf.flush(self.file, &mut out);
                return Some((out, i + delim.len()));
            }
            let unit = self.units[i];
            let found = match unit.ch {
                '\n' if unit.hard => Some((Inline::new(InlineKind::LineBreak, self.span(i, i)), i + 1)),
                '\\' => {
                    match self.ch(i + 1).filter(|&c| i + 1 < end && c.is_ascii_punctuation()) {
                        Some(c) => {
                            buf.push(c, unit.start, self.units[i + 1].end);
                            i += 2;
                            continue;
                        }
                        None => None,
                    }
                }
                '`' => match self.code_span(i, end) {
                    Ok(found) => Some(found),
                    Err(run) => {
                        self.push_text(&mut buf, i, run);
                        i += run;
                        continue;
                    }
                },
                '(' if is_form_start(self.text, unit.start) => self.code(i, end),
                '[' => self.link(i, end, depth),
                '!' if self.ch(i + 1) == Some('[') => self.link(i, end, depth),
                ':' => self.emoji(i, end),
                '*' | '_' | '~' | '|' => match self.style(i, end, depth) {
                    Ok(found) => Some(found),
                    Err(run) => {
                        self.push_text(&mut buf, i, run);
                        i += run;
                        continue;
                    }
                },
                _ => None,
            };
            if let Some((inline, next)) = found {
                buf.flush(self.file, &mut out);
                out.push(inline);
                i = next;
            } else {
                buf.push(unit.ch, unit.start, unit.end);
                i += 1;
            }
        }
        if closer.is_some() {
            return None;
        }
        buf.flush(self.file, &mut out);
        Some((out, end))
    }

    fn push_text(&self, buf: &mut TextBuf, start: usize, count: usize) {
        for unit in &self.units[start..start + count] {
            buf.push(unit.ch, unit.start, unit.end);
        }
    }

    /// Characters of units `start..end` as plain text, line breaks as spaces.
    fn plain(&self, start: usize, end: usize) -> String {
        self.units[start..end].iter().map(|u| if u.ch == '\n' { ' ' } else { u.ch }).collect()
    }

    fn run(&self, i: usize, end: usize, ch: char) -> usize {
        self.units[i..end].iter().take_while(|u| u.ch == ch).count()
    }

    // ---- styles ----

    /// Which marker starts at `i`, or `Err(n)`: the next `n` units are text.
    fn token(&self, i: usize, end: usize) -> Result<Delim, usize> {
        match self.units[i].ch {
            '*' if self.run(i, end, '*') >= 2 => Ok(Delim::Bold),
            '*' => Ok(Delim::Italic),
            // `_` has no double form, so a run of them (`__init__`) is text.
            '_' => match self.run(i, end, '_') {
                1 => Ok(Delim::Underline),
                run => Err(run),
            },
            '~' => match self.run(i, end, '~') {
                1 => Ok(Delim::Strike1),
                2 => Ok(Delim::Strike2),
                run => Err(run),
            },
            '|' if self.run(i, end, '|') >= 2 => Ok(Delim::Spoiler),
            _ => Err(1),
        }
    }

    fn can_open(&self, i: usize, delim: Delim, end: usize) -> bool {
        let after = i + delim.len();
        let glued_left = matches!(delim, Delim::Italic | Delim::Underline) && self.is_alnum(i.checked_sub(1));
        after < end && !self.is_space(Some(after)) && !glued_left
    }

    /// Whether a closer for `delim` stands at `i` (ignoring any range end).
    fn is_closer(&self, i: usize, delim: Delim) -> bool {
        let len = delim.len();
        if (i..i + len).any(|k| self.ch(k) != Some(delim.ch())) || self.is_space(i.checked_sub(1)) {
            return false;
        }
        let before = i.checked_sub(1).and_then(|k| self.ch(k));
        match delim {
            Delim::Underline => before != Some('_') && self.ch(i + 1) != Some('_') && !self.is_alnum(Some(i + 1)),
            Delim::Strike1 | Delim::Strike2 => before != Some('~') && self.ch(i + len) != Some('~'),
            Delim::Bold | Delim::Italic | Delim::Spoiler => true,
        }
    }

    /// Whether some closer for `delim` starts in `from..` and ends by `end`.
    fn has_closer(&self, delim: Delim, from: usize, end: usize) -> bool {
        let list = &self.closers[delim.index()];
        let at = list.partition_point(|&c| c < from);
        list.get(at).is_some_and(|&c| c + delim.len() <= end)
    }

    fn style(&mut self, i: usize, end: usize, depth: usize) -> Result<(Inline, usize), usize> {
        let delim = self.token(i, end)?;
        let len = delim.len();
        if depth >= MAX_DEPTH || !self.can_open(i, delim, end) || !self.has_closer(delim, i + len + 1, end) {
            return Err(len);
        }
        let key = (i, end, delim);
        let parsed = if let Some(done) = self.attempts.get(&key) {
            done.clone()
        } else {
            let parsed = self.seq(i + len, end, Some(delim), depth + 1);
            self.attempts.insert(key, parsed.clone());
            parsed
        };
        let (content, next) = parsed.ok_or(len)?;
        Ok((Inline::new(delim.wrap(content), self.span(i, next - 1)), next))
    }

    // ---- other inlines ----

    /// Inline code, or `Err(n)`: the `n` backticks at `i` are text.
    fn code_span(&self, i: usize, end: usize) -> Result<(Inline, usize), usize> {
        let run = self.run(i, end, '`');
        let mut k = i + run;
        while k < end {
            if self.units[k].ch == '`' {
                let len = self.run(k, end, '`');
                if len == run {
                    let mut code = self.plain(i + run, k);
                    if code.len() >= 2 && code.starts_with(' ') && code.ends_with(' ') && code.trim() != "" {
                        code.pop();
                        code.remove(0);
                    }
                    let inline = Inline::new(InlineKind::InlineCode(code.into()), self.span(i, k + run - 1));
                    return Ok((inline, k + run));
                }
                k += len;
            } else {
                k += 1;
            }
        }
        Err(run)
    }

    /// A reserved-head form starting at unit `i`.
    fn code(&mut self, i: usize, end: usize) -> Option<(Inline, usize)> {
        if !self.codes.contains_key(&i) {
            let at = self.units[i].start;
            let seg = self.segs.partition_point(|s| s.end <= at).min(self.segs.len() - 1);
            let (read, diags) = read_code(self.text, self.file, &self.segs[seg..], at);
            let stop = read.end.to_usize().max(at + 1);
            let next = self.units.partition_point(|u| u.start < stop);
            self.code_diags.insert(at, diags);
            self.codes.insert(i, (read.node, next));
        }
        let (node, next) = &self.codes[&i];
        if *next > end {
            // A form running past the end of link text: code binds tighter
            // than link brackets, so there is no link here.
            self.overrun = true;
            return None;
        }
        Some((Inline::new(InlineKind::Code(node.clone()), node.span), *next))
    }

    /// `[text](url)` at `i`, or `![alt](url)` when the unit at `i` is `!`.
    fn link(&mut self, i: usize, end: usize, depth: usize) -> Option<(Inline, usize)> {
        if depth >= MAX_DEPTH {
            return None;
        }
        let image = self.units[i].ch == '!';
        let open = if image { i + 1 } else { i };
        let close = self.closing_bracket(open, end)?;
        if close + 1 >= end || self.units[close + 1].ch != '(' {
            return None;
        }
        let (url, paren) = self.url(close + 1, end)?;
        let kind = if image {
            InlineKind::Image {
                alt: self.plain(open + 1, close).into(),
                url: url.into(),
            }
        } else {
            let outer = std::mem::replace(&mut self.overrun, false);
            let parsed = self.seq(open + 1, close, None, depth + 1);
            if std::mem::replace(&mut self.overrun, outer) {
                return None;
            }
            let content = parsed.map_or_default(|(c, _)| c);
            InlineKind::Link {
                content,
                url: url.into(),
            }
        };
        Some((Inline::new(kind, self.span(i, paren)), paren + 1))
    }

    fn closing_bracket(&self, open: usize, end: usize) -> Option<usize> {
        self.brackets.get(&open).copied().filter(|&close| close < end)
    }

    /// The URL after the `(` at unit `open`, up to the `)` that balances it
    /// (before `end`, with no whitespace in between): the URL and the unit of
    /// that `)`. A lookup, so failed links cost nothing to retry.
    fn url(&self, open: usize, end: usize) -> Option<(String, usize)> {
        let close = self.parens.get(&open).copied().filter(|&close| close < end)?;
        Some((self.units[open + 1..close].iter().map(|u| u.ch).collect(), close))
    }

    fn emoji(&self, i: usize, end: usize) -> Option<(Inline, usize)> {
        let name_len = self.units[i + 1..end]
            .iter()
            .take(32)
            .take_while(|u| u.ch.is_ascii_alphanumeric() || matches!(u.ch, '_' | '+' | '-'))
            .count();
        let close = i + 1 + name_len;
        if name_len == 0 || close >= end || self.units[close].ch != ':' {
            return None;
        }
        let name = self.plain(i + 1, close);
        emoji::lookup(&name)?;
        Some((Inline::new(InlineKind::Emoji(Sym::new(&name)), self.span(i, close)), close + 1))
    }

    /// Pushes the diagnostics of every code form in `content`.
    fn emit_diags(&mut self, content: &[Inline], diags: &mut Diags) {
        for inline in content {
            match &inline.kind {
                InlineKind::Code(node) => {
                    if let Some(found) = self.code_diags.remove(&node.span.start.to_usize()) {
                        diags.extend(found);
                    }
                }
                InlineKind::Bold(inner)
                | InlineKind::Italic(inner)
                | InlineKind::Underline(inner)
                | InlineKind::Strike(inner)
                | InlineKind::Spoiler(inner)
                | InlineKind::Link { content: inner, .. } => self.emit_diags(inner, diags),
                InlineKind::Text(_)
                | InlineKind::Image { .. }
                | InlineKind::InlineCode(_)
                | InlineKind::Emoji(_)
                | InlineKind::LineBreak => {}
            }
        }
    }
}

/// Pairs each `[` with its `]`, skipping backslash-escaped brackets.
fn match_brackets(units: &[Unit]) -> HashMap<usize, usize> {
    let mut pairs = HashMap::new();
    let mut open = Vec::new();
    let mut i = 0;
    while i < units.len() {
        match units[i].ch {
            '\\' if units.get(i + 1).is_some_and(|u| u.ch.is_ascii_punctuation()) => i += 1,
            '[' => open.push(i),
            ']' => {
                if let Some(start) = open.pop() {
                    pairs.insert(start, i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    pairs
}

/// Pairs each `(` with the `)` that balances it, never across whitespace (a
/// line break is whitespace too): a URL holds no whitespace, so a `(` with no
/// partner before the next whitespace cannot start one.
fn match_url_parens(units: &[Unit]) -> HashMap<usize, usize> {
    let mut pairs = HashMap::new();
    let mut open = Vec::new();
    for (i, unit) in units.iter().enumerate() {
        match unit.ch {
            '(' => open.push(i),
            ')' => {
                if let Some(start) = open.pop() {
                    pairs.insert(start, i);
                }
            }
            c if c.is_whitespace() => open.clear(),
            _ => {}
        }
    }
    pairs
}

/// Flattens the lines into units. Leading whitespace of each line is dropped;
/// trailing whitespace too, after deciding whether the line ends in a hard
/// break (two or more trailing spaces or tabs, or one backslash that is not
/// itself escaped: `a\\` ends in a literal backslash and a soft break).
fn units(text: &str, segs: &[Seg]) -> Vec<Unit> {
    let mut units = Vec::new();
    for (n, seg) in segs.iter().enumerate() {
        let last = n + 1 == segs.len();
        let line = &text[seg.start..seg.end];
        let body = line.trim_start_matches([' ', '\t']);
        let start = seg.end - body.len();
        let trimmed = body.trim_end_matches([' ', '\t']);
        let mut end = start + trimmed.len();
        let mut hard = body.len() - trimmed.len() >= 2;
        let backslashes = trimmed.bytes().rev().take_while(|&b| b == b'\\').count();
        if !last && !hard && body.len() == trimmed.len() && backslashes % 2 == 1 {
            hard = true;
            end -= 1;
        }
        units.extend(text[start..end].char_indices().map(|(at, ch)| Unit {
            start: start + at,
            end: start + at + ch.len_utf8(),
            ch,
            hard: false,
        }));
        if !last {
            units.push(Unit {
                start: end,
                end: seg.eol.max(end),
                ch: '\n',
                hard,
            });
        }
    }
    units
}
