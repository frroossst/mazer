# Mazer architecture

This document is the agreed design for the Mazer rewrite. The public types and function signatures in the skeleton crates (`mazer-syntax`, `mazer-math`, `mazer-doc`, `mazer-lsp`) are a contract: an implementer who believes one is wrong stops and reports it instead of changing it. Where this document and the skeleton code differ, the skeleton is authoritative.

## Agreed design

### Crate layout

```
mazer-syntax   Span, SourceMap, Diag, Sym, Node + reader, Block/Inline + markdown parser   (no dependencies)
mazer-math     Keyword registry, DisplayName table, MathMl emitter, Latex emitter (feature `latex`)
mazer-doc      alias/macro expansion, let substitution, labels, analyze/render/compile, locate, HTML output, Evaluator trait
mazer-cli / mazer-wasm / mazer-lsp     front ends; each depends only on mazer-doc
mazer-eval     later; implements Evaluator
```
- Heavy dependencies live only in front-end crates. No proc macros, no serde, no miette in the libraries.
- `mazer-dbg` is left completely untouched; the workspace gets `default-members` excluding it so plain `cargo build`/`cargo test` do not compile egui.
- Removed: mazer-parser, mazer-lisp, mazer-types, mazer-stdlib, mazer-macros, mazer-atog, mazer-html, mazer-render, mazer-units (and fastnum, strum, serde, syn/quote, miette).

### Source, spans, diagnostics (`mazer-syntax`)

```rust
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ByteOffset(u32);          // newtype, not alias: must not mix with UTF-16 columns
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct FileId(u16);              // 0 = the note, 1 = embedded stdlib.zr, ...
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Span { pub file: FileId, pub start: ByteOffset, pub end: ByteOffset }  // end exclusive
// Span: len(), contains(ByteOffset), to(Span), range()

pub struct SourceFile { pub name: String, pub text: String, line_starts: Vec<ByteOffset> }
pub struct SourceMap { files: Vec<SourceFile> }      // indexed by FileId; owns all text, trees never borrow
pub struct Line(u32);  pub struct ByteCol(u32);  pub struct Utf16Col(u32);   // Copy newtypes
// SourceMap: add(name, text) -> FileId, replace(file, text), text(span) -> &str,
//            line_col, line_col_utf16, offset_at(file, Line, Utf16Col) -> ByteOffset

pub enum Severity { Error, Warning, Hint }
pub enum DiagCode { UnknownForm, Arity, UnclosedParen, UnresolvedRef, /* grows per stage */ }
pub struct Diag {
    pub severity: Severity,
    pub code: DiagCode,
    pub message: String,
    pub span: Span,
    pub labels: Vec<(Span, String)>,   // secondary locations
    pub help: Option<String>,          // e.g. "did you mean `integral`?"
}
pub struct Diags(Vec<Diag>);           // sink passed down the pipeline; stages push and continue, never abort
```
- LSP edits replace the whole file and re-parse from scratch (no incremental parsing).
- `Diag` is plain data. CLI renders it with `annotate-snippets`, LSP converts to LSP diagnostics, wasm/HTML shows an inline error box.

### S-expression tree and reader (`mazer-syntax`)

```rust
pub struct Node { pub kind: NodeKind, pub span: Span }      // syntax only; no runtime values. Replaces LispAST.
pub enum NodeKind {
    Ident(Sym),            // any name: x, tau, Laplace, integral. Meaning is not known to the reader.
    Number(Box<str>),      // source text as written; never parsed in core
    Str(Box<str>),
    List(Vec<Node>),       // owned tree (not an arena); head is simply the first child
    Error,                 // reader recovery placeholder, always paired with a Diag
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Sym(u32);       // GLOBAL interner (rustc-style): Sym::new(&str), sym.as_str(); Debug prints the name
```
Reader rules:
- `(` `)` are the only delimiters; a paren always ends an atom.
- Number: starts with a digit, or `-` directly followed by a digit; optional fraction and exponent; stored as written.
- `-name` reads as `(- name)` only when the `-` is directly followed by an alphabetic character (a Unicode letter). Any other atom is an ordinary identifier: `->`, `-->`, `--`, `-` alone, `->>` stay single identifiers; `-3` and `-3.5e2` are numbers.
- `'x` reads as `(quote x)`.
- `"..."` is a `Str` node, escapes `\"` and `\\`. New; verify no fixture note has a `"` inside a show block.
- `;` to end of line is a comment, inside code only.
- Unclosed `(` gives an `Error` node plus one diagnostic at the outermost unclosed opening paren of the form; a stray `)` in prose is prose.

Names:
- Only two kinds of name: **keywords** (operations with arguments, defined in Rust) and **variables** (everything else: `x`, `tau`, `Laplace`).
- `tau` is an ordinary variable, also to the evaluator. Showing it as `τ` is a display table owned by the renderer.
- A typed `τ` and the name `tau` are different variables that happen to render the same.
- No "glyph" category and no separate resolved-name tree.

Reserved heads (the only parentheses that are code): `show`, `eval`, `let`, `alias`, `macro`, and inline `ref`.

### Document tree (`mazer-syntax`)

```rust
pub struct Document { pub blocks: Vec<Block> }
pub struct Block { pub kind: BlockKind, pub span: Span }
pub enum BlockKind {
    Heading   { level: u8, content: Vec<Inline> },             // rich content allowed in headings
    Paragraph (Vec<Inline>),
    List      { ordered: Option<u32>, items: Vec<ListItem> },  // Some(start) = numbered
    BlockQuote(Vec<Block>),
    CodeBlock { lang: Option<Box<str>>, code: Box<str> },
    Image     { alt: Box<str>, url: Box<str> },
    Code      (Node),     // reserved-head form alone in its paragraph: display equation, can be tagged
    Rule,                 // ---
}
pub struct ListItem {
    pub checked: Option<bool>,   // None = bullet, Some(false) = -[ ], Some(true) = -[x]
    pub content: Vec<Block>,     // blocks, so lists nest
    pub span: Span,
}

pub struct Inline { pub kind: InlineKind, pub span: Span }
pub enum InlineKind {
    Text      (Box<str>),
    Bold      (Vec<Inline>),          // all styles nest
    Italic    (Vec<Inline>),
    Underline (Vec<Inline>),
    Strike    (Vec<Inline>),
    Spoiler   (Vec<Inline>),
    Link      { content: Vec<Inline>, url: Box<str> },
    Image     { alt: Box<str>, url: Box<str> },
    InlineCode(Box<str>),             // never interpreted
    Emoji     (Sym),                  // shortcode name; unknown shortcodes stay Text
    Code      (Node),                 // reserved-head form inside a sentence
    LineBreak,
}
```
- No markdown pipe-table syntax; tables are the Lisp `(table ...)` keyword only.
- New vs today: ordered lists, nested lists, images, checklists as real list items, nestable styles, emoji.
- Emoji table is exactly 13 shortcodes: `:sob:` `:maple:` `:heart:` `:eyes:` `:thinking:` `:+1:` `:-1:` `:brain:` `:fire:` `:sparkles:` `:rocket:` `:coffee:` `:sunglasses:`.
- Keep today's rule: `*` directly after a letter or digit is a literal star.

### Keywords and emitters (`mazer-math`)

```rust
pub struct Keyword {
    pub name: &'static str,
    pub aliases: Vec<&'static str>,             // second names: + / add, lim / limit, ...
    pub doc: &'static str,                      // LSP hover, `mazer query`
    pub params: Vec<Param>,                     // argument checking + signature help
    pub examples: Vec<&'static str>,            // each rendered in a test
    pub sign: Option<&'static str>,             // what the bare keyword renders as (integral -> ∫); None = bare use is a diagnostic
    pub mathml: fn(&Call, &mut MathMl),         // mandatory; emitters push Diags, never return errors
    #[cfg(feature = "latex")]
    pub latex: Option<fn(&Call, &mut Latex)>,
}
pub struct Param { pub name: &'static str, pub doc: &'static str, pub kind: ParamKind }
pub enum ParamKind { Required, Optional, Rest }

impl Keyword {
    pub fn new(name: &'static str, doc: &'static str, mathml: fn(&Call, &mut MathMl)) -> Self;  // the 3 required things
    pub fn alias(self, name) -> Self;
    pub fn param(self, name, doc) -> Self;      // required
    pub fn optional(self, name, doc) -> Self;   // only after the required ones
    pub fn rest(self, name, doc) -> Self;       // must be last
    pub fn example(self, src) -> Self;
    pub fn sign(self, s) -> Self;
    #[cfg(feature = "latex")] pub fn latex(self, f: fn(&Call, &mut Latex)) -> Self;
}
static KEYWORDS: LazyLock<Registry> = LazyLock::new(|| Registry::from(vec![ /* Keyword::new(..).param(..) ... */ ]));

pub struct Call<'a> {
    pub keyword: &'a Keyword,
    pub span: Span,
    bound: Vec<Option<&'a Node>>,     // one slot per declared parameter
    rest: &'a [Node],
}
// Call: req(i) -> &Node, opt(i) -> Option<&Node>, rest() -> &[Node]   (i = PARAMETER position)

pub struct MathMl<'a> { out: String, diags: &'a mut Diags }   // writes straight into a String; no intermediate tree
// MathMl: expr(&Node)                         recursive dispatcher
//         tag(name, |o| ..), tag_attrs(..)    closure guarantees the closing tag
//         mi / mn / mo / mtext                the ONLY places text is escaped
//         row, frac, sub, sup, fence          shorthand
//         error(Diag)                         pushes the Diag and writes a visible <merror>

pub struct DisplayName {                       // renderer-only; the evaluator never reads it. Replaces mazer-atog.
    pub name: &'static str,       // "tau"
    pub unicode: &'static str,    // "τ"
    pub doc: &'static str,        // mandatory
    pub class: Class,
    #[cfg(feature = "latex")] pub latex: &'static str,   // "\\tau"
}
pub enum Class { Letter /* <mi> */, Operator /* <mo>, spaced */ }
// DisplayName::letter(name, unicode, doc) / DisplayName::operator(name, unicode, doc), then .latex("\\tau")
```
- Registry is built once at startup (LazyLock). LaTeX emitters are registered in a separate feature-gated file.
- Keyword definitions live in `mazer-math/src/keywords/{algebra,calculus,notation}.rs`; each file exposes `keywords() -> Vec<Keyword>`.
- Binding: required parameters first, extra arguments fill optionals left to right; wrong count is a diagnostic and the emitter is not called.
- `expr` dispatch: Number -> `<mn>`; Ident -> display table, else itself; List headed by a keyword -> that emitter.
- List headed by a non-keyword is **function application**: `(gen (Gamma) tau)` -> gen(Γ, τ); zero arguments, e.g. `(Gamma)`, renders just the variable. Head one typo from a keyword adds a did-you-mean hint (see decision 6 of step 1).
- `Latex` mirrors `MathMl` method names where they make sense.
- The built-in display-name table must be comprehensive (`hbar` and other common physics/math names). Notes add rare extras with `alias`.
- Tests: `rest` is last, optionals after required, no duplicate names, no keyword/display-name clash, every keyword has an example that renders without diagnostics; a report lists keywords and display names lacking LaTeX.

Keyword inventory (today's list: `mazer-types/src/implfuncs.rs`; today's output: `mazer-render/src/lib.rs`):
- **Carried over**, with their second names: arithmetic (`+ - * / ^ frac sqrt root jux`), comparison (`= != < > <= >= approx`), calculus (`integral sum prod limit derivative partial dd evalat`), functions (`sin cos tan cot sec csc arcsin arccos arctan ln log exp abs floor ceil fact binom`), linear algebra (`matrix vec det`), sets/logic (`set in notin subset superset union intersect and or not implies iff forall exists`), grouping (`paren bracket brace`), decoration (`text subscript superscript bar hat dot ddot arrow box prime funcapp`), `table`, `ty-judge`.
- **Removed:** `define`, `defunc` (replaced by `let`, `macro`) and `string` (all 31 uses in the notes are rewritten: `(string w)` -> `w`, `(string f(x))` -> `(f x)`).
- **New:** `quote`; font variants `bb`, `cal`, `frak`, `bold`, `rm`; `cases`.
- **Calculus argument order changes:** `(integral body var)` / `(integral body var lower upper)`; same for `sum` and `prod`. Variable always required, bounds optional at the end.

### Notation: alias and macro (`mazer-doc`)

```lisp
(alias R (bb R))                                   ; alias = macro with no parameters
(macro (laplace f s) "optional doc string" (jux Laplace (brace f) (paren s)))
```
```rust
pub struct Macro {
    pub name: Sym,
    pub params: Vec<Sym>,          // empty for an alias
    pub rest: Option<Sym>,
    pub doc: Option<Box<str>>,
    pub body: Node,                // template
    pub span: Span,                // definition site
}
pub struct Macros { by_name: HashMap<Sym, Macro> }
pub fn expand(node: &Node, macros: &Macros, diags: &mut Diags) -> Node;
```
- Pure lazy tree substitution; arguments are substituted exactly as written; nothing is evaluated.
- Results are re-expanded so macros can use macros; a name is not re-expanded inside its own expansion; depth cap 64, overflow is a diagnostic.
- Scope is the whole note: definitions from the embedded `stdlib.zr`, then the note, are collected before any expansion. Duplicate name is a diagnostic.
- Reusing a built-in keyword or display name is an error.
- Expanded nodes take the use-site span; the definition span is a secondary label.

### Variables: let and quote (`mazer-doc`)

`let` is a real variable binding and is different from alias.

```lisp
(let x 2)
(show x)          ; 2
(show (+ x 1))    ; 2 + 1      show substitutes bound variables everywhere
(eval (+ x 1))    ; 3
(let y (+ x 1))   ; y holds (+ 2 1): captured at the let
(let x 3)         ; reassignable any number of times; document order
(show (= 'x x))   ; x = 3      quote shows the name itself
```
```rust
pub struct Binding { pub name: Sym, pub value: Node /* unevaluated */, pub span: Span }
pub struct Bindings { by_name: HashMap<Sym, Binding> }
```
- The core owns the table, so `let` and `show` work with no evaluator.
- One pass in document order (after macro expansion, inside `analyze`) substitutes variables into every `let`, `show` and `eval`. Afterwards trees contain no bound variables.
- `(quote x)` / `'x` is never substituted.

### Equation tags and references (`mazer-doc`)

```lisp
(show (tag laplace) (= (laplace f s) (exp (* (- a) s))))     ; numbered display equation
As shown in (ref laplace), ...                                 ; renders "(1)" linking to it
```
```rust
pub struct Label { pub name: Sym, pub number: u32, pub anchor: Box<str> /* "eq-laplace" */, pub span: Span }
pub struct Labels { by_name: HashMap<Sym, Label> }
pub fn collect_labels(doc: &Document, diags: &mut Diags) -> Labels;
```
- Only explicitly tagged equations are numbered. `(tag name)` is valid only as the first item of a display `show`.
- Collected before rendering so forward references work; numbers follow document order.
- Diagnostics: unknown ref, duplicate tag, tag on an inline show.
- Boxing is independent: the `box` keyword, on a whole equation or a sub-expression.
- LaTeX mapping: `\label` / `\eqref`.

### Pipeline and evaluation interface (`mazer-doc`)

```rust
pub struct Options {
    pub name: String,
    pub target: Target,
    pub page: Page,
    pub evaluator: Option<Box<dyn Evaluator>>,     // None in the website build
}
pub enum Target { MathMl, #[cfg(feature = "latex")] Latex }
pub enum Page { Full { title: String }, Fragment }

pub struct Analysis {
    pub sources: SourceMap,
    pub written: Document,     // exactly as typed: LSP queries
    pub resolved: Document,    // macros expanded, let-variables substituted: render
    pub macros: Macros,
    pub labels: Labels,
    pub lets: Vec<Binding>,    // every `let`, in document order; used by the LSP
    pub diags: Diags,
}
pub fn analyze(src: &str, name: &str) -> Analysis;            // LSP calls only this, on every edit
pub fn render(analysis: &Analysis, opts: &mut Options, diags: &mut Diags) -> String;   // Options is mutable: the evaluator is called mutably

pub struct Output { pub html: String, pub diags: Vec<Diag>, pub sources: SourceMap }
pub fn compile(src: &str, opts: &mut Options) -> Output;      // analyze + render; never fails

pub trait Evaluator {
    fn eval(&mut self, expr: &Node, diags: &mut Diags) -> Option<Node>;   // result is another Node, rendered by the normal emitters
}

pub enum Cursor<'a> {
    Prose,
    Head { list: &'a Node },                 // first word of a list: offer keywords, macros
    Arg  { list: &'a Node, index: usize },   // inside a call: signature help, variables
    Name { ident: &'a Node },                // hover, go-to-definition
    Ref  { partial: &'a str },               // inside (ref ...): offer labels
}
pub fn locate(analysis: &Analysis, at: ByteOffset) -> Cursor<'_>;
```
- `analyze` order: parse -> collect alias/macro -> expand -> let substitution -> collect labels -> check keyword calls. No output produced.
- `compile` has no `Result`: a broken spot renders as a visible inline error box; the rest of the note renders normally.
- Embedded `stdlib.zr` is parsed once per process.
- `(eval ...)` is standalone only (a block or inline of its own); nested inside `show` is a diagnostic. `show` never evaluates.
- Evaluation runs during `render`, in document order; the evaluator keeps its own state and sees fully expanded, substituted trees.
- No evaluator present: `(eval ...)` gives a diagnostic and an inline error box.

### Decisions made during step 1

1. A single newline inside a paragraph is a soft break, kept as `\n` in `Text`; `mazer-doc` renders it as `<br/>` to match the old output. Two trailing spaces or a single unescaped trailing backslash make a `LineBreak` (`foo\\` at the end of a line is a literal backslash and a soft break).
2. `BlockKind::Code` is produced only for a reserved-head form that is alone in a top-level paragraph (blank lines around it). A form on its own line directly under text is `InlineKind::Code`.
3. Every `MathMl::expr` call writes exactly one element.
4. Parentheses are decided by precedence levels, never by inspecting a child's head. Each keyword file declares a `Prec` level per keyword; loosest to tightest: `Binder` (∀, ∃) < `Implication` (⟹, ⟺) < `Or` < `And` < `Not` (¬) < `Relation` (=, <, ∈, ...) < `Additive` (+, binary −, ∪) < `Prefix` (unary −/+, negative numbers) < `BigOp` (∫, ∑, ∏, lim, d/dx) < `Multiplicative` (×, ÷, ∩, juxtaposition) < `Fraction` < `Power` (superscripts) < `Postfix` (!, ′) < `Atom` (names, subscripts, fences, function applications, ...). Infix rows say how they group (`Assoc::Both`, `Left` for binary − and ÷, `Neither` for relation chains and ⟹/⟺, so `(a ⟹ b) ⟹ c` and `(a < b) = c`). Rules on top of the levels:
   - **Signs:** an expression that starts with a sign is parenthesised after another arithmetic operator and under a unary operator, big operator, postfix or script (`a + (−2)`, `−(−x)`, `a (−b)`, `∑ (−a)`), but not at the start of an arithmetic row or after a relation (`−2 + a`, `−s × t`, `x = −2`).
   - **Quantifiers:** the body runs to the right and is never parenthesised; the whole quantifier is parenthesised in any operand position (`(∀x. P) ∧ Q`).
   - **Big operators:** the body is parenthesised when additive or looser, or signed (`∑ (a + b)`); the big operator itself is parenthesised as a script base, a non-last factor, or under anything tighter than a product (`(∑ a) × b`, `b × ∑ a`, `∑ a + b`); a product or signed expression ending in one counts as one.
   - **Postfix and scripts:** `!` parenthesises its operand unless atomic, a postfix one too (`(x²)!`, `(n!)!`, never the double factorial `n!!`), while a prime does not (`n!′`, `f′′`); a script base is parenthesised unless atomic (`(n!)²`, `(a/b)²`, `x_i²`); a function-application head is parenthesised when looser than a power (`(f + g)(x)`, but `f′(x)`, `f⁻¹(x)`).
   The full keyword-to-level table and the per-kind recipes are in `mazer-math/src/keywords/mod.rs`.
5. Twelve old symbol names are keywords now and are not display names, so each of these keywords MUST have a `.sign(...)` so bare use still renders the symbol: `partial` ∂, `approx` ≈, `neq` ≠, `leq` ≤, `geq` ≥, `subset` ⊂, `in` ∈, `notin` ∉, `union` ∪, `forall` ∀, `exists` ∃, `arrow` →.
6. Did-you-mean on a list head that is not a keyword is an `UnknownForm` **hint** (not a warning), since the name may be deliberate. A keyword is suggested only if: the name is 4 to 7 characters and 1 edit away, or 8 or more and 2 edits away (3 or fewer: never); it starts with the same character as the name (so `fold`, `trace` get nothing); and it is not a prefix of the name (`sinh`, `arcsinh` are deliberate), except that a keyword of 4 or more characters with its last letter doubled is a typo (`fracc`, `integrall`). Edits count characters, and swapping two neighbours is one edit.
7. A code form never crosses a blank line (empty or whitespace-only). The text handed to the reader for one form ends at the first blank line after the opening paren, or at the end of the enclosing container (list item, block quote) if that comes first. A form not closed by then is unclosed: one `UnclosedParen` at its outermost opening paren, an `Error` node in the tree, and parsing resumes normally at the blank line. A form may still span several consecutive non-blank lines; a closed one may carry a list item or quote over unindented continuation lines, an unclosed one never stretches its container.
8. A reserved head counts only when directly followed by whitespace, `(` or `)`: `(let's see)`, `(show's over)` and `(eval"x")` are prose, as is a head at the very end of the input.
9. `ref` is never a block: `(ref x)` alone in a paragraph is a `Paragraph` holding one `InlineKind::Code`. `BlockKind::Code` is only for `show`, `eval`, `let`, `alias`, `macro`.
10. In code, `;` ends an atom and starts a comment (outside strings): `(a;c` + newline + `b)` reads as `(a b)`.
11. For authors: prose that happens to open a parenthesis with a reserved word, e.g. "(show that f is continuous)", is read as code. Write `\(show that f is continuous)` to keep it prose.

### Decisions made during step 2

1. **Node budget.** Macro expansion and `let` substitution share one budget per `analyze` of **250 000 created nodes** for the whole document: every node of an instantiated alias/macro template, every copied macro argument and every node of a substituted `let` value counts. Storing a value with `let` is free (a move); the value is charged each time it is substituted at a use, including a use inside a later `let`. On top of that, one expanded or substituted tree may have at most **100 000 nodes and 320 levels**. When the budget runs out, the expression being processed becomes an `Error` node (an error box), exactly **one** diagnostic is reported for the whole document (`MacroDepth` if a macro or alias ran it out, `Other` if a variable did; the message names the limit and the help suggests a definition is growing exponentially; the span is the expression that crossed the limit), and every later expansion or substitution that would create nodes is refused the same way, silently. A `let` refused this way makes its variable an error from then on. At the limit a page holds roughly 2.5 MB of `MathML` from created nodes, and `analyze` (run on every keystroke) stays well under a second.
2. `(show a b c ...)` with more than one expression (after an optional `(tag name)`) is exactly `(show (jux a b c ...))`. `analyze` builds that `jux` node in the resolved tree, so spacing and parentheses come from the `jux` keyword (`(show (+ a b) c)` reads `(a+b) c`) and checking and rendering see the same tree. A single expression is unchanged.
3. Reserved heads and `tag` may not be nested in code. A list headed by `show`, `eval`, `let`, `alias`, `macro`, `ref` or `tag` anywhere inside a `show`, an `eval` or a `let` value (also inside `quote`), other than the one `(tag name)` that may start a display `show`, is reported once at that inner list and replaced by an `Error` node, so the enclosing form is an error box. Codes: `NestedEval` for `eval`, `MisplacedTag` for `tag`, `BadDefinition` for `let`/`alias`/`macro`, `Other` for `show`/`ref` ("`ref` can only be used in prose, not inside `show`"). An alias or macro whose template contains such a list is rejected where it is defined (same codes) and the name is not defined, so `(alias T (tag a))` cannot smuggle a tag in. A reserved word that is not a list head, as in `(show (jux show me))`, is an ordinary variable.
4. A rest parameter is spelled `name...` in the signature and only as the last parameter; in the body it is written `name` and must stand inside a list, where the remaining arguments are spliced: `(macro (pair a rest...) (paren a rest))`.
5. `let`, `alias` and `macro` are accepted as a block or inline anywhere (paragraph, heading, list item, block quote, link, style) and produce no output. A paragraph, heading, list item, block quote or inline style whose content is only sound definitions and whitespace produces no output at all, and a list whose items all vanish produces no list. A broken definition shows an error box instead, which keeps its container.
6. A `let` may not reuse the name of an alias or macro (`DuplicateDefinition`, since macros are expanded first and the variable could never be used), nor a keyword, reserved word or `tag` (`ShadowsBuiltin`). Display names such as `tau` may be bound.
7. Bound variables are substituted everywhere outside `quote`: in head position and inside binders too. So after `(let x 2)`, `(sum x x 1 n)` shows the 2 as the summation variable. Use a different variable name, or quote it (`(sum x 'x 1 n)`).
8. A `let` value is checked once, where it is bound (after substituting earlier variables), and its keyword problems are reported at the `let`, which then shows an error box; uses of the variable still render the `<merror>` but do not report it again. A value that is a single name is not checked at the `let`. `analyze` merges diagnostics with the same code, span and message into the first.
9. The error-box rule. A code form renders as an error box (`<pre class="mazer-error">` for a block, `<span class="mazer-error">` inside a sentence) holding the escaped messages, instead of its normal output, when:
   1. its resolved tree contains an `Error` node (reader recovery, a failed macro use, a nested reserved form, an oversized expansion or substitution, the document's node budget running out), or
   2. `Analysis::diags` holds an *error*-severity diagnostic whose primary span lies inside the form and which rendering the form's expressions does not produce itself. Problems the math emitter reports (a keyword with the wrong number of arguments, a bare keyword without a sign) already render in place as `<merror>`, so they do not box the form; everything the document stage reports (bad definitions, macro arity and depth, misplaced or duplicate tags, unresolved references, an empty `show`, reader errors) does, or
   3. it is an `eval` and there is no evaluator (`NoEvaluator`) or the evaluator returned `None`.

   This applies to definitions too: a broken `let`, `alias` or `macro` shows a box where a good one shows nothing. A `let` renders no expression, so a keyword problem in its value boxes it. Warnings and hints never box. An `eval` that boxes for reason 1 or 2 is not evaluated. The box of a tagged display equation keeps the equation's `id`, so references to it still land on it.
10. Markup:
    - display equation: `<div class="mazer-equation" id="eq-name"><math display="block">..</math><span class="mazer-eqno">(n)</span></div>`, with no `id` or number when untagged; its error box is `<pre class="mazer-error" id="eq-name">`;
    - inline math: `<math display="inline"><mstyle displaystyle="true">..</mstyle></math>`;
    - tag: `(tag name)` only as the first item of a display `show`; the anchor is `eq-` followed by the name as written; numbers count tagged equations in document order from 1;
    - reference: `<a class="mazer-ref" href="#eq-name">(n)</a>`; inside the text of a link just `(n)`, since links do not nest;
    - checklist item: `<li><input type="checkbox" disabled> text</li>` (`checked disabled` when ticked); a list of only checklist items is `<ul class="mazer-checklist">`.
11. Dangerous URL schemes are neutralised. A link or image whose URL, trimmed, ASCII-lowercased and without ASCII whitespace and control characters, starts with `javascript:`, `vbscript:` or `data:` (except `data:image/` for an image) is written as its plain text (link) or alt text (image) with no `href`/`src`, and `render` reports one `Other` warning per occurrence at the link or image. `analyze` does not report it, so the LSP does not see it.

### Decisions made during step 3

1. **`--check`** is `compile` with the HTML thrown away: it prints the same diagnostics, deduplicated the same way, as a real compile of the same note (including the render-time ones: dangerous URLs, `(eval ...)` without an evaluator). It writes no file, and cannot be combined with `-o` or `--open`.
2. **`--strict`** makes warnings fail the run too, with or without `--check`; hints never fail it. The summary line then says so.
3. **Exit codes:** 0 success (warnings and hints do not fail); 1 the note has errors (the HTML is still written) or, under `--strict`, warnings; 2 usage or I/O error, including an output that is the same file as the input (found by canonical path, and on Unix by device and inode, so `./x.zr`, an absolute path, `a/../x.zr`, a symlink and a hard link are all refused). `query` exits 1 when nothing matches.
4. **Summary line:** printed whenever anything at all was reported, counting hints too (`1 error, 2 warnings, 1 hint`; zero counts are left out).
5. **Column conventions.** The CLI and the wasm console line report 1-based **character** columns. The LSP reports **UTF-16** units, as its protocol requires. Spans themselves are **byte** offsets. Never mix them: a column is converted from a span's byte offset at the edge.
6. **LaTeX target scope.** `--target latex` changes only how math is emitted: `\( ... \)` inline and `\[ ... \]` for a display equation (no `\displaystyle` is added to inline math), inside the same HTML as before. A full page does not load a TeX renderer; whole-document `.tex` output is not built.
7. **Editor scope of `let`.** A binding is in scope from the END of its `let` form, so a use inside the `let`'s own value sees the previous binding, and completion, hover and go-to-definition follow that rule.
8. **Completion of reserved heads.** The reserved words (`show`, `eval`, `let`, `alias`, `macro`, and `tag` first in a `show`) are offered only at the head of a top-level form, never at the head of a nested list, where nesting them is an error.
9. **Diagnostics printing** hands `annotate-snippets` only the lines a diagnostic's spans cover plus one line of context each side (with the real starting line number), so printing costs time proportional to the lines shown, not to the file.

### Helper APIs in the skeleton

- `mazer_syntax::reader::{read_form, read_str}`: read one form at an offset (returns a `Read`), or read a whole string as one `Node`.
- `mazer_syntax::markdown::parse(text, file, diags) -> Document`.
- `mazer_syntax::emoji::lookup(name) -> Option<&'static str>`: the 13 shortcodes.
- `Node` constructors (`new`, `ident`, `number`, `string`, `list`, `error`), accessors (`as_ident`, `as_list`, `head`, `args`, `is_form`), a `Display` impl that prints the tree as source, and `same_shape` (structural equality ignoring spans).
- `mazer_math::{bind, check, to_mathml, registry, display_name, display_names}`: argument binding, keyword-call checking, one-expression rendering, the global registry, and display-table lookup.
- `mazer_doc::macros::collect_macros(doc, &mut macros, diags)` and `mazer_doc::bindings::substitute(node, &bindings) -> Node`.

### Front ends

- **wasm:** `run_mazer(content: &str, window_name: &str) -> String` keeps its exact signature so `mazer.js` and adhyan.ca need no edits. Body is `compile(...)` with `Target::MathMl`, `Page::Fragment`, no evaluator. Diagnostics also go to the browser console.
- **CLI** (binary named `mazer`), hand-written argument parsing, diagnostics via `annotate-snippets`:
  ```
  mazer <file.zr | ->  [-o|--output <path>] [--open] [--fragment] [--target latex] [--check] [--strict]
  mazer query <text...> [--file <note.zr>]
  ```
  Default output is `<name>.html` next to the input (stdout for the input `-`, which is standard input and is named `<stdin>`). `--check` compiles exactly as a normal run does, prints every diagnostic that run would print, and writes nothing. `--strict` makes warnings fail the run as well. The output may never be the same file as the input. `query` searches keywords, display names and, with `--file`, that note's macros and aliases.
- **LSP** (`mazer-lsp`, on `lsp-server`, stdio, full-text sync, editor: Neovim with a small Lua snippet):
  ```rust
  struct Server { connection: lsp_server::Connection, open: HashMap<Url, OpenNote> }
  struct OpenNote { version: i32, analysis: Analysis }
  ```
  First: diagnostics; completion (keywords, display names with their character, macros/aliases/let variables, labels in `ref`, the 13 emoji); hover; signature help. Then, in order: go-to-definition, semantic-token colouring, live preview on a local port.
- **LaTeX:** math expressions first (`--target latex`); whole-document `.tex` output is later.

## Rules for contributors

- Models: **Opus or Sonnet only.**
- Branch: **`agentic-rewrite`** (created from `master`). Each agent works in its own git worktree. `master` is never touched; nothing is pushed.
- **Agents may commit** on that branch. Commits are credited to Claude: authored as Claude (`--author`), ending with the session's attribution trailers.
- The skeleton's public signatures are the contract. An agent that believes one is wrong stops and reports instead of changing it.
- Old crates stay until step 3 so the old binary can produce reference output. `mazer-dbg` is never modified.
- Build facts: custom `CARGO_TARGET_DIR`, nightly toolchain, dev profile uses cranelift, wasm must be built with `--release` (`make wasm`).
- **Checkpoints: pause only on problems** (failing tests, a disputed signature, a serious review finding). Otherwise steps run back to back.
- Copy `~/Desktop/Projects/adhyan.ca/notes/*.zr` into `fixtures/notes/` in this repo. **They are never committed**: `fixtures/notes/` and the reference HTML go in `.gitignore`. The website repository is never edited.
- Tests that use these fixtures skip cleanly when the directory is absent.
- Migrated copies (new calculus order, `string` removed) are produced in the same ignored directory; the user copies them back to the site by hand.
- Fixtures live at `/home/home/Desktop/Projects/mazer/fixtures/notes/` (12 `.zr` notes) and the old-CLI reference HTML at `/home/home/Desktop/Projects/mazer/fixtures/reference/`. Both are git-ignored and must never be committed.
- `DiagCode` is `#[non_exhaustive]`, so adding variants is allowed.
