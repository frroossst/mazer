// mazer equation search
// ---------------------
// Ctrl+K (Cmd+K on macOS), `/`, or the small round button in the corner
// opens a box that searches the equations on THIS page by their source, as
// the author typed it: `(funcapp rho x)` finds every equation whose source
// contains that text. Enter or a click scrolls to the equation, highlights it
// for a moment and moves the keyboard focus there.
//
// It relies only on the `data-mazer-src` attribute mazer writes on every
// rendered equation (`div.mazer-equation` for display math, the `math`
// element for inline math, `span.mazer-math` for the LaTeX target).
//
// The query is compared after normalising whitespace on both sides. A form
// copied whole from a note, `(show (tag e) (frac 1 2))`, also works: the
// leading `(show `/`(eval `, a `(tag name)` and the form's own closing paren
// are dropped from the query. Pasted text loses its `;` comments and its line
// breaks become spaces. Exact matches come first, then matches that start and
// end at token boundaries, then the rest.
//
// One dependency-free file that works inline in a script element, loaded by
// a script element's `src`, and when import()-ed as a module for its side
// effect. Loading it twice installs it once. The index is rebuilt every time
// the box opens, so it does not matter when (or how often) the notes were
// rendered. `window.mazerSearch = false` before it loads turns it off;
// `window.mazerSearchButton = false` keeps only the keyboard shortcuts.
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

  // One space between tokens, none just inside a paren, one between `)` and
  // `(`: `( frac  1\n2 )` and `(frac 1 2)` normalise to the same text, and
  // so do `x)(y` and `x) (y`.
  const normalize = (s) =>
    String(s).replace(/\s+/g, ' ').trim().replace(/\( /g, '(').replace(/ \)/g, ')').replace(/\)\(/g, ') (');

  // Removes `;` comments, to the end of their line, outside string literals
  // (where `\"` and `\\` are escapes), as the mazer reader does.
  function stripComments(text) {
    let out = '';
    let inString = false;
    for (let i = 0; i < text.length; i++) {
      const c = text[i];
      if (inString) {
        out += c;
        if (c === '\\' && i + 1 < text.length) out += text[++i];
        else if (c === '"') inString = false;
      } else if (c === ';') {
        while (i + 1 < text.length && text[i + 1] !== '\n') i++;
      } else {
        if (c === '"') inString = true;
        out += c;
      }
    }
    return out;
  }

  // Pasted text as it goes into the one-line input: no comments, and line
  // breaks as spaces (a browser would delete them, gluing `(funcapp` and
  // `rho` of two lines together).
  const pasteText = (text) => stripComments(String(text)).replace(/\r\n|[\r\n]/g, ' ');

  // The balance of parens outside strings, and the lowest it gets.
  function parens(s) {
    let depth = 0;
    let low = 0;
    let inString = false;
    for (let i = 0; i < s.length; i++) {
      const c = s[i];
      if (inString) {
        if (c === '\\') i++;
        else if (c === '"') inString = false;
      } else if (c === '"') inString = true;
      else if (c === '(') depth++;
      else if (c === ')') low = Math.min(low, --depth);
    }
    return { depth, low };
  }

  // A normalised query without what the stored sources leave out: a leading
  // `(show ` or `(eval `, then a `(tag name)`, and the form's own `)` when it
  // is the one paren too many. `(show` alone (or `(show (tag`) leaves
  // nothing, which lists every equation, as an empty query does.
  function unwrap(q) {
    const head = /^\((?:show|eval)(?: |$)/.exec(q);
    if (!head) return q;
    let rest = q.slice(head[0].length);
    const tag = /^\(tag (?:[^\s()"]+|"(?:[^"\\]|\\.)*")\)(?: |$|(?=\)))/.exec(rest);
    if (tag) rest = rest.slice(tag[0].length);
    else if (/^\(tag(?: [^\s()"]*)?$/.test(rest)) rest = '';
    if (rest.endsWith(')')) {
      const all = parens(rest);
      const before = parens(rest.slice(0, -1));
      if (all.depth === -1 && before.low >= 0) rest = rest.slice(0, -1).trimEnd();
    }
    return rest;
  }

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

  const DELIMITER = /[ ()]/;

  // Whether the match of `needle` at `at` in `hay` neither starts nor ends
  // in the middle of a token: each end is the end of the source, next to a
  // space or a paren, or is itself a space or a paren.
  function bounded(hay, needle, at) {
    const end = at + needle.length;
    const starts = at === 0 || DELIMITER.test(hay[at - 1]) || DELIMITER.test(needle[0]);
    const ends = end === hay.length || DELIMITER.test(hay[end]) || DELIMITER.test(needle[needle.length - 1]);
    return starts && ends;
  }

  // `items` are `{ src }` (or index entries that also carry `norm`/`folded`).
  // Returns `{ item, start, end, rank }`: rank 0 for a source that is the
  // whole query, 1 for a match on token boundaries, 2 for the rest; by rank,
  // then in input order. `start`/`end` delimit the match in the normalised
  // source (the first one on token boundaries, if any).
  function search(query, items) {
    const q = unwrap(normalize(query));
    const sensitive = caseSensitive(q);
    const needle = sensitive ? q : fold(q);
    const ranks = [[], [], []];
    for (const item of items) {
      const norm = item.norm ?? normalize(item.src);
      const hay = sensitive ? norm : (item.folded ?? fold(norm));
      let at = hay.indexOf(needle);
      if (at < 0) continue;
      let rank = 2;
      if (hay === needle) rank = 0;
      else if (needle) {
        for (let i = at; i >= 0; i = hay.indexOf(needle, i + 1)) {
          if (bounded(hay, needle, i)) {
            at = i;
            rank = 1;
            break;
          }
        }
      }
      ranks[rank].push({ item, start: at, end: at + needle.length, rank });
    }
    return ranks[0].concat(ranks[1], ranks[2]);
  }

  const matches = (query, src) => search(query, [{ src }]).length > 0;

  // [before, matched, after]
  const split = (text, start, end) => [text.slice(0, start), text.slice(start, end), text.slice(end)];

  const api = { normalize, unwrap, stripComments, pasteText, matches, search, split, MAX_ROWS };
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
.mazer-search-fab{position:fixed;right:12px;bottom:12px;bottom:calc(12px + env(safe-area-inset-bottom,0px));z-index:2147483645;display:flex;align-items:center;justify-content:center;width:36px;height:36px;margin:0;padding:0;font:inherit;color:CanvasText;background:Canvas;border:1px solid GrayText;border-radius:50%;box-shadow:0 1px 4px rgba(0,0,0,.2);opacity:.55;cursor:pointer}
.mazer-search-fab:hover,.mazer-search-fab:focus-visible{opacity:1}
.mazer-search-fab svg{display:block;width:18px;height:18px}
@media (max-width:480px){.mazer-search-backdrop{padding:8px}.mazer-search-panel{max-height:calc(100vh - 16px)}}
@media print{.mazer-search-fab,.mazer-search-backdrop{display:none!important}}
`;

  let style = null; // the style element; added once, on first need
  let box = null; // the backdrop; built on the first open
  let button = null; // the floating button, once the page has equations
  let input, list, count, note, live;
  let entries = []; // { el, src, norm, folded, heading } in document order
  let hits = [];
  let active = -1; // selected row, -1 for none
  let previous = null; // focused before the box opened
  let pending = false; // a render is scheduled
  let pressedOnBackdrop = false;
  let flashed = null; // the highlighted equation and its timer
  let tabbed = null; // the equation given a temporary tabindex to be focused
  const later = root.requestAnimationFrame ? (f) => root.requestAnimationFrame(f) : (f) => setTimeout(f, 16);

  function make(tag, className, attrs) {
    const el = doc.createElement(tag);
    if (className) el.className = className;
    for (const [k, v] of Object.entries(attrs || {})) el.setAttribute(k, v);
    return el;
  }

  function addStyle() {
    if (style && style.isConnected) return;
    style = make('style');
    style.textContent = CSS;
    (doc.head || doc.documentElement).appendChild(style);
  }

  function build() {
    addStyle();
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
    input.addEventListener('paste', onPaste);
    doc.body.appendChild(box);
  }

  const isOpen = () => box !== null && !box.hidden;

  function open() {
    if (!box) build();
    addStyle(); // the page may have replaced its head
    if (!box.isConnected) doc.body.appendChild(box); // ... or its body
    if (!isOpen()) {
      previous = doc.activeElement;
      entries = collect();
      box.hidden = false;
      render();
      input.select(); // the last query stays, selected, ready to be replaced
    }
    input.focus();
  }

  // Closes the box; focus goes back where it was unless `restore` is false
  // (a jump moves it to the equation instead).
  function close(restore) {
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
    if (restore !== false && back && back !== doc.body && back.isConnected && back.focus) back.focus({ preventScroll: true });
  }

  // A heading's text, with each equation in it as its source rather than
  // the text of its rendered math.
  function headingText(heading) {
    let text = '';
    const walk = (node) => {
      for (const child of node.childNodes) {
        if (child.nodeType === 3) text += child.data;
        else if (child.nodeType === 1) {
          const src = child.getAttribute('data-mazer-src');
          if (src) text += src;
          else walk(child);
        }
      }
    };
    walk(heading);
    return text.replace(/\s+/g, ' ').trim();
  }

  // Every equation and the text of the heading before it, in document order.
  function collect() {
    const out = [];
    let heading = '';
    for (const el of doc.querySelectorAll('h1,h2,h3,h4,h5,h6,[data-mazer-src]')) {
      if (box.contains(el)) continue;
      if (/^h[1-6]$/i.test(el.tagName)) {
        heading = headingText(el);
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
    for (const e of [copy, ...copy.querySelectorAll('[id],[data-mazer-src],[tabindex]')]) {
      e.removeAttribute('id');
      e.removeAttribute('data-mazer-src');
      e.removeAttribute('tabindex');
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
    let hit = hits[i];
    if (!hit) return; // nothing to go to; the box stays open
    if (!hit.item.el.isConnected) {
      // The page re-rendered while the box was open: index it again, run the
      // query again, and go to the same source if it is still there.
      const { src } = hit.item;
      entries = collect();
      render();
      hit = hits.find((h) => h.item.src === src) || hits[0];
      if (!hit) return; // stays open, showing what there is now
    }
    const target = hit.item.el;
    close(false);
    const still = root.matchMedia && root.matchMedia('(prefers-reduced-motion: reduce)').matches;
    target.scrollIntoView({ block: 'center', behavior: still ? 'auto' : 'smooth' });
    flash(target);
    focusOn(target);
  }

  // Moves the keyboard focus (and a screen reader) to the equation, without
  // scrolling again. An element that cannot take focus gets `tabindex="-1"`
  // until its highlight goes.
  function focusOn(el) {
    if (tabbed && tabbed !== el) untab(tabbed);
    if (typeof el.focus !== 'function') return;
    if (!el.hasAttribute('tabindex') && !(el.tabIndex >= 0)) {
      el.setAttribute('tabindex', '-1');
      tabbed = el;
    }
    el.focus({ preventScroll: true });
  }

  function untab(el) {
    if (tabbed !== el) return;
    tabbed = null;
    el.removeAttribute('tabindex');
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
      // Still focused: the tabindex goes when the focus leaves, so the
      // focus is not dropped from under the reader.
      if (tabbed === el && doc.activeElement === el) el.addEventListener('blur', () => untab(el), { once: true });
      else untab(el);
    }, HIGHLIGHT_MS);
    flashed = { el, timer };
  }

  // The composition-committing Enter arrives with `isComposing` or, in
  // Safari, as keyCode 229.
  const composing = (e) => e.isComposing || e.keyCode === 229;

  function onInputKey(e) {
    if (composing(e)) return;
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

  // Pastes the text without comments and with its line breaks as spaces, at
  // the caret, then searches as for typing.
  function onPaste(e) {
    const data = e.clipboardData;
    const text = data && data.getData ? data.getData('text/plain') : '';
    if (!text) return;
    e.preventDefault();
    const clean = pasteText(text);
    // `insertText` keeps the browser's undo history and fires `input`.
    let done = false;
    try {
      done = typeof doc.execCommand === 'function' && doc.execCommand('insertText', false, clean);
    } catch (err) {
      done = false;
    }
    if (done) return;
    const value = input.value;
    const start = input.selectionStart ?? value.length;
    const end = input.selectionEnd ?? start;
    if (typeof input.setRangeText === 'function') input.setRangeText(clean, start, end, 'end');
    else input.value = value.slice(0, start) + clean + value.slice(end);
    schedule();
  }

  const editable = (t) =>
    !!t && (t.isContentEditable || /^(input|textarea|select)$/i.test(t.tagName || ''));

  // Listens on `document` in the bubble phase and stands aside when a
  // handler of the page has already taken the key (`defaultPrevented`).
  doc.addEventListener('keydown', (e) => {
    if (e.defaultPrevented || composing(e)) return;
    const key = e.key || '';
    if ((e.ctrlKey || e.metaKey) && !e.altKey && !e.shiftKey && key.toLowerCase() === 'k') {
      e.preventDefault(); // or the browser focuses its own search bar
      if (e.repeat) return; // held down: one toggle, not a flicker
      if (isOpen()) close();
      else open();
    } else if (key === 'Escape' && isOpen()) {
      e.preventDefault();
      close();
    } else if (key === '/' && !e.ctrlKey && !e.metaKey && !e.altKey && !e.repeat) {
      // Inside a shadow root `e.target` is the host; the real target is
      // the first entry of the composed path.
      const target = (e.composedPath && e.composedPath()[0]) || e.target;
      if (editable(target)) return;
      // Shift is allowed: some layouts need it to type `/` at all.
      e.preventDefault();
      open();
    }
  });

  // ---- the button, for devices without a keyboard ----

  const wantButton = root.mazerSearchButton !== false;

  function glass() {
    const ns = 'http://www.w3.org/2000/svg';
    const svg = doc.createElementNS(ns, 'svg');
    for (const [k, v] of Object.entries({
      viewBox: '0 0 24 24', fill: 'none', stroke: 'currentColor', 'stroke-width': '2.2',
      'stroke-linecap': 'round', 'aria-hidden': 'true', focusable: 'false',
    })) svg.setAttribute(k, v);
    const circle = doc.createElementNS(ns, 'circle');
    circle.setAttribute('cx', '10.5');
    circle.setAttribute('cy', '10.5');
    circle.setAttribute('r', '6.5');
    const handle = doc.createElementNS(ns, 'path');
    handle.setAttribute('d', 'M15.5 15.5 21 21');
    svg.append(circle, handle);
    return svg;
  }

  // Adds the button once the page has an equation. Returns whether it is
  // there (or will never be).
  function addButton() {
    if (button || !wantButton) return true;
    if (!doc.body || !doc.querySelector('[data-mazer-src]')) return false;
    addStyle();
    button = make('button', 'mazer-search-fab', { type: 'button', 'aria-label': 'Search equations', title: 'Search equations (Ctrl+K or /)' });
    button.appendChild(glass());
    button.addEventListener('click', () => open());
    doc.body.appendChild(button);
    return true;
  }

  // At load, or else at the first interaction after the notes are rendered.
  const INTERACTIONS = ['pointerdown', 'touchstart', 'keydown', 'focusin'];
  function onInteraction() {
    if (addButton()) for (const type of INTERACTIONS) doc.removeEventListener(type, onInteraction, true);
  }
  function watchForEquations() {
    if (addButton()) return;
    for (const type of INTERACTIONS) doc.addEventListener(type, onInteraction, true);
  }
  if (doc.readyState === 'loading') doc.addEventListener('DOMContentLoaded', watchForEquations);
  else watchForEquations();

  api.open = open;
  api.close = () => close();
})();
