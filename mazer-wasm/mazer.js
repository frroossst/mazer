// mazer auto-loader
// ------------------
// Drop this file plus the wasm-pack `pkg/` folder into your site (e.g. under
// `/mazer/`) and add ONE tag to your page:
//
//     <script type="module" src="/mazer/mazer.js"></script>
//
// Then anywhere you want a note rendered, write:
//
//     <div data-mazer="/notes/2026-06-14.zr"></div>
//
// On load this module boots the wasm `.zr` compiler, fetches each referenced
// file, compiles it to HTML, and replaces the element's contents. Each block is
// isolated: a missing file or a malformed note shows an inline error box but
// never stops the other blocks from rendering.
//
// Notes:
//   * The page must be served over http(s) — `fetch()` does not work on file://.
//   * Code blocks are syntax-highlighted by injecting Arborium after render.
//     Opt out by setting `window.mazerHighlight = false` before this module loads.
//   * Equation search: once every block is rendered, `pkg/mazer_search.js`
//     is loaded, so Ctrl+K (Cmd+K) or `/` opens a box that finds an equation
//     on the page by its source. If it cannot be loaded, the notes are not
//     affected (a warning goes to the console). Opt out by setting
//     `window.mazerSearch = false` before this module loads.

import init, { run_mazer } from './pkg/mazer_wasm.js';

const ARBORIUM =
  'https://cdn.jsdelivr.net/npm/@arborium/arborium/dist/arborium.iife.js';
const SEARCH_URL = new URL('./pkg/mazer_search.js', import.meta.url);

async function renderAll() {
  // `init()` with no argument resolves `mazer_wasm_bg.wasm` relative to the
  // pkg module URL, so this works no matter what path `/mazer/` is mounted at.
  await init();

  const blocks = [...document.querySelectorAll('[data-mazer]')];
  let hasCode = false;

  await Promise.all(
    blocks.map(async (el) => {
      const src = el.getAttribute('data-mazer');
      try {
        const res = await fetch(src);
        if (!res.ok) throw new Error(`${res.status} ${res.statusText}`);
        const text = await res.text();
        el.innerHTML = run_mazer(text, src);
        if (el.querySelector('pre code')) hasCode = true;
      } catch (e) {
        console.error(`[mazer] failed to render ${src}:`, e);
        const msg = `mazer: failed to load ${src}\n${e}`;
        const pre = document.createElement('pre');
        pre.className = 'mazer-error';
        pre.textContent = msg; // textContent escapes automatically
        el.replaceChildren(pre);
      }
    })
  );

  // Inject the syntax highlighter AFTER content is in the DOM, so its on-load
  // scan picks up the freshly inserted `pre code` elements.
  if (hasCode && window.mazerHighlight !== false) {
    const s = document.createElement('script');
    s.src = ARBORIUM;
    s.setAttribute('data-selector', 'pre code');
    s.setAttribute('data-theme', 'github-light');
    document.head.appendChild(s);
  }

  // The search box indexes the page each time it opens, so it only has to
  // be loaded once; it is imported for its side effect.
  if (window.mazerSearch !== false) {
    try {
      await import(SEARCH_URL.href);
    } catch (e) {
      console.warn('[mazer] equation search not available:', e);
    }
  }
}

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', renderAll);
} else {
  renderAll();
}
