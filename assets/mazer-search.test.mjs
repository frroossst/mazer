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

// Whether `el` has the focus. Not `assert.equal`: its message would print
// two fake DOM trees in full.
const focused = (p, el, message = 'the focus is elsewhere') => assert.ok(p.doc.activeElement === el, message);

const SCRIPT = readFileSync(join(dirname(fileURLToPath(import.meta.url)), 'mazer-search.js'), 'utf8');

// ---- a fake DOM ----

class Event {
  constructor(type, init = {}) {
    Object.assign(this, { bubbles: true, ctrlKey: false, metaKey: false, altKey: false, shiftKey: false, isComposing: false, repeat: false }, init);
    this.type = type;
    this.defaultPrevented = false;
    this.stopped = false;
  }
  preventDefault() { this.defaultPrevented = true; }
  stopPropagation() { this.stopped = true; }
  // `path` (given in `init`) stands for a path through a shadow root.
  composedPath() { return this.path || [this.target]; }
}

class Target {
  constructor() { this.listeners = {}; }
  // The capture flag is ignored; `{ once: true }` is honoured.
  addEventListener(type, fn, opts) {
    const once = typeof opts === 'object' && opts !== null && opts.once;
    const listener = once ? Object.assign((e) => { this.removeEventListener(type, listener); fn(e); }, { original: fn }) : fn;
    (this.listeners[type] ||= []).push(listener);
  }
  removeEventListener(type, fn) { this.listeners[type] = (this.listeners[type] || []).filter((f) => f !== fn && f.original !== fn); }
  // Bubbles from the target through its ancestors to the document and window
  // (or stays on the target when it does not bubble).
  dispatchEvent(event) {
    event.target = this;
    for (let node = this; node && !event.stopped; node = event.bubbles ? node.parentEvent() : null) {
      event.currentTarget = node;
      for (const fn of [...(node.listeners[event.type] || [])]) fn(event);
    }
    return !event.defaultPrevented;
  }
}

class Node extends Target {
  constructor(doc, nodeType) { super(); this.nodeType = nodeType; this.ownerDocument = doc; this.parentNode = null; this.childNodes = []; }
  parentEvent() { return this.parentNode || (this === this.ownerDocument.documentElement ? this.ownerDocument : null); }
  get isConnected() {
    let n = this;
    while (n.parentNode) n = n.parentNode;
    return n === this.ownerDocument.documentElement;
  }
}

class Text extends Node {
  constructor(doc, data) { super(doc, 3); this.data = String(data); }
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
  constructor(doc, tag, ns) {
    super(doc, 1);
    this.namespaceURI = ns || null;
    this.tagName = ns || tag.toLowerCase() === 'math' ? tag : tag.toUpperCase();
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
    const copy = new Element(this.ownerDocument, this.tagName, this.namespaceURI);
    copy.attrs = new Map(this.attrs);
    if (deep) for (const c of this.childNodes) copy.appendChild(c.cloneNode(true));
    return copy;
  }
  // As in a browser: focusable natively (0) or by a tabindex, else -1.
  get tabIndex() {
    if (this.hasAttribute('tabindex')) return Number(this.getAttribute('tabindex'));
    return /^(A|BUTTON|INPUT|SELECT|TEXTAREA)$/.test(this.tagName) ? 0 : -1;
  }
  // Records the options of every call; the element that loses the focus
  // gets a `blur` event.
  focus(opts) {
    this.focusCalls = [...(this.focusCalls || []), opts];
    const doc = this.ownerDocument;
    const old = doc.activeElement;
    doc.activeElement = this;
    if (old && old !== this) old.dispatchEvent(new Event('blur', { bubbles: false }));
  }
  blur() {
    const doc = this.ownerDocument;
    if (doc.activeElement !== this) return;
    doc.activeElement = doc.body;
    this.dispatchEvent(new Event('blur', { bubbles: false }));
  }
  select() { this.selected = true; }
  setRangeText(text, start, end, mode) {
    this.value = this.value.slice(0, start) + text + this.value.slice(end);
    if (mode === 'end') this.selectionStart = this.selectionEnd = start + text.length;
  }
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
  createElementNS(ns, tag) { return new Element(this, tag, ns); }
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
// `opts.globals` are set on the global object before the script runs.
function page(build, opts = {}) {
  const win = new Window(opts);
  const doc = win.document;
  const h = (tag, attrs = {}, ...kids) => {
    const e = doc.createElement(tag);
    for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, v);
    e.append(...kids);
    return e;
  };
  if (build) doc.body.append(...build(h));
  const ctx = vm.createContext({
    window: win, document: doc, setTimeout: (f, ms) => win.setTimeout(f, ms), clearTimeout: (t) => win.clearTimeout(t),
    ...opts.globals,
  });
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
  // A paste of `clip` into the box's input; returns the event.
  const paste = (clip) => {
    const e = new Event('paste', { clipboardData: { getData: (t) => (t === 'text/plain' ? clip : '') } });
    input().dispatchEvent(e);
    return e;
  };
  const srcs = () => rows().map((r) => r.querySelector('.mazer-search-src').textContent);
  const button = () => doc.querySelectorAll('.mazer-search-fab');
  return { win, doc, ctx, h, api, load, box, input, isOpen, rows, key, type, text, paste, srcs, button };
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

test('normalize puts a space between ) and (', () => {
  const { normalize, matches } = pure();
  assert.equal(normalize('(f x)(g y)'), '(f x) (g y)');
  assert.equal(normalize('a)()(b'), 'a) () (b');
  assert.equal(normalize('x) \n (integral'), 'x) (integral');
  assert.ok(matches('x)(integral', '(+ (funcapp rho x) (integral y))'));
  assert.ok(matches('(funcapp rho x) (integral', '(+ (funcapp rho x)(integral y))'));
  // Only `)(`: a name glued to a paren stays glued.
  assert.equal(normalize('f(x)'), 'f(x)');
});

test('a whole copied form searches for its expressions', () => {
  const { unwrap, normalize, matches } = pure();
  const u = (q) => unwrap(normalize(q));
  assert.equal(u('(show (frac 1 2))'), '(frac 1 2)');
  assert.equal(u('(eval (+ x 1))'), '(+ x 1)');
  assert.equal(u('(show a b c)'), 'a b c');
  assert.equal(u('(show (tag mass) (= (funcapp Delta x) (funcapp delta x)))'), '(= (funcapp Delta x) (funcapp delta x))');
  // Partial pastes: no closing paren yet, or not even a whole expression.
  assert.equal(u('(show (frac 1 2)'), '(frac 1 2)');
  assert.equal(u('(show (tag mass) (= (funcapp Delta x)'), '(= (funcapp Delta x)');
  assert.equal(u('(show (tag e) x'), 'x');
  assert.equal(u('(show (tag "a b\\" c") (= q 1))'), '(= q 1)');
  // Nothing left: every equation, as for an empty query.
  for (const q of ['(show', '(show ', '(eval', '(show (tag', '(show (tag ma', '(show (tag mass)', '(show (tag mass))']) {
    assert.equal(u(q), '', q);
  }
  // Not a show: left alone.
  for (const q of ['(shower x)', '(sh', 'show (frac 1 2))', '(frac 1 2))', '(+ (show']) assert.equal(u(q), normalize(q), q);
  // A paren in a string does not count.
  assert.equal(u('(show (text ")"))'), '(text ")")');
  assert.equal(u('(show (text "("))'), '(text "(")');
  // Through `search`, with the stored sources (no show, no tag).
  assert.ok(matches('(show (frac 1 2))', '(frac 1 2)'));
  assert.ok(matches('(show (tag mass) (= (funcapp Delta x)', '(= (funcapp Delta x) (funcapp delta x))'));
  assert.ok(matches('  ( show   (frac 1 2) ) ', '(+ (frac 1 2) 1)'));
  assert.ok(!matches('(show (frac 1 3))', '(frac 1 2)'));
  const { search } = pure();
  const items = [{ src: '(a)' }, { src: '(b)' }];
  assert.equal(search('(show', items).length, 2);
});

test('comments are stripped outside strings; pasted lines become spaces', () => {
  const { stripComments, pasteText, matches } = pure();
  assert.equal(stripComments('(a ; c\n b)'), '(a \n b)');
  assert.equal(stripComments('(a;c\nb) ; last'), '(a\nb) ');
  assert.equal(stripComments('(text "a ; b") ; c'), '(text "a ; b") ');
  assert.equal(stripComments('(text "a\\" ; b") ; c'), '(text "a\\" ; b") ');
  assert.equal(stripComments('(text "a\\\\") ; c'), '(text "a\\\\") ');
  assert.equal(pasteText('(funcapp\nrho x)'), '(funcapp rho x)');
  assert.equal(pasteText('(funcapp\r\nrho\rx) ; c\r\n'), '(funcapp rho x)  ');
  const typed = '(show (= (integral (funcapp rho x) x)\n         (integral (jux m (funcapp delta (- x a))) x)   ; a comment\n         m))';
  assert.ok(matches(pasteText(typed), USER));
});

test('ranking: exact sources, then whole tokens, then the rest, in document order', () => {
  const { search } = pure();
  const items = [{ src: '(+ nn 1)' }, { src: '(+ a n)' }, { src: 'n' }, { src: '(f n)' }, { src: 'nm' }];
  const hits = search('n', items);
  assert.deepEqual(plain(hits.map((h) => h.item.src)), ['n', '(+ a n)', '(f n)', '(+ nn 1)', 'nm']);
  assert.deepEqual(plain(hits.map((h) => h.rank)), [0, 1, 1, 2, 2]);
  // The highlighted match is the first one on token boundaries.
  const [hit] = search('n', [{ src: '(+ nn n)' }]);
  assert.equal(hit.start, 6);
  // A query that starts or ends with a paren is on a boundary there.
  assert.equal(search('(frac 1', [{ src: 'a(frac 1 2)' }])[0].rank, 1);
  assert.equal(search('2)', [{ src: '(frac 1 2)b' }])[0].rank, 1);
  // Case-insensitive exact match.
  assert.equal(search('delta', [{ src: 'Delta' }])[0].rank, 0);
});

test('a short equation after 60 longer ones is the first row', () => {
  const p = page((h) => [
    ...Array.from({ length: 60 }, (_, i) => h('math', { display: 'inline', 'data-mazer-src': `(+ n ${i})` })),
    h('div', { class: 'mazer-equation', 'data-mazer-src': 'n' }),
  ]);
  p.key('k', { ctrlKey: true });
  p.type('n');
  assert.equal(p.srcs()[0], 'n');
  assert.equal(p.rows().length, 50);
  assert.equal(p.text('.mazer-search-count'), '61 of 61', 'the count is the total');
  const target = p.doc.querySelector('div.mazer-equation');
  p.key('Enter');
  assert.ok(target.classList.contains('mazer-search-hit'));
});

// ---- the box, through the fake DOM ----

test('Ctrl+K opens, a second Ctrl+K closes and focus returns', () => {
  const p = page(NOTE);
  const before = p.doc.body.querySelector('p');
  p.doc.activeElement = before;
  const e = p.key('k', { ctrlKey: true });
  assert.ok(e.defaultPrevented, 'the browser shortcut is prevented');
  assert.ok(p.isOpen());
  focused(p, p.input());
  assert.equal(p.rows().length, 4);
  p.key('k', { ctrlKey: true });
  assert.ok(!p.isOpen());
  focused(p, before);
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
  assert.ok(p.ctx.__mazerSearch === p.api, 'the second copy did nothing');
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

test('holding Ctrl+K does not toggle the box', () => {
  const p = page(NOTE);
  p.key('k', { ctrlKey: true });
  for (let i = 0; i < 5; i++) {
    const e = p.key('k', { ctrlKey: true, repeat: true });
    assert.ok(e.defaultPrevented, 'the browser shortcut stays prevented');
    assert.ok(p.isOpen(), `repeat ${i}`);
  }
  p.key('k', { ctrlKey: true });
  assert.ok(!p.isOpen());
  for (let i = 0; i < 3; i++) p.key('k', { ctrlKey: true, repeat: true }, p.doc.body);
  assert.ok(!p.isOpen(), 'a repeat does not open it either');
  // A held `/` opens nothing on its repeats.
  p.key('/', { repeat: true }, p.doc.body);
  assert.ok(!p.isOpen());
});

test('/ in an input inside a shadow root does not open the box', () => {
  const p = page((h) => [...NOTE(h), h('my-widget')]);
  const host = p.doc.querySelector('my-widget');
  const inner = p.doc.createElement('input'); // in the shadow tree, not a child
  const e = new Event('keydown', { key: '/', path: [inner, host, p.doc.body] });
  host.dispatchEvent(e);
  assert.ok(!p.isOpen());
  assert.ok(!e.defaultPrevented);
  // A non-editable element in the shadow tree: it opens.
  const span = p.doc.createElement('span');
  host.dispatchEvent(new Event('keydown', { key: '/', path: [span, host, p.doc.body] }));
  assert.ok(p.isOpen());
});

test('composition: isComposing and keyCode 229 keys are left alone', () => {
  const p = page(NOTE);
  // Not opened while composing.
  assert.ok(!p.key('k', { ctrlKey: true, isComposing: true }, p.doc.body).defaultPrevented);
  assert.ok(!p.isOpen());
  assert.ok(!p.key('/', { isComposing: true }, p.doc.body).defaultPrevented);
  assert.ok(!p.isOpen());
  p.key('k', { ctrlKey: true });
  // The Enter that commits a composition does not jump, from the input...
  for (const mods of [{ isComposing: true }, { keyCode: 229 }]) {
    const e = p.key('Enter', mods);
    assert.ok(p.isOpen(), JSON.stringify(mods));
    assert.ok(!e.defaultPrevented, JSON.stringify(mods));
    assert.ok(p.isOpen());
    p.key('ArrowDown', mods);
    assert.equal(p.rows().filter((r) => r.getAttribute('aria-selected') === 'true').length, 0);
  }
  // ... and an Escape that ends one does not close the box.
  p.key('Escape', { isComposing: true });
  p.key('Escape', { keyCode: 229 });
  p.key('Escape', { isComposing: true }, p.doc.body);
  assert.ok(p.isOpen());
  p.key('Escape');
  assert.ok(!p.isOpen());
});

test('Tab stays in the input', () => {
  const p = page(NOTE);
  p.key('k', { ctrlKey: true });
  for (const mods of [{}, { shiftKey: true }]) {
    const e = p.key('Tab', mods);
    assert.ok(e.defaultPrevented, 'the focus does not leave the dialog');
    assert.ok(p.isOpen());
    focused(p, p.input());
  }
});

test('reopening selects the last query, ready to be replaced', () => {
  const p = page(NOTE);
  p.key('k', { ctrlKey: true });
  p.type('(frac');
  p.key('Escape');
  p.input().selected = false;
  p.key('k', { ctrlKey: true });
  assert.ok(p.input().selected);
  assert.equal(p.input().value, '(frac');
  assert.equal(p.rows().length, 1, 'and its results are shown');
});

test('closing restores the focus without scrolling', () => {
  const p = page((h) => [...NOTE(h), h('a', { href: '#x' }, 'link')]);
  const link = p.doc.querySelector('a');
  link.focus();
  link.focusCalls = [];
  p.key('k', { ctrlKey: true });
  p.key('Escape');
  focused(p, link);
  assert.deepEqual(plain(link.focusCalls), [{ preventScroll: true }]);
});

test('after a jump the focus is on the equation, which gets a tabindex while highlighted', () => {
  const p = page((h) => [...NOTE(h), h('a', { href: '#x' }, 'link')]);
  const link = p.doc.querySelector('a');
  link.focus();
  const target = p.doc.querySelectorAll('[data-mazer-src]')[2];
  p.key('k', { ctrlKey: true });
  p.type('Delta');
  p.key('Enter');
  focused(p, target, 'not back on the link');
  assert.equal(target.getAttribute('tabindex'), '-1');
  assert.deepEqual(plain(target.focusCalls), [{ preventScroll: true }]);
  assert.deepEqual(plain(target.scrolled), [{ block: 'center', behavior: 'smooth' }]);
  // The highlight goes; the tabindex stays while the equation has the focus.
  p.win.runTimers();
  assert.ok(!target.classList.contains('mazer-search-hit'));
  assert.equal(target.getAttribute('tabindex'), '-1');
  link.focus();
  assert.equal(target.getAttribute('tabindex'), null, 'gone with the focus');
  // Focus moved away before the highlight ends: gone with the highlight.
  p.key('k', { ctrlKey: true }, link);
  p.type('Delta');
  p.key('Enter');
  focused(p, target);
  link.focus();
  assert.equal(target.getAttribute('tabindex'), '-1');
  p.win.runTimers();
  assert.equal(target.getAttribute('tabindex'), null);
});

test('a tabindex the equation already had is kept; a second jump clears the first', () => {
  const p = page((h) => [
    h('div', { class: 'mazer-equation', 'data-mazer-src': '(a)', tabindex: '0' }),
    h('div', { class: 'mazer-equation', 'data-mazer-src': '(b)' }),
    h('div', { class: 'mazer-equation', 'data-mazer-src': '(c)' }),
  ]);
  const [a, b, c] = p.doc.querySelectorAll('[data-mazer-src]');
  p.key('k', { ctrlKey: true });
  p.type('(a');
  p.key('Enter');
  focused(p, a);
  p.win.runTimers();
  p.doc.body.focus();
  assert.equal(a.getAttribute('tabindex'), '0');
  p.key('k', { ctrlKey: true }, p.doc.body);
  p.type('(b');
  p.key('Enter');
  assert.equal(b.getAttribute('tabindex'), '-1');
  p.key('k', { ctrlKey: true });
  p.type('(c');
  p.key('Enter');
  focused(p, c);
  assert.equal(b.getAttribute('tabindex'), null);
  assert.equal(c.getAttribute('tabindex'), '-1');
});

test('the page re-rendered while the box was open: Enter finds the new equation', () => {
  const p = page(NOTE);
  p.key('k', { ctrlKey: true });
  p.type('frac');
  // The page renders its notes again: new elements, same sources.
  const old = p.doc.querySelector('math[data-mazer-src]');
  const fresh = p.h('math', { display: 'inline', 'data-mazer-src': '(frac 1 2)' });
  old.parentNode.appendChild(fresh);
  old.remove();
  p.key('Enter');
  assert.ok(!p.isOpen());
  assert.ok(fresh.classList.contains('mazer-search-hit'));
  assert.equal(fresh.scrolled.length, 1);
});

test('the page re-rendered without the match: the box stays open with the new results', () => {
  const p = page(NOTE);
  p.key('k', { ctrlKey: true });
  p.type('frac');
  assert.equal(p.rows().length, 1);
  p.doc.querySelector('math[data-mazer-src]').remove();
  p.key('Enter');
  assert.ok(p.isOpen());
  assert.equal(p.rows().length, 0);
  assert.equal(p.text('.mazer-search-note'), 'No equation matches.');
  assert.equal(p.text('.mazer-search-count'), '0 of 3');
});

test('a heading with math shows the math by its source', () => {
  const p = page((h) => [
    h('h2', {}, 'About ', h('math', { display: 'inline', 'data-mazer-src': '(frac 1 2)' }, h('mn', {}, '1'), h('mn', {}, '2')), ' and ', h('em', {}, 'more')),
    h('div', { class: 'mazer-equation', 'data-mazer-src': '(sqrt x)' }),
  ]);
  p.key('k', { ctrlKey: true });
  assert.deepEqual(p.rows().map((r) => r.querySelector('.mazer-search-ctx').textContent), ['About (frac 1 2) and more', 'About (frac 1 2) and more']);
});

test('a pasted form loses comments and line breaks and is found', () => {
  const p = page((h) => [
    h('div', { class: 'mazer-equation', 'data-mazer-src': USER }),
    h('div', { class: 'mazer-equation', 'data-mazer-src': '(funcapp rho x)' }),
  ]);
  const [user] = p.doc.querySelectorAll('[data-mazer-src]');
  p.key('k', { ctrlKey: true });
  const clip = '(show (= (integral (funcapp rho x) x)\n         (integral (jux m (funcapp delta (- x a))) x)   ; a comment\n         m))';
  const e = p.paste(clip);
  assert.ok(e.defaultPrevented, 'the browser does not paste it as well');
  assert.ok(!p.input().value.includes('\n'));
  assert.ok(!p.input().value.includes('comment'));
  assert.deepEqual(p.srcs(), [USER]);
  p.key('Enter');
  assert.ok(user.classList.contains('mazer-search-hit'));
  // At the caret, replacing the selection.
  p.key('k', { ctrlKey: true });
  p.type('(funcapp  x)');
  p.input().selectionStart = 9;
  p.input().selectionEnd = 10;
  p.paste('rho ; the density\n');
  assert.equal(p.input().value, '(funcapp rho  x)');
  assert.equal(p.rows().length, 2);
  // Without text on the clipboard the browser handles it.
  const none = new Event('paste', { clipboardData: { getData: () => '' } });
  p.input().dispatchEvent(none);
  assert.ok(!none.defaultPrevented);
});

test('the floating button opens the box', () => {
  const p = page(NOTE);
  const [button] = p.button();
  assert.equal(p.button().length, 1);
  assert.equal(button.tagName, 'BUTTON');
  assert.equal(button.getAttribute('type'), 'button');
  assert.equal(button.getAttribute('aria-label'), 'Search equations');
  assert.equal(button.children[0].tagName, 'svg');
  assert.equal(button.children[0].namespaceURI, 'http://www.w3.org/2000/svg');
  assert.equal(p.doc.querySelectorAll('style').length, 1, 'its styles are there before any open');
  button.focus();
  button.dispatchEvent(new Event('click'));
  assert.ok(p.isOpen());
  focused(p, p.input());
  p.key('Escape');
  focused(p, button);
  assert.ok(/@media print\{[^}]*\.mazer-search-fab[^}]*display:none/.test(SCRIPT), 'not printed');
});

test('no button without equations, until they are rendered; none when turned off', () => {
  const p = page(null);
  assert.equal(p.button().length, 0);
  p.doc.dispatchEvent(new Event('pointerdown'));
  assert.equal(p.button().length, 0);
  p.doc.body.append(p.h('math', { display: 'inline', 'data-mazer-src': 'x' }));
  p.doc.dispatchEvent(new Event('pointerdown'));
  assert.equal(p.button().length, 1, 'added at the first interaction after the render');
  p.doc.dispatchEvent(new Event('keydown', { key: 'a' }));
  assert.equal(p.button().length, 1);
  assert.equal(p.doc.listeners.pointerdown.length, 0, 'and stops listening');
  const off = page(NOTE, { globals: { mazerSearchButton: false } });
  assert.equal(off.button().length, 0);
  off.doc.dispatchEvent(new Event('pointerdown'));
  assert.equal(off.button().length, 0);
  off.key('k', { ctrlKey: true });
  assert.ok(off.isOpen(), 'the keyboard still works');
});

// ---- a real page, if given ----

// Arguments come in pairs, `page.html forms.json`: a full MathML page from
// the CLI, and the `show`/`eval` forms of its note exactly as typed (with
// the wrapper, tags, comments and line breaks), in document order, as a JSON
// array of strings. Every form, pasted into the box, must find the equation
// it rendered: the one at the same position on the page.
const args = process.argv.slice(2);
for (let a = 0; a < args.length; a += 2) {
  const pagePath = args[a];
  const formsPath = args[a + 1];
  test(`every equation of ${pagePath} is found by its form as typed`, () => {
    const html = readFileSync(pagePath, 'utf8').replace(/<script\b[\s\S]*?<\/script>/gi, '');
    const decode = (s) => s.replace(/&(amp|lt|gt|quot|#39);/g, (_, e) => ({ amp: '&', lt: '<', gt: '>', quot: '"', '#39': "'" })[e]);
    const values = [...html.matchAll(/ data-mazer-src="([^"]*)"/g)].map((m) => decode(m[1]));
    assert.ok(values.length > 0, 'the page has equations');
    assert.equal(values.length, (html.match(/<math[\s>]/g) || []).length, 'one source per <math> element');
    const { matches, normalize, search, pasteText } = pure();
    for (const v of values) {
      assert.ok(v.trim(), 'no empty source');
      assert.ok(matches(v, v), v);
      // Outside strings, the stored source is already in normal form.
      if (!v.includes('"')) assert.equal(normalize(v), v);
    }
    assert.ok(formsPath, 'a forms file goes with every page');
    const forms = JSON.parse(readFileSync(formsPath, 'utf8'));
    assert.equal(forms.length, values.length, 'one rendered equation per form');
    const items = values.map((src, i) => ({ src, i }));
    for (const [i, form] of forms.entries()) {
      const hits = search(pasteText(form), items);
      assert.ok(hits.some((h) => h.item.i === i), `form ${i} ${JSON.stringify(form)} does not find ${JSON.stringify(values[i])}`);
    }
    console.log(`# ${pagePath}: ${values.length} equations, each found by its form as typed`);
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
