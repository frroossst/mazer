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
| Diagnostics | Everything `mazer` would report, as you type: unclosed parens, wrong argument counts (with the signature as help; inside notation, with a link to the template form of the alias or macro), unknown references, duplicate definitions, did-you-mean hints, dangerous link URLs. Related locations ("first defined here") are linked. An `eval` is only a hint: there is no evaluator in the editor. |
| Completion | After `(` inside code: every keyword (by each of its names) with its signature, sign, parameters and examples, inserted with tab stops for the required arguments; your macros and aliases. As an argument: `let` variables in scope (with their value), display names with their character (`tau` τ; you can also type words of the description, e.g. `arrow`), aliases, then keywords. Inside `(table ...)`: the directives `header`, `row`, `align`, `style` first, then `left`/`center`/`right` inside `(align ...)` and `grid`/`clean`/`plain` in `(style ...)`. Inside `(ref `: your equation labels with their numbers. In prose: `show`, `let`, `alias`, `macro`, `ref`, `eval` after `(`, and the 13 emoji shortcodes after `:`. Nothing inside comments, strings or code spans. |
| Hover | Keywords (signature, doc, parameters, sign, examples), display names (the character), macros and aliases (signature, doc string, expansion, where defined), `let` variables (the value in scope and where it was bound), `(ref name)` / `(tag name)` (the equation number and its source), reserved words. |
| Signature help | Inside any keyword, macro or reserved-word call, with the parameter the current argument binds to highlighted, by the same rule the renderer uses (required first, then optional left to right, then rest): typing the third argument of `(integral body var [lower] [upper])` highlights `[lower]`. |
| Go to definition / references | Macros and aliases, `let` variables (the binding in scope, so a re-bound name goes to the right `let`), equation labels, macro parameters. |
| Outline | Document symbols: headings as a tree, with the macros, aliases, `let` variables and tagged equations of each section. |
| Semantic colouring | Code coloured by what it is: reserved words, keywords, macros, display names, variables, numbers, strings, comments, labels. |
| Live preview | The `mazer.preview` command serves the note, rendered, on `http://127.0.0.1:<port>/doc/<n>` and reloads the page whenever you type. Loopback only; see "Robustness" below. |

Capabilities advertised: UTF-16 positions, full text sync (open, change,
close, save), completion (triggers `(` `:` `'`; not space) with
`completionItem/resolve`, hover, signature help (triggers space and `(`),
definition, references, document symbols, semantic tokens (full), and the
command `mazer.preview`.

Completion lists are light: each item carries its label, kind, `detail`
(signature, character or value), a one-line plain `documentation`, sort and
filter text, the edit and a small `data` payload; the full markdown docs
(parameters, examples, sources) come from `completionItem/resolve`. When the
client supports `completionList.itemDefaults.editRange` (Neovim 0.11 does),
the shared edit range is sent once. An argument position (about 320 items)
is about 77 KB of JSON that way, 111 KB without it (it was 175 KB with the
docs inline).

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
-- or, for completion only on <C-x><C-o>:
-- require('mazer').setup({ autotrigger = false })
```

It registers the `zr` filetype for `*.zr`, starts `mazer-lsp` for each note
(root: the note's directory), links the semantic token types to highlight
groups (`@lsp.type.<type>.zr`, overridable by your colour scheme), turns on
the built-in completion popup, and adds `:MazerPreview`, which opens the live
preview in your browser.

Hover is `K`, signature help `<C-s>` in insert mode, go to definition
`<C-]>` / `gd` if you map it, references `grr`, outline `gO` (Neovim 0.11
defaults).

Completion pops up by itself after `(`, `:` and `'`, and anywhere with
`<C-x><C-o>`; `require('mazer').setup({ autotrigger = false })` makes it
manual only. It never inserts anything by itself: in `.zr` buffers
`completeopt` is set, buffer-locally, to `menu,menuone,noselect,popup`
(Neovim's default `menu,popup` would select and insert the first item, so
typing `(show` would give `(aliasshow`). Choose with `<C-n>`/`<C-p>`, accept
with `<C-y>`; Enter and Space with nothing selected are just a newline and a
space. Neovim 0.11's built-in popup shows the one-line doc of the selected
item; it does not resolve items for their full docs (hover with `K` after
inserting a name shows them). Accepting an item in the middle of a word
inserts it before the rest of the word (a Neovim limitation: it replaces only
up to the cursor).

`nvim/test_typing.lua` types into `.zr` buffers key by key in a headless
Neovim, with the menu popping up, and checks every buffer holds exactly what
was typed; `cargo test -p mazer-lsp --test nvim_typing` runs it when `nvim`
0.11+ is installed (on PATH or `/opt/nvim-linux-x86_64/bin/nvim`).

## Other editors

Start `mazer-lsp` with no arguments; it speaks LSP over stdin/stdout. Attach
it to files with the extension `.zr` (language id `zr`). It reads no
configuration. For the preview, send `workspace/executeCommand` with
`{"command": "mazer.preview", "arguments": ["<document URI>"]}`; the result
is the URL to open.

## Robustness

- **Framing.** The server reads the `Content-Length` framing itself (on top
  of `lsp-server`'s message parser): a message larger than 64 MiB, a header
  line longer than 8 KiB, malformed framing or a body that is not JSON-RPC
  ends the session, and `mazer-lsp` exits with status 1 and the reason on
  stderr. (`lsp-server`'s own transport would try to allocate whatever
  `Content-Length` says and abort.) A malformed *message* inside good
  framing (unknown method, bad parameters) is answered with an error and the
  session goes on.
- **Preview server.** Bound to `127.0.0.1` only. The request head must
  arrive within 5 seconds of connecting, however slowly it trickles in; at
  most 32 connections are served at once and further ones are closed
  immediately; a request with more than one `Host` header is a 400, one
  whose `Host` is not `127.0.0.1:<port>` or `localhost:<port>` a 403. A note
  closed in the editor is no longer served (404); reopen it and run
  `mazer.preview` again to preview it.
- **Big input.** Every keystroke re-runs `analyze` on the whole note. The
  markdown stage is quadratic on some pathological input (200 000 `|`
  characters, or a list nested 8 000 deep, take tens of seconds); real notes
  are far from that.

## Tests

```sh
cargo test -p mazer-lsp
MAZER_FIXTURES=/path/to/notes cargo test -p mazer-lsp --test fixtures
```

`tests/stdio.rs` runs a whole session against the built binary over real
JSON-RPC; `tests/fixtures.rs` calls every feature at every character of every
note in `MAZER_FIXTURES` (default `../fixtures/notes`, skipped when absent);
`tests/nvim_typing.rs` runs `nvim/test_typing.lua` (skipped without nvim).
