// Tests for mazer-search.js, with no dependencies:
//
//     node assets/mazer-search.test.mjs [page.html]
//
// The script is evaluated in a fresh `vm` context per test, against the small
// fake DOM below (enough of `document`, elements, events and timers for what
// the script uses). The matching helpers are tested through
// `globalThis.__mazerSearch`; the box itself through key, input and click
// events dispatched on the fake DOM.
//
// With a page argument, every `data-mazer-src="..."` value in that file is
// decoded and must find itself when typed as a query.

import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import vm from 'node:vm';
import assert from 'node:assert/strict';

// Values made inside the vm context have that context's prototypes, which
// strict deepEqual tells apart; compare plain copies.
const plain = (x) => JSON.parse(JSON.stringify(x));

const SCRIPT = readFileSync(join(dirname(fileURLToPath(import.meta.url)), 'mazer-search.js'), 'utf8');

// ---- a fake DOM ----

class Event {
  constructor(type, init = {}) {
    Object.assign(this, { bubbles: true, ctrlKey: false, metaKey: false, altKey: false, shiftKey: false, isComposing: false }, init);
    this.type = type;
    this.defaultPrevented = false;
    this.stopped = false;
  }
  preventDefault() { this.defaultPrevented = true; }
  stopPropagation() { this.stopped = true; }
}

class Target {
  constructor() { this.listeners = {}; }
  addEventListener(type, fn) { (this.listeners[type] ||= []).push(fn); }
  removeEventListener(type, fn) { this.listeners[type] = (this.listeners[type] || []).filter((f) => f !== fn); }
  // Bubbles from the target through its ancestors to the document and window.
  dispatchEvent(event) {
    event.target = this;
    for (let node = this; node && !event.stopped; node = node.parentEvent()) {
      event.currentTarget = node;
      for (const fn of [...(node.listeners[event.type] || [])]) fn(event);
    }
    return !event.defaultPrevented;
  }
}

class Node extends Target {
  constructor(doc) { super(); this.ownerDocument = doc; this.parentNode = null; this.childNodes = []; }
  parentEvent() { return this.parentNode || (this === this.ownerDocument.documentElement ? this.ownerDocument : null); }
  get isConnected() {
    let n = this;
    while (n.parentNode) n = n.parentNode;
    return n === this.ownerDocument.documentElement;
  }
}

class Text extends Node {
  constructor(doc, data) { super(doc); this.data = String(data); }
  get textContent() { return this.data; }
  set textContent(v) { this.data = String(v); }
  cloneNode() { return new Text(this.ownerDocument, this.data); }
}

// Selectors: a comma list of compounds of `tag`, `.class`, `#id`, `[attr]`
// and `[attr="value"]`. No combinators.
function compile(selector) {
  const alts = selector.split(',').map((part) => {
    const tests = [];
    for (const m of part.trim().matchAll(/([a-z][a-z0-9-]*)|\.([\w-]+)|#([\w-]+)|\[([\w-]+)(?:="([^"]*)")?\]/gi)) {
      if (m[1]) tests.push((e) => e.tagName.toLowerCase() === m[1].toLowerCase());
      else if (m[2]) tests.push((e) => e.classList.contains(m[2]));
      else if (m[3]) tests.push((e) => e.getAttribute('id') === m[3]);
      else tests.push((e) => e.hasAttribute(m[4]) && (m[5] === undefined || e.getAttribute(m[4]) === m[5]));
    }
    return (e) => tests.every((t) => t(e));
  });
  return (e) => alts.some((t) => t(e));
}

class Element extends Node {
  constructor(doc, tag) {
    super(doc);
    this.tagName = tag.toLowerCase() === 'math' ? 'math' : tag.toUpperCase();
    this.attrs = new Map();
    this.style = {};
    this.value = '';
    this.scrolled = [];
    const self = this;
    this.classList = {
      list: () => (self.getAttribute('class') || '').split(/\s+/).filter(Boolean),
      contains(c) { return this.list().includes(c); },
      add(c) { if (!this.contains(c)) self.setAttribute('class', [...this.list(), c].join(' ')); },
      remove(c) { if (self.hasAttribute('class')) self.setAttribute('class', this.list().filter((x) => x !== c).join(' ')); },
    };
  }
  getAttribute(k) { return this.attrs.has(k) ? this.attrs.get(k) : null; }
  setAttribute(k, v) { this.attrs.set(k, String(v)); }
  removeAttribute(k) { this.attrs.delete(k); }
  hasAttribute(k) { return this.attrs.has(k); }
  get id() { return this.getAttribute('id') || ''; }
  set id(v) { this.setAttribute('id', v); }
  get className() { return this.getAttribute('class') || ''; }
  set className(v) { this.setAttribute('class', v); }
  get hidden() { return this.hasAttribute('hidden'); }
  set hidden(v) { if (v) this.setAttribute('hidden', ''); else this.removeAttribute('hidden'); }
  get isContentEditable() {
    for (let n = this; n instanceof Element; n = n.parentNode) {
      const v = n.getAttribute('contenteditable');
      if (v !== null) return v !== 'false';
    }
    return false;
  }
  get children() { return this.childNodes.filter((n) => n instanceof Element); }
  get textContent() { return this.childNodes.map((n) => n.textContent).join(''); }
  set textContent(v) { this.replaceChildren(); if (v !== '') this.appendChild(new Text(this.ownerDocument, v)); }
  appendChild(child) {
    if (child.parentNode) child.parentNode.removeChild(child);
    child.parentNode = this;
    this.childNodes.push(child);
    return child;
  }
  removeChild(child) {
    this.childNodes = this.childNodes.filter((n) => n !== child);
    child.parentNode = null;
    return child;
  }
  append(...nodes) { for (const n of nodes) this.appendChild(typeof n === 'string' ? new Text(this.ownerDocument, n) : n); }
  replaceChildren(...nodes) { for (const n of [...this.childNodes]) this.removeChild(n); this.append(...nodes); }
  remove() { if (this.parentNode) this.parentNode.removeChild(this); }
  contains(other) {
    for (let n = other; n; n = n.parentNode) if (n === this) return true;
    return false;
  }
  descendants() {
    const out = [];
    const walk = (n) => { for (const c of n.children) { out.push(c); walk(c); } };
    walk(this);
    return out;
  }
  matches(sel) { return compile(sel)(this); }
  closest(sel) {
    const test = compile(sel);
    for (let n = this; n instanceof Element; n = n.parentNode) if (test(n)) return n;
    return null;
  }
  querySelectorAll(sel) { return this.descendants().filter(compile(sel)); }
  querySelector(sel) { return this.querySelectorAll(sel)[0] || null; }
  cloneNode(deep) {
    const copy = new Element(this.ownerDocument, this.tagName);
    copy.attrs = new Map(this.attrs);
    if (deep) for (const c of this.childNodes) copy.appendChild(c.cloneNode(true));
    return copy;
  }
  focus() { this.ownerDocument.activeElement = this; }
  blur() { if (this.ownerDocument.activeElement === this) this.ownerDocument.activeElement = this.ownerDocument.body; }
  select() { this.selected = true; }
  scrollIntoView(opts) { this.scrolled.push(opts); }
}

class Document extends Target {
  constructor(win) {
    super();
    this.win = win;
    this.documentElement = new Element(this, 'html');
    this.head = this.documentElement.appendChild(new Element(this, 'head'));
    this.body = this.documentElement.appendChild(new Element(this, 'body'));
    this.activeElement = this.body;
  }
  parentEvent() { return this.win; }
  createElement(tag) { return new Element(this, tag); }
  createTextNode(text) { return new Text(this, text); }
  querySelectorAll(sel) { return [this.documentElement, ...this.documentElement.descendants()].filter(compile(sel)); }
  querySelector(sel) { return this.querySelectorAll(sel)[0] || null; }
}

class Window extends Target {
  constructor({ reducedMotion = false, deferFrames = false } = {}) {
    super();
    this.timers = [];
    this.frames = [];
    this.document = new Document(this);
    this.reducedMotion = reducedMotion;
    this.deferFrames = deferFrames;
  }
  parentEvent() { return null; }
  matchMedia(q) { return { matches: q.includes('reduce') && this.reducedMotion }; }
  // Runs at once, unless frames are deferred (then `runFrames` runs them).
  requestAnimationFrame(f) { if (this.deferFrames) this.frames.push(f); else f(); return 0; }
  runFrames() { const fs = this.frames; this.frames = []; for (const f of fs) f(); }
  setTimeout(f, ms) { const t = { f, ms }; this.timers.push(t); return t; }
  clearTimeout(t) { this.timers = this.timers.filter((x) => x !== t); }
  runTimers() { const ts = this.timers; this.timers = []; for (const t of ts) t.f(); }
}

// A page: `h(tag, attrs, ...children)` builds elements.
function page(build, opts) {
  const win = new Window(opts);
  const doc = win.document;
  const h = (tag, attrs = {}, ...kids) => {
    const e = doc.createElement(tag);
    for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, v);
    e.append(...kids);
    return e;
  };
  if (build) doc.body.append(...build(h));
  const ctx = vm.createContext({ window: win, document: doc, setTimeout: (f, ms) => win.setTimeout(f, ms), clearTimeout: (t) => win.clearTimeout(t) });
  // The script reads `requestAnimationFrame`/`matchMedia` from the global.
  ctx.requestAnimationFrame = (f) => win.requestAnimationFrame(f);
  ctx.matchMedia = (q) => win.matchMedia(q);
  const load = () => vm.runInContext(SCRIPT, ctx);
  load();
  const api = ctx.__mazerSearch;
  const box = () => doc.querySelectorAll('.mazer-search-backdrop');
  const input = () => doc.querySelector('.mazer-search-input');
  const isOpen = () => box().length === 1 && !box()[0].hidden;
  const rows = () => doc.querySelectorAll('.mazer-search-row');
  const key = (k, mods = {}, target = doc.activeElement) => {
    const e = new Event('keydown', { key: k, ...mods });
    target.dispatchEvent(e);
    return e;
  };
  const type = (text) => {
    input().value = text;
    input().dispatchEvent(new Event('input'));
  };
  const text = (sel) => (doc.querySelector(sel) || { textContent: '' }).textContent;
  return { win, doc, ctx, h, api, load, box, input, isOpen, rows, key, type, text };
}

// A note: two headings, display and inline equations, one LaTeX span.
const NOTE = (h) => [
  h('h1', {}, 'Dirac delta'),
  h('div', { class: 'mazer-equation', id: 'eq-mass', 'data-mazer-src': '(= (integral (funcapp rho x) x) m)' },
    h('math', { display: 'block' }, h('mi', {}, 'm')), h('span', { class: 'mazer-eqno' }, '(1)')),
  h('p', {}, 'Half is ', h('math', { display: 'inline', 'data-mazer-src': '(frac 1 2)' }, h('mstyle', { displaystyle: 'true' }, h('mn', {}, '1'))), '.'),
  h('h2', {}, 'More'),
  h('div', { class: 'mazer-equation', 'data-mazer-src': '(= (funcapp Delta x) (funcapp delta x))' }, h('math', { display: 'block' }, h('mi', {}, 'x'))),
  h('p', {}, h('span', { class: 'mazer-math', 'data-mazer-src': '(sqrt x)' }, '\\(\\sqrt{x}\\)')),
];

// ---- a tiny runner ----

const tests = [];
const test = (name, fn) => tests.push({ name, fn });

// ---- matching ----

const pure = () => page(null).api;
const USER = '(= (integral (funcapp rho x) x) (integral (jux m (funcapp delta (- x a))) x) m)';

test('normalize collapses whitespace and spaces inside parens', () => {
  const { normalize } = pure();
  assert.equal(normalize('(frac\n  1\t2)'), '(frac 1 2)');
  assert.equal(normalize('  ( frac 1 2 )  '), '(frac 1 2)');
  assert.equal(normalize('( (a b ) )'), '((a b))');
  assert.equal(normalize('(=\r\n    (funcapp delta (- t a))\n    )'), '(= (funcapp delta (- t a)))');
  assert.equal(normalize(''), '');
});

test('smart case: lowercase query ignores case, uppercase query does not', () => {
  const { matches } = pure();
  assert.ok(matches('delta', '(funcapp Delta x)'));
  assert.ok(matches('delta', '(funcapp delta x)'));
  assert.ok(matches('Delta', '(funcapp Delta x)'));
  assert.ok(!matches('Delta', '(funcapp delta x)'));
  assert.ok(!matches('DELTA', '(funcapp Delta x)'));
});

test('partial queries match', () => {
  const { matches } = pure();
  assert.ok(matches('(funcapp rh', '(funcapp rho x)'));
  assert.ok(matches('rho x)', '(funcapp rho x)'));
  assert.ok(matches('(fu', '(funcapp rho x)'));
});

test("the user's example", () => {
  const { matches } = pure();
  assert.ok(matches('(funcapp rho x)', USER));
  assert.ok(matches('( funcapp   rho x )', USER));
  assert.ok(matches('funcapp delta (- x', USER));
  assert.ok(matches('(funcapp\n  delta\n  (- x a))', USER));
  assert.ok(!matches('(funcapp sigma x)', USER));
});

test('an empty query matches everything, in order', () => {
  const { search } = pure();
  const items = [{ src: '(a)' }, { src: '(b)' }, { src: '(c)' }];
  assert.deepEqual(plain(search('', items).map((h) => h.item.src)), ['(a)', '(b)', '(c)']);
  assert.deepEqual(plain(search('   ', items).map((h) => h.item.src)), ['(a)', '(b)', '(c)']);
  assert.deepEqual(plain(search('(b', items).map((h) => h.item.src)), ['(b)']);
});

test('highlight splitting gives before, match and after', () => {
  const { search, split, normalize } = pure();
  const [hit] = search('FUNCAPP Rho', [{ src: '(funcapp Rho x)' }]);
  assert.equal(hit, undefined); // `FUNCAPP` is case-sensitive and absent
  const [h1] = search('( funcapp  rho', [{ src: '(= (FUNCAPP rho x)  m)' }]);
  const norm = normalize('(= (FUNCAPP rho x)  m)');
  assert.deepEqual(plain(split(norm, h1.start, h1.end)), ['(= ', '(FUNCAPP rho', ' x) m)']);
  assert.deepEqual(plain(split('abc', 0, 0)), ['', '', 'abc']);
});

test('folding keeps indices for characters whose lower case is longer', () => {
  const { search } = pure();
  const [hit] = search('x', [{ src: '(İ x)' }]);
  assert.equal(hit.start, 3);
});

// ---- the box, through the fake DOM ----

test('Ctrl+K opens, a second Ctrl+K closes and focus returns', () => {
  const p = page(NOTE);
  const before = p.doc.body.querySelector('p');
  p.doc.activeElement = before;
  const e = p.key('k', { ctrlKey: true });
  assert.ok(e.defaultPrevented, 'the browser shortcut is prevented');
  assert.ok(p.isOpen());
  assert.equal(p.doc.activeElement, p.input());
  assert.equal(p.rows().length, 4);
  p.key('k', { ctrlKey: true });
  assert.ok(!p.isOpen());
  assert.equal(p.doc.activeElement, before);
});

test('Cmd+K (Meta) opens; Ctrl+Shift+K and Ctrl+Alt+K do not', () => {
  const p = page(NOTE);
  assert.ok(!p.key('K', { ctrlKey: true, shiftKey: true }).defaultPrevented);
  assert.ok(!p.isOpen());
  assert.ok(!p.key('k', { ctrlKey: true, altKey: true }).defaultPrevented);
  assert.ok(!p.isOpen());
  p.key('k', { metaKey: true });
  assert.ok(p.isOpen());
});

test('/ opens from the body but not from an input, textarea, select or contenteditable', () => {
  const p = page((h) => [...NOTE(h), h('input'), h('textarea'), h('select'), h('div', { contenteditable: 'true' }, h('span', { class: 'editable' }))]);
  for (const sel of ['input', 'textarea', 'select', '.editable']) {
    const target = p.doc.body.querySelector(sel);
    const e = p.key('/', {}, target);
    assert.ok(!p.isOpen(), sel);
    assert.ok(!e.defaultPrevented, sel);
  }
  assert.ok(!p.key('/', { ctrlKey: true }, p.doc.body).defaultPrevented);
  assert.ok(!p.isOpen());
  const e = p.key('/', {}, p.doc.body);
  assert.ok(e.defaultPrevented);
  assert.ok(p.isOpen());
  // Typed into the box, `/` is just a character.
  assert.ok(!p.key('/').defaultPrevented);
  assert.ok(p.isOpen());
});

test('Escape closes, from the input or from elsewhere', () => {
  const p = page(NOTE);
  p.key('k', { ctrlKey: true });
  const e = p.key('Escape');
  assert.ok(!p.isOpen());
  assert.ok(e.stopped, 'the page does not see it');
  p.key('k', { ctrlKey: true });
  p.key('Escape', {}, p.doc.body);
  assert.ok(!p.isOpen());
});

test('a click on the backdrop closes; a click in the panel does not', () => {
  const p = page(NOTE);
  p.key('k', { ctrlKey: true });
  const [box] = p.box();
  const panel = p.doc.querySelector('.mazer-search-panel');
  panel.dispatchEvent(new Event('mousedown'));
  panel.dispatchEvent(new Event('click'));
  assert.ok(p.isOpen());
  // Pressed in the input, released over the backdrop: still open.
  p.input().dispatchEvent(new Event('mousedown'));
  box.dispatchEvent(new Event('click'));
  assert.ok(p.isOpen());
  box.dispatchEvent(new Event('mousedown'));
  box.dispatchEvent(new Event('click'));
  assert.ok(!p.isOpen());
});

test('typing filters, in document order, with a count and headings', () => {
  const p = page(NOTE);
  p.key('/', {}, p.doc.body);
  assert.equal(p.text('.mazer-search-count'), '4 of 4');
  const srcs = () => p.rows().map((r) => r.querySelector('.mazer-search-src').textContent);
  assert.deepEqual(srcs(), ['(= (integral (funcapp rho x) x) m)', '(frac 1 2)', '(= (funcapp Delta x) (funcapp delta x))', '(sqrt x)']);
  assert.deepEqual(p.rows().map((r) => r.querySelector('.mazer-search-ctx').textContent), ['Dirac delta', 'Dirac delta', 'More', 'More']);
  p.type('(funcapp  rh');
  assert.equal(p.text('.mazer-search-count'), '1 of 4');
  assert.equal(p.rows().length, 1);
  const mark = p.rows()[0].querySelector('mark');
  assert.equal(mark.textContent, '(funcapp rh');
  p.type('x');
  assert.deepEqual(srcs(), ['(= (integral (funcapp rho x) x) m)', '(= (funcapp Delta x) (funcapp delta x))', '(sqrt x)']);
  p.type('Delta');
  assert.deepEqual(srcs(), ['(= (funcapp Delta x) (funcapp delta x))']);
  assert.equal(p.text('.mazer-search-sr'), '1 of 4 equations');
});

test('rows hold a clone of the math, with no id or source attribute', () => {
  const p = page(NOTE);
  p.key('k', { ctrlKey: true });
  const previews = p.rows().map((r) => r.querySelector('.mazer-search-math').children[0]);
  assert.deepEqual(previews.map((e) => e.tagName.toLowerCase()), ['math', 'math', 'math', 'span']);
  assert.equal(previews[0].getAttribute('display'), 'block');
  assert.equal(p.doc.querySelectorAll('#eq-mass').length, 1);
  assert.equal(p.doc.querySelectorAll('[data-mazer-src]').length, 4);
  // Reopening does not count the clones.
  p.key('Escape');
  p.key('k', { ctrlKey: true });
  assert.equal(p.text('.mazer-search-count'), '4 of 4');
});

test('ids inside a copied equation are stripped; equations inside the box are not indexed', () => {
  // A LaTeX display equation has no `math` element: the div itself is copied.
  const p = page((h) => [
    h('div', { class: 'mazer-equation', id: 'eq-tex', 'data-mazer-src': '(sqrt y)' }, '\\[\\sqrt{y}\\]', h('span', { id: 'inner' }, '(1)')),
  ]);
  p.key('k', { ctrlKey: true });
  assert.equal(p.rows().length, 1);
  assert.equal(p.doc.querySelectorAll('#eq-tex').length, 1);
  assert.equal(p.doc.querySelectorAll('#inner').length, 1);
  assert.equal(p.doc.querySelectorAll('[data-mazer-src]').length, 1);
  // Something with the attribute put inside the box is not an equation of the page.
  p.doc.querySelector('.mazer-search-panel').append(p.h('span', { 'data-mazer-src': '(intruder)' }));
  p.key('Escape');
  p.key('k', { ctrlKey: true });
  assert.equal(p.text('.mazer-search-count'), '1 of 1');
});

test('ArrowDown and ArrowUp move the selection and wrap', () => {
  const p = page(NOTE);
  p.key('k', { ctrlKey: true });
  const selected = () => p.rows().findIndex((r) => r.getAttribute('aria-selected') === 'true');
  assert.equal(selected(), -1);
  p.key('ArrowDown');
  assert.equal(selected(), 0);
  assert.equal(p.input().getAttribute('aria-activedescendant'), p.rows()[0].id);
  p.key('ArrowUp');
  assert.equal(selected(), 3, 'wraps to the last');
  p.key('ArrowDown');
  assert.equal(selected(), 0, 'wraps to the first');
  p.key('ArrowDown');
  p.key('ArrowDown');
  assert.equal(selected(), 2);
  assert.equal(p.rows().filter((r) => r.getAttribute('aria-selected') === 'true').length, 1);
  p.type('(frac');
  assert.equal(selected(), -1, 'a new query clears the selection');
  assert.equal(p.input().getAttribute('aria-activedescendant'), null);
  // From no selection, ArrowUp goes to the last row.
  p.type('');
  p.key('ArrowUp');
  assert.equal(selected(), 3);
});

test('Enter jumps to the selected equation, highlights it and closes', () => {
  const p = page(NOTE);
  const target = p.doc.querySelectorAll('[data-mazer-src]')[2];
  p.key('k', { ctrlKey: true });
  p.key('ArrowDown');
  p.key('ArrowDown');
  p.key('ArrowDown');
  const enter = p.key('Enter');
  assert.ok(enter.stopped && enter.defaultPrevented);
  assert.ok(!p.isOpen());
  assert.deepEqual(plain(target.scrolled), [{ block: 'center', behavior: 'smooth' }]);
  assert.ok(target.classList.contains('mazer-search-hit'));
  assert.equal(p.win.timers.length, 1);
  assert.equal(p.win.timers[0].ms, 2000);
  p.win.runTimers();
  assert.ok(!target.classList.contains('mazer-search-hit'));
});

test('Enter with no selection jumps to the first match; inline math is highlighted itself', () => {
  const p = page(NOTE);
  const inline = p.doc.querySelector('math[data-mazer-src]');
  p.key('k', { ctrlKey: true });
  p.type('frac 1');
  p.key('Enter');
  assert.equal(inline.scrolled.length, 1);
  assert.ok(inline.classList.contains('mazer-search-hit'));
});

test('Enter with no match keeps the box open', () => {
  const p = page(NOTE);
  p.key('k', { ctrlKey: true });
  p.type('(nothing here');
  assert.equal(p.text('.mazer-search-note'), 'No equation matches.');
  p.key('Enter');
  assert.ok(p.isOpen());
});

test('clicking a row jumps there', () => {
  const p = page(NOTE);
  const latex = p.doc.querySelector('span[data-mazer-src]');
  p.key('k', { ctrlKey: true });
  p.rows()[3].querySelector('code').dispatchEvent(new Event('click'));
  assert.ok(!p.isOpen());
  assert.equal(latex.scrolled.length, 1);
  assert.ok(latex.classList.contains('mazer-search-hit'));
});

test('reduced motion scrolls without animation', () => {
  const p = page(NOTE, { reducedMotion: true });
  const first = p.doc.querySelector('[data-mazer-src]');
  p.key('k', { ctrlKey: true });
  p.key('Enter');
  assert.deepEqual(plain(first.scrolled), [{ block: 'center', behavior: 'auto' }]);
});

test('a second jump moves the highlight at once', () => {
  const p = page(NOTE);
  const [a, b] = p.doc.querySelectorAll('[data-mazer-src]');
  p.key('k', { ctrlKey: true });
  p.key('Enter');
  p.key('k', { ctrlKey: true });
  p.type('frac');
  p.key('Enter');
  assert.ok(!a.classList.contains('mazer-search-hit'));
  assert.ok(b.classList.contains('mazer-search-hit'));
  assert.equal(p.win.timers.length, 1);
});

test('Enter does not reach the page (an Enter-to-click helper on window)', () => {
  const p = page(NOTE);
  let seen = 0;
  p.win.addEventListener('keydown', (e) => { if (e.key === 'Enter') seen += 1; });
  p.key('k', { ctrlKey: true });
  p.key('ArrowDown');
  p.key('Enter');
  assert.equal(seen, 0);
});

test('a page handler that already took Ctrl+K wins', () => {
  const p = page(NOTE);
  p.doc.body.addEventListener('keydown', (e) => { if (e.key === 'k') e.preventDefault(); });
  p.key('k', { ctrlKey: true }, p.doc.body);
  assert.ok(!p.isOpen());
});

test('empty page, late rendering and the row cap', () => {
  const p = page(null);
  p.key('k', { ctrlKey: true });
  assert.equal(p.text('.mazer-search-note'), 'No equations on this page.');
  assert.equal(p.rows().length, 0);
  p.key('Escape');
  // Notes rendered after the script loaded are found on the next open.
  for (let i = 0; i < 60; i++) {
    p.doc.body.append(p.h('math', { display: 'inline', 'data-mazer-src': `(+ x ${i})` }));
  }
  p.key('k', { ctrlKey: true });
  assert.equal(p.rows().length, 50);
  assert.equal(p.text('.mazer-search-count'), '60 of 60');
  assert.equal(p.text('.mazer-search-note'), '… and 10 more, keep typing');
  p.type('(+ x 5');
  assert.equal(p.rows().length, 11); // 5 and 50..59
  assert.equal(p.doc.querySelector('.mazer-search-note').hidden, true);
});

test('typing renders once per frame, and Enter before the frame uses the new query', () => {
  const p = page(NOTE, { deferFrames: true });
  const sqrt = p.doc.querySelector('span[data-mazer-src]');
  p.key('k', { ctrlKey: true });
  p.key('ArrowDown');
  p.type('(sq');
  p.type('(sqrt');
  assert.equal(p.rows().length, 4, 'not rendered yet');
  assert.equal(p.win.frames.length, 1, 'one frame for both keystrokes');
  p.key('Enter');
  assert.ok(!p.isOpen());
  assert.equal(sqrt.scrolled.length, 1, 'the first row of the new query, not the old selection');
  p.win.runFrames(); // the stale frame does nothing on a closed box
  assert.ok(!p.isOpen());
});

test('a box removed by the page comes back on the next open', () => {
  const p = page(NOTE);
  p.key('k', { ctrlKey: true });
  p.key('Escape');
  p.box()[0].remove();
  p.key('k', { ctrlKey: true });
  assert.ok(p.isOpen());
  assert.equal(p.rows().length, 4);
});

test('loading the script twice installs one box and one set of listeners', () => {
  const p = page(NOTE);
  p.load();
  assert.equal(p.ctx.__mazerSearch, p.api, 'the second copy did nothing');
  assert.equal(p.doc.listeners.keydown.length, 1);
  p.key('k', { ctrlKey: true });
  assert.equal(p.box().length, 1);
  assert.ok(p.isOpen(), 'one listener: opened, not opened and closed again');
  assert.equal(p.doc.querySelectorAll('style').length, 1);
});

test('window.mazerSearch = false turns it off', () => {
  const win = new Window();
  const ctx = vm.createContext({ window: win, document: win.document, mazerSearch: false });
  vm.runInContext(SCRIPT, ctx);
  assert.equal(ctx.__mazerSearch, undefined);
  assert.equal(win.document.listeners.keydown, undefined);
});

test('the script builds markup only through the DOM', () => {
  assert.ok(!/innerHTML|outerHTML|insertAdjacentHTML|document\.write/.test(SCRIPT));
  assert.ok(!/<\/script/i.test(SCRIPT));
  assert.ok(!/<!--/.test(SCRIPT));
});

// ---- a real page, if given ----

const pagePath = process.argv[2];
if (pagePath) {
  test(`every equation of ${pagePath} finds itself`, () => {
    const html = readFileSync(pagePath, 'utf8');
    const decode = (s) => s.replace(/&(amp|lt|gt|quot|#39);/g, (_, e) => ({ amp: '&', lt: '<', gt: '>', quot: '"', '#39': "'" })[e]);
    const values = [...html.matchAll(/data-mazer-src="([^"]*)"/g)].map((m) => decode(m[1])).filter((v) => v.trim());
    const { matches } = pure();
    for (const v of values) assert.ok(matches(v, v), v);
    console.log(`# ${values.length} data-mazer-src values checked`);
  });
}

let failed = 0;
for (const { name, fn } of tests) {
  try {
    fn();
    console.log(`ok - ${name}`);
  } catch (e) {
    failed += 1;
    console.log(`not ok - ${name}\n${String(e && e.stack || e).replace(/^/gm, '  ')}`);
  }
}
if (failed) {
  console.log(`${failed} of ${tests.length} tests failed`);
  process.exit(1);
}
console.log(`all ${tests.length} tests passed`);
