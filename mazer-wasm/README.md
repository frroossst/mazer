# mazer-wasm

Render mazer `.zr` notes **in the browser** and drop them into your own HTML
(your blog, your CSS, your layout). The `.zr` → HTML compiler — the markdown
parser, the math keyword renderer (Lisp forms to MathML), and the document
builder — runs entirely as a WebAssembly module. A tiny JS auto-loader (`mazer.js`) does only the page
wiring: find the blocks, fetch the files, swap in the rendered HTML.

## Build

```sh
# one-time: install the wasm bundler
cargo install wasm-pack

# build the wasm module + JS glue into mazer-wasm/pkg/
make wasm
```

This produces `mazer-wasm/pkg/` (the `.wasm` binary and generated ESM glue),
plus `pkg/mazer_search.js`, a copy of the equation search script
(`assets/mazer-search.js`; see [Search](#search)).

## Use it in your site

Copy two things into your site — say under `/mazer/`:

- `mazer.js` (the auto-loader)
- `pkg/` (the wasm module + glue, and `mazer_search.js`)

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

## Search

Every page that shows notes can search its equations by their source.
Press **Ctrl+K** (**Cmd+K** on macOS), or **/** when you are not typing in a
field, or tap the small round magnifying-glass button in the bottom-right
corner (for phones and tablets), and type part of an equation as you wrote
it: `(funcapp rho x)` finds every equation whose source contains that text.

- **Spacing.** Spacing and line breaks do not matter, nor does a space
  between `)` and `(`: `x)(integral` finds `x) (integral`.
- **Case.** A lowercase query ignores case; a query with an uppercase letter
  does not (`Delta` and `delta` are different names).
- **Whole forms.** A form copied whole from a note works as it is:
  `(show (tag mass) (= (funcapp Delta x) (funcapp delta x)))` searches for
  the expression, without the `(show`, the tag and the closing paren, also
  when the copy stops half-way. A paste loses its `;` comments, and its line
  breaks become spaces (a browser would otherwise glue the lines together).
- **Order.** Equations whose whole source is the query come first, then
  those where the query matches whole tokens (it starts and ends at a space, a
  paren or the end of the source), then the rest, each group in page order.
  The list shows at most 50 rows; the count is the total. Each row has the
  rendered math, the source with the match marked, and the heading above it.
- **Keys.** **Up**/**Down** move through the list, **Enter** (or a click)
  scrolls to the equation, highlights it for two seconds and moves the
  keyboard focus to it; **Escape** (or Ctrl+K again, or a click outside)
  closes the box and puts the focus back where it was.

- It searches the current page only, whatever was rendered on it, however
  late; nothing is fetched or indexed ahead of time.
- It relies on the `data-mazer-src` attribute mazer puts on every rendered
  equation (`div.mazer-equation` for a display equation, the `math` element
  for inline math, `span.mazer-math` for inline math with the LaTeX target),
  which holds the source as typed, on one line, without comments.
- The script is one file with no dependencies, `assets/mazer-search.js` in
  this repository. `make wasm` copies it to `pkg/mazer_search.js`, and
  `mazer.js` loads it with `import()` once every block is rendered. If it is
  missing, the notes still render and the console gets a warning. Opt out
  with `window.mazerSearch = false` before the module loads.
- The button appears once the page has an equation (at load, or at the first
  click, tap or key after the notes are rendered), is not printed, and can be
  turned off with `window.mazerSearchButton = false` before the script loads;
  the keyboard shortcuts stay.
- The CLI's full pages and the language server's live preview carry the same
  script inline when the note has an equation, so they need nothing extra.
- Its styles use system colours and only classes that start with
  `mazer-search-`; the jumped-to equation gets `mazer-search-hit` for two
  seconds, which a site may restyle.

## Notes

- **Serve over http(s).** `fetch()` does not work from `file://`, so open the
  page through a web server (e.g. `python3 -m http.server`), not by
  double-clicking the file.
- **Syntax highlighting.** If a note contains a code block, the loader injects
  [Arborium](https://www.npmjs.com/package/@arborium/arborium) (from a CDN)
  after rendering. Opt out with `window.mazerHighlight = false` before the
  module loads. Math renders as native MathML and needs no script.
- **Error isolation.** A missing file shows an inline error box in that block
  only; the other blocks still render. `run_mazer` never fails and never
  panics, so a bad note can't poison the module. A broken form inside a note
  (a keyword with the wrong number of arguments, an unknown reference, an
  unclosed paren, `(eval ...)`, which the website build cannot run) becomes
  an error box for that one form, `<pre class="mazer-error">` for a display
  and `<span class="mazer-error">` inside a sentence; the rest of the note
  renders normally.
- **Console diagnostics.** Every diagnostic is also written to the browser
  console, one line each, `console.error` for errors and `console.warn` for
  warnings and hints:
  `mazer: <name>:<line>:<col>: <severity>[<code>]: <message>` (line and column
  are 1-based and the column counts characters, as the `mazer` command-line
  tool does; `<name>` is the `data-mazer` path), followed by `; help: ...` when
  there is a suggestion.
- **Styling.** Besides the ordinary HTML elements, the output uses these
  classes: `mazer-equation` (display equation), `mazer-eqno` (its number),
  `mazer-ref` (a `(ref name)` link), `mazer-checklist` (a task list) and
  `mazer-error`. [`docs/site-styles.css`](../docs/site-styles.css) has ready
  rules for the first four, and a `.note pre.mazer-error` override for sites
  whose `.note pre` rule would otherwise strip the block error box; the CLI's
  full-page output carries a small default style for `mazer-error`.
- **Trust model.** The loader sets `innerHTML` from the compiled output. That
  output is your own first-party `.zr` content, which is exactly what you want
  rendered as HTML. Don't point `data-mazer` at `.zr` files from untrusted third
  parties.

## Deploying

1. **Always deploy `mazer_wasm_bg.wasm` and `mazer_wasm.js` together.** The
   glue is generated for one exact build of the module. If a browser (or a CDN)
   has an old `mazer_wasm.js` cached and fetches the new `.wasm`, instantiation
   fails and every block stays at "Loading note...". Two precautions, both on
   the site side (this repository does not edit `mazer.js`):
   - Wrap the loader's `init()` call in `try`/`catch` and, on failure, put a
     visible message in each block (for example "mazer failed to load, try a
     hard refresh") instead of leaving the placeholder.
   - Add a version query string to the glue import, for example
     `import init, { run_mazer } from './pkg/mazer_wasm.js?v=0.16.0';`, and
     change it on every deploy so the browser fetches the new glue. The glue
     finds its `.wasm` by a URL that drops the query string, so also pass the
     versioned address to the initialiser, `await init('./pkg/mazer_wasm_bg.wasm?v=0.16.0')`,
     to get a matching pair.
2. **Deploy `mazer_search.js` next to them.** `make deploy` copies three
   files to the site: `mazer_wasm_bg.wasm`, `mazer_wasm.js` and
   `mazer_search.js`. The script does not call into the module; it only reads
   the `data-mazer-src` attributes in the page, so with a module built before
   that attribute existed it opens but finds no equations. Give it the same
   `?v=` query string as the other two in the loader so a browser does not
   keep an old copy. A loader that does not import it simply has no search
   box.
3. **Migrate old notes before the new wasm goes live.** Notes written for the
   old mazer must be updated first: `integral`, `sum` and `prod` now take the
   body and the variable first and the bounds last, and the `string` form is
   gone (write a quoted string instead). `make deploy` runs
   `mazer --check --strict` on every `*.zr` under the site's `notes/`
   directory and refuses to copy anything if one of them fails; it never
   modifies a note.

## Try the demo

```sh
make wasm
cd mazer-wasm
python3 -m http.server 8000
# open http://localhost:8000/
```

`index.html` is a minimal blog wrapper that renders `demo/note.zr` and
`demo/about.zr`. Copy it as a starting point for your own site.
