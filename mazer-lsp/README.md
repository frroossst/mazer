# mazer-lsp

A language server for mazer notes (`.zr`): markdown with embedded Lisp math.
It exists so you do not have to remember the syntax: the editor tells you
which keywords exist, what their arguments are, what a symbol is called and
what a name means.

Synchronous, over stdio, built on `lsp-server` (no async runtime). Each open
note is re-analyzed from its full text on every change; notes are small, so
this is instant.

## What it offers

| Feature | What you get |
|---|---|
| Diagnostics | Everything `mazer` would report, as you type: unclosed parens, wrong argument counts (with the signature as help), unknown references, duplicate definitions, did-you-mean hints, dangerous link URLs. Related locations ("first defined here") are linked. An `eval` is only a hint: there is no evaluator in the editor. |
| Completion | After `(` inside code: every keyword (by each of its names) with its signature, sign, parameters and examples, inserted with tab stops for the required arguments; your macros and aliases. As an argument: `let` variables in scope (with their value), display names with their character (`tau` τ; you can also type words of the description, e.g. `arrow`), aliases, then keywords. Inside `(ref `: your equation labels with their numbers. In prose: `show`, `let`, `alias`, `macro`, `ref`, `eval` after `(`, and the 13 emoji shortcodes after `:`. Nothing inside comments, strings or code spans. |
| Hover | Keywords (signature, doc, parameters, sign, examples), display names (the character), macros and aliases (signature, doc string, expansion, where defined), `let` variables (the value in scope and where it was bound), `(ref name)` / `(tag name)` (the equation number and its source), reserved words. |
| Signature help | Inside any keyword, macro or reserved-word call, with the parameter the current argument binds to highlighted, by the same rule the renderer uses (required first, then optional left to right, then rest): typing the third argument of `(integral body var [lower] [upper])` highlights `[lower]`. |
| Go to definition / references | Macros and aliases, `let` variables (the binding in scope, so a re-bound name goes to the right `let`), equation labels, macro parameters. |
| Outline | Document symbols: headings as a tree, with the macros, aliases, `let` variables and tagged equations of each section. |
| Semantic colouring | Code coloured by what it is: reserved words, keywords, macros, display names, variables, numbers, strings, comments, labels. |
| Live preview | The `mazer.preview` command serves the note, rendered, on `http://127.0.0.1:<port>/doc/<n>` and reloads the page whenever you type. Loopback only. |

Capabilities advertised: UTF-16 positions, full text sync (open, change,
close, save), completion (triggers `(` `:` `'` space), hover, signature help
(triggers space and `(`), definition, references, document symbols, semantic
tokens (full), and the command `mazer.preview`.

## Build and install

```sh
cargo install --path mazer-lsp      # puts `mazer-lsp` in ~/.cargo/bin
```

`cargo build -p mazer-lsp --release` builds it without installing.

## Neovim

Needs Neovim 0.11 or newer; no plugins. Make `nvim/mazer.lua` loadable as the
module `mazer` and require it:

```sh
ln -s "$PWD/mazer-lsp/nvim/mazer.lua" ~/.config/nvim/lua/mazer.lua
```

```lua
-- init.lua
require('mazer')
-- or, if you use nvim-cmp or another completion plugin:
-- require('mazer').setup({ autocomplete = false })
```

It registers the `zr` filetype for `*.zr`, starts `mazer-lsp` for each note
(root: the note's directory), links the semantic token types to highlight
groups (`@lsp.type.<type>.zr`, overridable by your colour scheme), turns on
the built-in completion popup, and adds `:MazerPreview`, which opens the live
preview in your browser.

Hover is `K`, signature help `<C-s>` in insert mode, go to definition
`<C-]>` / `gd` if you map it, references `grr`, outline `gO` (Neovim 0.11
defaults).

## Other editors

Start `mazer-lsp` with no arguments; it speaks LSP over stdin/stdout. Attach
it to files with the extension `.zr` (language id `zr`). It reads no
configuration. For the preview, send `workspace/executeCommand` with
`{"command": "mazer.preview", "arguments": ["<document URI>"]}`; the result
is the URL to open.

## Tests

```sh
cargo test -p mazer-lsp
MAZER_FIXTURES=/path/to/notes cargo test -p mazer-lsp --test fixtures
```

`tests/stdio.rs` runs a whole session against the built binary over real
JSON-RPC; `tests/fixtures.rs` calls every feature at every character of every
note in `MAZER_FIXTURES` (default `../fixtures/notes`, skipped when absent).
