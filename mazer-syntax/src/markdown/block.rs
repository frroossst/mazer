//! Block structure: which lines form headings, paragraphs, lists, quotes and
//! fences. Inline content is handed to [`super::inline`].

use super::{inline, is_blank, is_form_start, read_code};
use crate::diag::{DiagCode, Diags};
use crate::doc::{Block, BlockKind, Inline, InlineKind, ListItem};
use crate::node::Node;
use crate::reader::Seg;
use crate::span::{ByteOffset, FileId, Span};

/// How deeply quotes and lists may nest before markers are read as text.
const MAX_DEPTH: usize = 32;

/// Read-only view of the note with the line-level helpers. `Copy`, so closures
/// can capture it while the parser itself is borrowed mutably.
#[derive(Clone, Copy)]
struct Src<'a> {
    text: &'a str,
    file: FileId,
}

struct Fence {
    ch: u8,
    len: usize,
    indent: usize,
    lang: Option<Box<str>>,
}

#[derive(Clone, Copy)]
struct Marker {
    /// `Some(start)` for a numbered item.
    ordered: Option<u32>,
    checked: Option<bool>,
    /// Byte offset of the marker itself.
    at: usize,
    /// Byte offset where the item's content starts.
    content: usize,
    /// Column (relative to the line's start) where content starts.
    content_col: usize,
}

pub(super) struct BlockParser<'a, 'd> {
    src: Src<'a>,
    diags: &'d mut Diags,
}

impl<'a, 'd> BlockParser<'a, 'd> {
    pub(super) const fn new(text: &'a str, file: FileId, diags: &'d mut Diags) -> Self {
        Self {
            src: Src { text, file },
            diags,
        }
    }

    /// Parses `lines` (already stripped of any container prefix) into blocks.
    /// `depth` is 0 for the document itself.
    pub(super) fn blocks(&mut self, lines: &[Seg], depth: usize) -> Vec<Block> {
        let src = self.src;
        let nest = depth < MAX_DEPTH;
        let mut out = Vec::new();
        let mut quote_view: Option<Vec<Seg>> = None;
        // Lines before `free` were already read as part of an unclosed form
        // that started in an earlier quote or list item; container scans do
        // not search them for forms again (see `Src::collect`).
        let mut free = 0;
        let mut i = 0;
        while i < lines.len() {
            let line = lines[i];
            i = if src.is_blank(line) {
                i + 1
            } else if let Some(fence) = src.fence_open(line) {
                self.fenced(lines, i, fence, &mut out)
            } else if let Some((level, at, content)) = src.heading(line) {
                self.heading(lines, i, level, at, content, &mut out)
            } else if src.is_rule(line) {
                let (_, at) = src.indent(line);
                out.push(Block::new(BlockKind::Rule, src.span(at, src.trim_end(line))));
                i + 1
            } else if nest && src.quote_marker(line).is_some() {
                let view = quote_view.get_or_insert_with(|| lines.iter().map(|&l| src.strip_quote(l)).collect());
                self.quote(lines, view, i, depth, &mut free, &mut out)
            } else if let Some(marker) = src.list_marker(line).filter(|_| nest) {
                self.list(lines, i, marker, depth, &mut free, &mut out)
            } else if let Some(image) = src.image_line(line) {
                out.push(image);
                i + 1
            } else {
                self.paragraph(lines, i, depth, &mut out)
            };
        }
        out
    }

    fn inlines(&mut self, segs: &[Seg]) -> Vec<Inline> {
        inline::parse(self.src.text, self.src.file, segs, self.diags)
    }

    fn fenced(&self, lines: &[Seg], i: usize, fence: Fence, out: &mut Vec<Block>) -> usize {
        let src = self.src;
        let (_, at) = src.indent(lines[i]);
        let mut end = lines[i].end;
        let mut code = String::new();
        let mut j = i + 1;
        while j < lines.len() {
            let line = lines[j];
            j += 1;
            end = line.end;
            if src.closes_fence(line, &fence) {
                break;
            }
            let content = src.strip_cols(line, fence.indent);
            code.push_str(&src.text[content.start..content.end]);
            code.push('\n');
        }
        let kind = BlockKind::CodeBlock {
            lang: fence.lang,
            code: code.into(),
        };
        out.push(Block::new(kind, src.span(at, end)));
        j
    }

    fn heading(&mut self, lines: &[Seg], i: usize, level: u8, at: usize, first: Seg, out: &mut Vec<Block>) -> usize {
        let src = self.src;
        let (_, last) = src.extent(lines, i, first.start);
        let mut segs = vec![first];
        segs.extend_from_slice(&lines[i + 1..=last]);
        if let Some(seg) = segs.last_mut() {
            seg.end = src.strip_closing_hashes(*seg);
        }
        let end = segs.last().map_or(first.start, |&s| src.trim_end(s)).max(at);
        let content = self.inlines(&segs);
        out.push(Block::new(BlockKind::Heading { level, content }, src.span(at, end)));
        last + 1
    }

    fn paragraph(&mut self, lines: &[Seg], i: usize, depth: usize, out: &mut Vec<Block>) -> usize {
        let src = self.src;
        let (_, mut last) = src.extent(lines, i, lines[i].start);
        while let Some(&next) = lines.get(last + 1) {
            if src.is_blank(next) || src.interrupts(next, depth) {
                break;
            }
            (_, last) = src.extent(lines, last + 1, next.start);
        }
        let segs = &lines[i..=last];
        let (_, start) = src.indent(lines[i]);
        let span = src.span(start, src.trim_end(lines[last]).max(start));
        let content = self.inlines(segs);
        let kind = match sole_code(&content) {
            Some(node) if depth == 0 => BlockKind::Code(node.clone()),
            _ => BlockKind::Paragraph(content),
        };
        out.push(Block::new(kind, span));
        last + 1
    }

    fn quote(
        &mut self,
        lines: &[Seg],
        view: &[Seg],
        i: usize,
        depth: usize,
        free: &mut usize,
        out: &mut Vec<Block>,
    ) -> usize {
        let src = self.src;
        let (count, next_free) = src.collect(&view[i..], view[i].start, free.saturating_sub(i), |t| {
            src.quote_marker(lines[i + t]).is_some()
        });
        *free = i + next_free;
        let inner = &view[i..i + count];
        let content = self.blocks(inner, depth + 1);
        let at = src.quote_marker(lines[i]).unwrap_or(lines[i].start);
        let end = src.trim_end(lines[i + count - 1]).max(at);
        out.push(Block::new(BlockKind::BlockQuote(content), src.span(at, end)));
        i + count
    }

    fn list(
        &mut self,
        lines: &[Seg],
        i: usize,
        first: Marker,
        depth: usize,
        free: &mut usize,
        out: &mut Vec<Block>,
    ) -> usize {
        let src = self.src;
        let mut items = Vec::new();
        let mut marker = first;
        let mut j = i;
        loop {
            let m = marker;
            let rest = &lines[j..];
            let (count, next_free) = src.collect(rest, m.content, free.saturating_sub(j), |t| {
                src.is_blank(rest[t]) || src.indent(rest[t]).0 >= m.content_col
            });
            *free = j + next_free;
            let mut inner = Vec::with_capacity(count);
            inner.push(Seg::new(m.content, rest[0].end, rest[0].eol));
            inner.extend(rest[1..count].iter().map(|&l| src.strip_cols(l, m.content_col)));
            let content = self.blocks(&inner, depth + 1);
            let end = src.trim_end(rest[count - 1]).max(m.content);
            items.push(ListItem {
                checked: m.checked,
                content,
                span: src.span(m.at, end),
            });
            j += count;

            let mut k = j;
            while k < lines.len() && src.is_blank(lines[k]) {
                k += 1;
            }
            match lines.get(k).and_then(|&l| src.list_marker(l)) {
                Some(next) if next.ordered.is_some() == first.ordered.is_some() && !src.is_rule(lines[k]) => {
                    marker = next;
                    j = k;
                }
                _ => break,
            }
        }
        let end = items.last().map_or(first.content, |it| it.span.end.to_usize());
        let kind = BlockKind::List {
            ordered: first.ordered,
            items,
        };
        out.push(Block::new(kind, src.span(first.at, end)));
        j
    }
}

/// The form of a paragraph whose only content (ignoring whitespace) is one
/// reserved-head form. `ref` is inline only, so a lone `(ref x)` stays in its
/// paragraph.
fn sole_code(content: &[Inline]) -> Option<&Node> {
    let mut code = None;
    for inline in content {
        match &inline.kind {
            InlineKind::Code(node) if code.is_none() && !node.is_form("ref") => code = Some(node),
            InlineKind::Text(text) if text.trim().is_empty() => {}
            InlineKind::LineBreak => {}
            _ => return None,
        }
    }
    code
}

impl Src<'_> {
    fn span(self, start: usize, end: usize) -> Span {
        Span::new(self.file, ByteOffset::from_usize(start), ByteOffset::from_usize(end))
    }

    fn is_blank(self, line: Seg) -> bool {
        is_blank(self.text, line)
    }

    /// End of the line with trailing whitespace removed.
    fn trim_end(self, line: Seg) -> usize {
        line.start + self.text[line.start..line.end].trim_end_matches([' ', '\t']).len()
    }

    /// Leading whitespace of a line: its width in columns and the byte offset
    /// of the first other character.
    const fn indent(self, line: Seg) -> (usize, usize) {
        let bytes = self.text.as_bytes();
        let mut col = 0;
        let mut at = line.start;
        while at < line.end {
            match bytes[at] {
                b' ' => col += 1,
                b'\t' => col += 4 - col % 4,
                _ => break,
            }
            at += 1;
        }
        (col, at)
    }

    /// Removes up to `cols` columns of leading whitespace.
    const fn strip_cols(self, line: Seg, cols: usize) -> Seg {
        let bytes = self.text.as_bytes();
        let mut col = 0;
        let mut at = line.start;
        while at < line.end && col < cols {
            match bytes[at] {
                b' ' => col += 1,
                b'\t' => col += 4 - col % 4,
                _ => break,
            }
            at += 1;
        }
        Seg::new(at, line.end, line.eol)
    }

    fn fence_open(self, line: Seg) -> Option<Fence> {
        let bytes = self.text.as_bytes();
        let (indent, at) = self.indent(line);
        let ch = *bytes.get(at).filter(|_| at < line.end)?;
        if ch != b'`' && ch != b'~' {
            return None;
        }
        let len = bytes[at..line.end].iter().take_while(|&&b| b == ch).count();
        if len < 3 {
            return None;
        }
        let info = self.text[at + len..line.end].trim();
        if ch == b'`' && info.contains('`') {
            return None;
        }
        Some(Fence {
            ch,
            len,
            indent,
            lang: info.split_whitespace().next().map(Into::into),
        })
    }

    fn closes_fence(self, line: Seg, fence: &Fence) -> bool {
        let (_, at) = self.indent(line);
        let rest = &self.text.as_bytes()[at..line.end];
        let len = rest.iter().take_while(|&&b| b == fence.ch).count();
        len >= fence.len && rest[len..].iter().all(|&b| b == b' ' || b == b'\t')
    }

    /// `#`..`######` then a space, tab or end of line: the level, the offset
    /// of the first `#` and the content.
    fn heading(self, line: Seg) -> Option<(u8, usize, Seg)> {
        let bytes = self.text.as_bytes();
        let (_, at) = self.indent(line);
        let hashes = bytes[at..line.end].iter().take_while(|&&b| b == b'#').count();
        let level = u8::try_from(hashes).ok().filter(|l| (1..=6).contains(l))?;
        let after = at + hashes;
        if after < line.end && !matches!(bytes[after], b' ' | b'\t') {
            return None;
        }
        let (_, content) = self.indent(Seg::new(after, line.end, line.eol));
        Some((level, at, Seg::new(content, line.end, line.eol)))
    }

    /// The end of a heading line without a closing `###` sequence.
    fn strip_closing_hashes(self, seg: Seg) -> usize {
        let body = self.text[seg.start..seg.end].trim_end_matches([' ', '\t']);
        let open = body.trim_end_matches('#');
        if open.len() < body.len() && (open.is_empty() || open.ends_with([' ', '\t'])) {
            seg.start + open.trim_end_matches([' ', '\t']).len()
        } else {
            seg.start + body.len()
        }
    }

    fn is_rule(self, line: Seg) -> bool {
        let (_, at) = self.indent(line);
        let rest = &self.text.as_bytes()[at..line.end];
        let Some(&ch) = rest.first().filter(|&&c| matches!(c, b'-' | b'*' | b'_')) else {
            return false;
        };
        let mut marks = 0;
        for &b in rest {
            match b {
                b' ' | b'\t' => {}
                _ if b == ch => marks += 1,
                _ => return false,
            }
        }
        marks >= 3
    }

    fn quote_marker(self, line: Seg) -> Option<usize> {
        let (_, at) = self.indent(line);
        (at < line.end && self.text.as_bytes()[at] == b'>').then_some(at)
    }

    /// The line without its `>` and one following space or tab; lines without
    /// a `>` are returned unchanged.
    fn strip_quote(self, line: Seg) -> Seg {
        let Some(at) = self.quote_marker(line) else {
            return line;
        };
        let mut start = at + 1;
        if start < line.end && matches!(self.text.as_bytes()[start], b' ' | b'\t') {
            start += 1;
        }
        Seg::new(start, line.end, line.eol)
    }

    fn list_marker(self, line: Seg) -> Option<Marker> {
        let bytes = self.text.as_bytes();
        let (_, at) = self.indent(line);
        let end = line.end;
        let space_or_end = |p: usize| p >= end || matches!(bytes[p], b' ' | b'\t');
        let check = |p: usize| match bytes.get(p..p + 3).filter(|_| p + 3 <= end) {
            Some(b"[ ]") => Some(false),
            Some(b"[x]" | b"[X]") => Some(true),
            _ => None,
        };
        let first = *bytes.get(at).filter(|_| at < end)?;
        let (ordered, mut checked, after) = match first {
            // Old checklist syntax: `-[ ]` with no space after the dash.
            b'-' if check(at + 1).is_some() => (None, check(at + 1), at + 4),
            b'-' | b'*' | b'+' if space_or_end(at + 1) => (None, None, at + 1),
            b'0'..=b'9' => {
                let digits = bytes[at..end].iter().take_while(|b| b.is_ascii_digit()).count();
                let delim = at + digits;
                if digits > 9 || delim >= end || !matches!(bytes[delim], b'.' | b')') || !space_or_end(delim + 1) {
                    return None;
                }
                let start = self.text[at..delim].parse().ok()?;
                (Some(start), None, delim + 1)
            }
            _ => return None,
        };
        let mut content = self.skip_spaces(after, end);
        // GitHub checklist syntax: `- [ ] task`.
        if ordered.is_none()
            && checked.is_none()
            && content > after
            && let Some(state) = check(content).filter(|_| space_or_end(content + 3))
        {
            checked = Some(state);
            content = self.skip_spaces(content + 3, end);
        }
        let content_col = if content >= end {
            self.column(line, after) + 1
        } else {
            self.column(line, content)
        };
        Some(Marker {
            ordered,
            checked,
            at,
            content,
            content_col,
        })
    }

    const fn skip_spaces(self, mut at: usize, end: usize) -> usize {
        while at < end && matches!(self.text.as_bytes()[at], b' ' | b'\t') {
            at += 1;
        }
        at
    }

    /// Column of byte `at` within `line`, counting tabs to multiples of 4.
    fn column(self, line: Seg, at: usize) -> usize {
        self.text[line.start..at].chars().fold(0, |col, c| if c == '\t' { col + 4 - col % 4 } else { col + 1 })
    }

    /// A line holding only `![alt](url)`.
    fn image_line(self, line: Seg) -> Option<Block> {
        let (_, at) = self.indent(line);
        let end = self.trim_end(line);
        let body = &self.text[at..end];
        if !body.starts_with("![") || !body.ends_with(')') {
            return None;
        }
        let mut scratch = Diags::new();
        let inlines = inline::parse(self.text, self.file, &[Seg::new(at, end, line.eol)], &mut scratch);
        match inlines.as_slice() {
            [Inline {
                kind: InlineKind::Image { alt, url },
                span,
            }] => Some(Block::new(
                BlockKind::Image {
                    alt: alt.clone(),
                    url: url.clone(),
                },
                *span,
            )),
            _ => None,
        }
    }

    /// Whether `line` starts a block that ends a paragraph.
    fn interrupts(self, line: Seg, depth: usize) -> bool {
        let nest = depth < MAX_DEPTH;
        self.fence_open(line).is_some()
            || self.heading(line).is_some()
            || self.is_rule(line)
            || (nest && self.quote_marker(line).is_some())
            || (nest
                && self
                    .list_marker(line)
                    .is_some_and(|m| m.content < line.end && m.ordered.is_none_or(|n| n == 1)))
            || self.image_line(line).is_some()
    }

    /// Index of the last line covered by the text that starts on line `i` at
    /// byte `from`: line `i` itself, unless a code form starting on it runs
    /// onto later lines (and so on, if a form ends on a line where another
    /// one starts). A form never runs past a blank line, so neither does this.
    ///
    /// Returns `(closed, reach)`: `closed` is the last line covered by closed
    /// forms, `reach` the last line read at all. They differ only when the
    /// last form found is unclosed; it then starts on line `closed` and runs
    /// to the line before the next blank line (or the last of `lines`).
    fn extent(self, lines: &[Seg], i: usize, from: usize) -> (usize, usize) {
        let mut k = i;
        let mut pos = from;
        while let Some(at) = self.find_form(lines[k], pos) {
            let (read, diags) = read_code(self.text, self.file, &lines[k..], at);
            let end = read.end.to_usize().max(at + 1);
            let last = (k + lines[k..].partition_point(|l| l.end < end)).min(lines.len() - 1);
            if diags.iter().any(|d| d.code == DiagCode::UnclosedParen) {
                return (k, last);
            }
            k = last;
            pos = end.max(lines[k].start);
        }
        (k, k)
    }

    /// The first code form on `line` at or after `from`, skipping escaped
    /// characters and code spans closed on the same line.
    fn find_form(self, line: Seg, from: usize) -> Option<usize> {
        let bytes = self.text.as_bytes();
        let mut at = from;
        while at < line.end {
            match bytes[at] {
                b'\\' => at += 2,
                b'`' => {
                    let run = bytes[at..line.end].iter().take_while(|&&b| b == b'`').count();
                    at = closing_ticks(&bytes[..line.end], at + run, run).map_or(at + run, |close| close + run);
                }
                b'(' if is_form_start(self.text, at) => return Some(at),
                _ => at += 1,
            }
        }
        None
    }

    /// How many lines, starting with `view[0]`, belong to a container (quote
    /// or list item). `belongs(t)` says whether line `t` would belong on its
    /// own. Lines covered by a *closed* code form that started inside the
    /// container belong regardless; an unclosed form does not stretch the
    /// container, so it ends where the container's own lines end (or at a
    /// blank line). Code inside a fence is never searched for forms.
    /// Trailing blank lines are left out.
    ///
    /// Forms are searched for only on lines from index `free` on: the lines
    /// before it were already read by an unclosed form that started earlier.
    /// Returns the line count and the new `free`, so that no line is read
    /// over and over by one unclosed form after another.
    fn collect(self, view: &[Seg], first_start: usize, free: usize, belongs: impl Fn(usize) -> bool) -> (usize, usize) {
        let first = Seg::new(first_start, view[0].end, view[0].eol);
        let mut fence = self.fence_open(first);
        let mut free = free;
        // Lines up to `covered` are inside a closed form; lines up to
        // `inside` are inside an unclosed one, so nothing starts on them.
        let (mut covered, mut inside) = (0, 0);
        if fence.is_none() && free == 0 {
            (covered, inside) = self.extent(view, 0, first_start);
            free = free.max(inside + 1);
        }
        let mut last = covered;
        let mut t = covered + 1;
        while t < view.len() {
            let line = view[t];
            if t <= covered {
                last = t;
            } else if !belongs(t) {
                break;
            } else if self.is_blank(line) {
                // Belongs only if something non-blank follows.
            } else {
                last = t;
                if t <= inside {
                    // Part of an unclosed form: no fence or form starts here.
                } else if let Some(open) = &fence {
                    if self.closes_fence(line, open) {
                        fence = None;
                    }
                } else if let Some(open) = self.fence_open(line) {
                    fence = Some(open);
                } else if t >= free {
                    (covered, inside) = self.extent(view, t, line.start);
                    free = free.max(inside + 1);
                    last = covered;
                }
            }
            t += 1;
        }
        (last + 1, free)
    }
}

/// Start of the next run of exactly `run` backticks at or after `from`.
pub(super) fn closing_ticks(bytes: &[u8], from: usize, run: usize) -> Option<usize> {
    let mut at = from;
    while at < bytes.len() {
        if bytes[at] == b'`' {
            let len = bytes[at..].iter().take_while(|&&b| b == b'`').count();
            if len == run {
                return Some(at);
            }
            at += len;
        } else {
            at += 1;
        }
    }
    None
}
