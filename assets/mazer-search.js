// mazer equation search
// ---------------------
// Ctrl+K (Cmd+K on macOS) or `/` opens a box that searches the equations on
// THIS page by their source, as the author typed it: `(funcapp rho x)` finds
// every equation whose source contains that text. Enter or a click scrolls to
// the equation and highlights it for a moment.
//
// It relies only on the `data-mazer-src` attribute mazer writes on every
// rendered equation (`div.mazer-equation` for display math, the `math`
// element for inline math, `span.mazer-math` for the LaTeX target).
//
// One dependency-free file that works inline in a script element, loaded by
// a script element's `src`, and when import()-ed as a module for its side
// effect.
// Loading it twice installs it once. The index is rebuilt every time the box
// opens, so it does not matter when (or how often) the notes were rendered.
//
// The pure helpers are on `globalThis.__mazerSearch` (for the tests in
// mazer-search.test.mjs, and for pages that want `open()`/`close()`).
(() => {
  'use strict';
  const root = globalThis;
  if (root.__mazerSearch || root.mazerSearch === false) return;

  const MAX_ROWS = 50; // rows rendered at once; more are summarised
  const HIGHLIGHT_MS = 2000; // how long a jumped-to equation stays highlighted

  // ---- matching (pure) ----

  // One space between tokens, none just inside a paren: `( frac  1\n2 )`
  // and `(frac 1 2)` normalise to the same text.
  const normalize = (s) =>
    String(s).replace(/\s+/g, ' ').trim().replace(/\( /g, '(').replace(/ \)/g, ')');

  // Lower-cases character by character, keeping any character whose lower
  // case has another length (a dotted capital I), so an index into the
  // folded text is an index into the original.
  const fold = (s) =>
    Array.from(s, (c) => {
      const l = c.toLowerCase();
      return l.length === c.length ? l : c;
    }).join('');

  // Smart case: a query with an uppercase letter is case-sensitive, because
  // `Delta` and `delta` are different names in mazer.
  const caseSensitive = (q) => fold(q) !== q;

  // `items` are `{ src }` (or index entries that also carry `norm`/`folded`).
  // Returns `{ item, start, end }` in input order; `start`/`end` delimit the
  // first match in the normalised source.
  function search(query, items) {
    const q = normalize(query);
    const sensitive = caseSensitive(q);
    const needle = sensitive ? q : fold(q);
    const hits = [];
    for (const item of items) {
      const norm = item.norm ?? normalize(item.src);
      const hay = sensitive ? norm : (item.folded ?? fold(norm));
      const at = hay.indexOf(needle);
      if (at >= 0) hits.push({ item, start: at, end: at + needle.length });
    }
    return hits;
  }

  const matches = (query, src) => search(query, [{ src }]).length > 0;

  // [before, matched, after]
  const split = (text, start, end) => [text.slice(0, start), text.slice(start, end), text.slice(end)];

  const api = { normalize, matches, search, split, MAX_ROWS };
  root.__mazerSearch = api;
  if (typeof document === 'undefined') return;

  // ---- the box ----

  const doc = document;
  const CSS = `
.mazer-search-backdrop{position:fixed;top:0;right:0;bottom:0;left:0;z-index:2147483646;display:flex;justify-content:center;align-items:flex-start;padding:10vh 16px 16px;box-sizing:border-box;background:rgba(0,0,0,.35);background:color-mix(in srgb,CanvasText 30%,transparent)}
.mazer-search-backdrop[hidden]{display:none}
.mazer-search-backdrop *{box-sizing:border-box}
.mazer-search-panel{display:flex;flex-direction:column;width:640px;max-width:100%;max-height:80vh;background:Canvas;color:CanvasText;border:1px solid GrayText;border-radius:8px;box-shadow:0 8px 32px rgba(0,0,0,.3);overflow:hidden;text-align:left;line-height:1.4}
.mazer-search-input{display:block;width:100%;margin:0;padding:.6em .75em;font:inherit;font-size:16px;color:CanvasText;background:Canvas;border:0;border-bottom:1px solid GrayText;border-radius:0}
.mazer-search-count,.mazer-search-note{padding:.3em .75em;font-size:.8em;color:GrayText}
.mazer-search-note{padding:.75em}
.mazer-search-list{flex:1 1 auto;min-height:0;margin:0;padding:0;list-style:none;overflow-y:auto;overscroll-behavior:contain}
.mazer-search-row{margin:0;padding:.5em .75em;border-top:1px solid rgba(127,127,127,.25);cursor:pointer}
.mazer-search-row:hover{background:rgba(127,127,127,.12)}
.mazer-search-row[aria-selected="true"]{background:Highlight;color:HighlightText}
.mazer-search-ctx{font-size:.8em;color:GrayText;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.mazer-search-row[aria-selected="true"] .mazer-search-ctx{color:inherit}
.mazer-search-math{overflow-x:auto;padding:.2em 0}
.mazer-search-math>*{margin:0}
.mazer-search-src{display:block;margin:0;padding:0;background:none;color:inherit;font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;font-size:.8em;white-space:pre-wrap;overflow-wrap:anywhere}
.mazer-search-sr{position:absolute;width:1px;height:1px;margin:-1px;padding:0;overflow:hidden;clip:rect(0 0 0 0);white-space:nowrap;border:0}
.mazer-search-hit{outline:2px solid Highlight;outline-offset:3px;border-radius:4px;background-color:rgba(255,213,0,.25);background-color:color-mix(in srgb,Highlight 15%,transparent)}
@media (max-width:480px){.mazer-search-backdrop{padding:8px}.mazer-search-panel{max-height:calc(100vh - 16px)}}
`;

  let box = null; // the backdrop; built on the first open
  let input, list, count, note, live;
  let entries = []; // { el, src, norm, folded, heading } in document order
  let hits = [];
  let active = -1; // selected row, -1 for none
  let previous = null; // focused before the box opened
  let pending = false; // a render is scheduled
  let pressedOnBackdrop = false;
  let flashed = null; // the highlighted equation and its timer
  const later = root.requestAnimationFrame ? (f) => root.requestAnimationFrame(f) : (f) => setTimeout(f, 16);

  function make(tag, className, attrs) {
    const el = doc.createElement(tag);
    if (className) el.className = className;
    for (const [k, v] of Object.entries(attrs || {})) el.setAttribute(k, v);
    return el;
  }

  function build() {
    const style = make('style');
    style.textContent = CSS;
    (doc.head || doc.documentElement).appendChild(style);

    box = make('div', 'mazer-search-backdrop');
    box.hidden = true;
    const panel = make('div', 'mazer-search-panel', { role: 'dialog', 'aria-modal': 'true', 'aria-label': 'Search equations' });
    input = make('input', 'mazer-search-input', {
      type: 'text', role: 'combobox', 'aria-label': 'Search equations by source', 'aria-autocomplete': 'list',
      'aria-expanded': 'true', 'aria-controls': 'mazer-search-list', placeholder: 'Search equations, e.g. (frac 1 2)',
      autocomplete: 'off', autocapitalize: 'off', spellcheck: 'false',
    });
    count = make('div', 'mazer-search-count', { 'aria-hidden': 'true' });
    live = make('div', 'mazer-search-sr', { role: 'status', 'aria-live': 'polite' });
    list = make('ul', 'mazer-search-list', { id: 'mazer-search-list', role: 'listbox', 'aria-label': 'Matching equations' });
    note = make('div', 'mazer-search-note');
    panel.append(input, count, live, list, note);
    box.appendChild(panel);

    // Keep the focus in the input whatever is clicked inside the panel.
    panel.addEventListener('mousedown', (e) => {
      if (e.target !== input) e.preventDefault();
    });
    // Close only when the press also started on the backdrop, so a text
    // selection dragged out of the input does not close the box.
    box.addEventListener('mousedown', (e) => { pressedOnBackdrop = e.target === box; });
    box.addEventListener('click', (e) => {
      if (e.target === box && pressedOnBackdrop) close();
    });
    list.addEventListener('click', (e) => {
      const row = e.target.closest && e.target.closest('.mazer-search-row');
      if (row) jump(Number(row.getAttribute('data-index')));
    });
    input.addEventListener('input', schedule);
    input.addEventListener('keydown', onInputKey);
    doc.body.appendChild(box);
  }

  const isOpen = () => box !== null && !box.hidden;

  function open() {
    if (!box) build();
    if (!box.isConnected) doc.body.appendChild(box); // the page replaced its body
    if (!isOpen()) {
      previous = doc.activeElement;
      entries = collect();
      box.hidden = false;
      render();
      input.select(); // the last query stays, selected, ready to be replaced
    }
    input.focus();
  }

  function close() {
    if (!isOpen()) return;
    box.hidden = true;
    pending = false;
    list.replaceChildren(); // drop the clones and the references to the page
    entries = [];
    hits = [];
    input.removeAttribute('aria-activedescendant');
    input.blur();
    const back = previous;
    previous = null;
    if (back && back !== doc.body && back.isConnected && back.focus) back.focus({ preventScroll: true });
  }

  // Every equation and the text of the heading before it, in document order.
  function collect() {
    const out = [];
    let heading = '';
    for (const el of doc.querySelectorAll('h1,h2,h3,h4,h5,h6,[data-mazer-src]')) {
      if (box.contains(el)) continue;
      if (/^h[1-6]$/i.test(el.tagName)) {
        heading = el.textContent.replace(/\s+/g, ' ').trim();
        continue;
      }
      const src = el.getAttribute('data-mazer-src');
      if (!src) continue;
      const norm = normalize(src);
      out.push({ el, src, norm, folded: fold(norm), heading });
    }
    return out;
  }

  // Typing schedules one render per frame, so a fast typist is never held
  // up by the rendering.
  function schedule() {
    if (pending) return;
    pending = true;
    later(() => {
      if (pending && isOpen()) render();
    });
  }

  function render() {
    pending = false;
    hits = search(input.value, entries);
    const shown = Math.min(hits.length, MAX_ROWS);
    const rows = [];
    for (let i = 0; i < shown; i++) rows.push(row(hits[i], i));
    list.replaceChildren(...rows);
    count.textContent = entries.length ? `${hits.length} of ${entries.length}` : '';
    let message = '';
    if (!entries.length) message = 'No equations on this page.';
    else if (!hits.length) message = 'No equation matches.';
    else if (hits.length > shown) message = `… and ${hits.length - shown} more, keep typing`;
    note.textContent = message;
    note.hidden = !message;
    live.textContent = hits.length ? `${hits.length} of ${entries.length} equations` : message;
    select(-1);
  }

  function row(hit, i) {
    const li = make('li', 'mazer-search-row', { id: `mazer-search-opt-${i}`, role: 'option', 'aria-selected': 'false', 'data-index': String(i) });
    const { el, norm, heading } = hit.item;
    if (heading) {
      const ctx = make('div', 'mazer-search-ctx');
      ctx.textContent = heading;
      li.appendChild(ctx);
    }
    const math = make('div', 'mazer-search-math', { 'aria-hidden': 'true' });
    math.appendChild(preview(el));
    const code = make('code', 'mazer-search-src');
    const [before, found, after] = split(norm, hit.start, hit.end);
    const mark = make('mark');
    mark.textContent = found;
    code.append(doc.createTextNode(before), mark, doc.createTextNode(after));
    li.append(math, code);
    return li;
  }

  // A copy of the rendered math: the `math` element of a display equation,
  // otherwise the element itself (inline math, LaTeX). Ids are stripped so
  // the page never gets a duplicate, and the source attribute so nothing
  // counts the copy as an equation.
  function preview(el) {
    const isMath = el.tagName.toLowerCase() === 'math';
    const copy = ((!isMath && el.querySelector('math')) || el).cloneNode(true);
    for (const e of [copy, ...copy.querySelectorAll('[id],[data-mazer-src]')]) {
      e.removeAttribute('id');
      e.removeAttribute('data-mazer-src');
    }
    copy.classList.remove('mazer-search-hit');
    return copy;
  }

  function select(i) {
    const old = list.children[active];
    if (old) old.setAttribute('aria-selected', 'false');
    active = i;
    const now = list.children[i];
    if (!now) {
      input.removeAttribute('aria-activedescendant');
      return;
    }
    now.setAttribute('aria-selected', 'true');
    input.setAttribute('aria-activedescendant', now.id);
    if (now.scrollIntoView) now.scrollIntoView({ block: 'nearest' });
  }

  function move(step) {
    const n = list.children.length;
    if (!n) return;
    if (active < 0) select(step > 0 ? 0 : n - 1);
    else select((active + step + n) % n);
  }

  // `i` indexes the rows on screen, which are always `hits[0..]`.
  function jump(i) {
    const hit = hits[i];
    if (!hit) return; // nothing to go to; the box stays open
    const target = hit.item.el;
    close();
    if (!target.isConnected) return; // re-rendered while the box was open
    const still = root.matchMedia && root.matchMedia('(prefers-reduced-motion: reduce)').matches;
    target.scrollIntoView({ block: 'center', behavior: still ? 'auto' : 'smooth' });
    flash(target);
  }

  function flash(el) {
    if (flashed) {
      clearTimeout(flashed.timer);
      flashed.el.classList.remove('mazer-search-hit');
    }
    el.classList.add('mazer-search-hit');
    const timer = setTimeout(() => {
      el.classList.remove('mazer-search-hit');
      flashed = null;
    }, HIGHLIGHT_MS);
    flashed = { el, timer };
  }

  function onInputKey(e) {
    if (e.isComposing) return;
    // Typed and then pressed Enter (or an arrow) within one frame: bring the
    // rows up to date with the query first.
    if (pending && /^(Enter|ArrowDown|ArrowUp)$/.test(e.key)) render();
    switch (e.key) {
      case 'ArrowDown': move(1); break;
      case 'ArrowUp': move(-1); break;
      case 'Enter': jump(active < 0 ? 0 : active); break;
      case 'Escape': close(); break;
      case 'Tab': break; // the input is the only stop in the dialog
      default: return;
    }
    // Handled here: the page's own key handlers (an Enter-to-click helper,
    // say) must not act on it as well.
    e.preventDefault();
    e.stopPropagation();
  }

  const editable = (t) =>
    !!t && (t.isContentEditable || /^(input|textarea|select)$/i.test(t.tagName || ''));

  // Listens on `document` in the bubble phase and stands aside when a
  // handler of the page has already taken the key (`defaultPrevented`).
  doc.addEventListener('keydown', (e) => {
    if (e.defaultPrevented || e.isComposing) return;
    const key = e.key || '';
    if ((e.ctrlKey || e.metaKey) && !e.altKey && !e.shiftKey && key.toLowerCase() === 'k') {
      e.preventDefault(); // or the browser focuses its own search bar
      if (isOpen()) close();
      else open();
    } else if (key === 'Escape' && isOpen()) {
      e.preventDefault();
      close();
    } else if (key === '/' && !e.ctrlKey && !e.metaKey && !e.altKey && !editable(e.target)) {
      // Shift is allowed: some layouts need it to type `/` at all.
      e.preventDefault();
      open();
    }
  });

  api.open = open;
  api.close = close;
})();
