# mazer-wasm

Render mazer `.zr` notes **in the browser** and drop them into your own HTML
(your blog, your CSS, your layout). The `.zr` → HTML compiler — the markdown
parser, the Lisp interpreter, and the document builder — runs entirely as a
WebAssembly module. A tiny JS auto-loader (`mazer.js`) does only the page
wiring: find the blocks, fetch the files, swap in the rendered HTML.

## Build

```sh
# one-time: install the wasm bundler
cargo install wasm-pack

# build the wasm module + JS glue into mazer-wasm/pkg/
make wasm
```

This produces `mazer-wasm/pkg/` (the `.wasm` binary and generated ESM glue).

## Use it in your site

Copy two things into your site — say under `/mazer/`:

- `mazer.js` (the auto-loader)
- `pkg/` (the wasm module + glue)

Add **one** tag to your page:

```html
<script type="module" src="/mazer/mazer.js"></script>
```

Then, anywhere you want a note, reference a `.zr` file:

```html
<div data-mazer="/notes/2026-06-14.zr"></div>
```

On load, the auto-loader boots the wasm compiler, fetches each referenced file,
compiles it, and replaces the element's contents with the rendered HTML. Your
notes stay as ordinary `.zr` files; the page inherits your site's styling.

## Notes

- **Serve over http(s).** `fetch()` does not work from `file://`, so open the
  page through a web server (e.g. `python3 -m http.server`), not by
  double-clicking the file.
- **Syntax highlighting.** If a note contains a code block, the loader injects
  [Arborium](https://www.npmjs.com/package/@arborium/arborium) (from a CDN)
  after rendering. Opt out with `window.mazerHighlight = false` before the
  module loads. Math renders as native MathML and needs no script.
- **Error isolation.** A missing file or a malformed note shows an inline error
  box in that block only; the other blocks still render. (`run_mazer` returns an
  error snippet instead of panicking, so a bad note can't poison the module.)
- **Trust model.** The loader sets `innerHTML` from the compiled output. That
  output is your own first-party `.zr` content, which is exactly what you want
  rendered as HTML. Don't point `data-mazer` at `.zr` files from untrusted third
  parties.

## Try the demo

```sh
make wasm
cd mazer-wasm
python3 -m http.server 8000
# open http://localhost:8000/
```

`index.html` is a minimal blog wrapper that renders `demo/note.zr` and
`demo/about.zr`. Copy it as a starting point for your own site.
