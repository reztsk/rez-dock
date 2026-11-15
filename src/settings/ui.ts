/** Mini-bibliothèque de contrôles pour le panneau de réglages (DOM natif, zéro dépendance). */
import { PALETTE } from '../shared/defaults';
import { GLYPHS, type GlyphName } from '../shared/icons';
import { isValidHex, normalizeHex } from '../shared/theme';

export function el<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  className = '',
  text?: string,
): HTMLElementTagNameMap[K] {
  const e = document.createElement(tag);
  if (className) e.className = className;
  if (text !== undefined) e.textContent = text;
  return e;
}

export function page(title: string, lead: string): HTMLElement {
  const p = el('div', 'page');
  p.append(el('h1', '', title), el('p', 'lead', lead));
  return p;
}

export function h2(text: string): HTMLElement {
  return el('h2', '', text);
}

export function card(...rows: HTMLElement[]): HTMLElement {
  const c = el('div', 'card');
  c.append(...rows);
  return c;
}

export function row(title: string, desc: string | null, control?: HTMLElement): HTMLElement {
  const r = el('div', 'row');
  const t = el('div', 'text');
  t.appendChild(el('div', 'title', title));
  if (desc) t.appendChild(el('div', 'desc', desc));
  r.appendChild(t);
  if (control) r.appendChild(control);
  return r;
}

export function stackRow(title: string, desc: string | null, ...children: HTMLElement[]): HTMLElement {
  const r = el('div', 'row stack');
  const t = el('div', 'text');
  t.appendChild(el('div', 'title', title));
  if (desc) t.appendChild(el('div', 'desc', desc));
  r.append(t, ...children);
  return r;
}

export function glyph(name: GlyphName): string {
  return GLYPHS[name];
}

export function button(label: string, opts: { icon?: GlyphName; kind?: 'primary' | 'danger'; onClick: () => void }): HTMLButtonElement {
  const b = el('button', `btn ${opts.kind ?? ''}`);
  if (opts.icon) b.insertAdjacentHTML('beforeend', GLYPHS[opts.icon]);
  b.appendChild(document.createTextNode(label));
  b.onclick = opts.onClick;
  return b;
}

export function iconButton(icon: GlyphName, title: string, onClick: () => void, danger = false): HTMLButtonElement {
  const b = el('button', `icon-btn${danger ? ' danger' : ''}`);
  b.title = title;
  b.innerHTML = GLYPHS[icon];
  b.onclick = (e) => {
    e.stopPropagation();
    onClick();
  };
  return b;
}

export function toggle(value: boolean, onChange: (v: boolean) => void, small = false): HTMLButtonElement {
  const t = el('button', `toggle${value ? ' on' : ''}${small ? ' small' : ''}`);
  t.setAttribute('role', 'switch');
  t.setAttribute('aria-checked', String(value));
  t.onclick = (e) => {
    e.stopPropagation();
    const v = !t.classList.contains('on');
    t.classList.toggle('on', v);
    t.setAttribute('aria-checked', String(v));
    onChange(v);
  };
  return t;
}

export function slider(opts: {
  value: number;
  min: number;
  max: number;
  step: number;
  format: (v: number) => string;
  onInput: (v: number) => void;
}): HTMLElement {
  const wrap = el('div', 'slider');
  const input = el('input');
  input.type = 'range';
  input.min = String(opts.min);
  input.max = String(opts.max);
  input.step = String(opts.step);
  input.value = String(opts.value);
  const out = el('output', '', opts.format(opts.value));
  const paint = () => {
    const p = ((Number(input.value) - opts.min) / (opts.max - opts.min)) * 100;
    input.style.setProperty('--p', `${p}%`);
  };
  paint();
  input.oninput = () => {
    const v = Number(input.value);
    out.textContent = opts.format(v);
    paint();
    opts.onInput(v);
  };
  wrap.append(input, out);
  return wrap;
}

/** Palette étendue + sélecteur libre + saisie hexadécimale. */
export function colorPicker(value: string, onChange: (hex: string) => void): HTMLElement {
  const wrap = el('div');
  wrap.style.display = 'flex';
  wrap.style.flexDirection = 'column';
  wrap.style.gap = '12px';

  const grid = el('div', 'swatches');
  const tools = el('div', 'color-tools');
  const pick = el('label', 'color-pick');
  const native = el('input');
  native.type = 'color';
  native.value = normalizeHex(value);
  pick.appendChild(native);
  const hex = el('input', 'text-input hex');
  hex.value = normalizeHex(value);
  hex.maxLength = 7;
  hex.spellcheck = false;
  tools.append(pick, hex, el('span', 'desc', 'Palette, sélecteur libre ou code hexadécimal'));

  const set = (v: string, from?: 'native' | 'hex') => {
    const h = normalizeHex(v);
    if (from !== 'native') native.value = h;
    if (from !== 'hex') hex.value = h;
    grid.querySelectorAll('.swatch').forEach((s) => s.classList.toggle('on', (s as HTMLElement).dataset.c === h));
    onChange(h);
  };

  for (const c of PALETTE) {
    const s = el('button', `swatch${normalizeHex(c) === normalizeHex(value) ? ' on' : ''}`);
    s.dataset.c = normalizeHex(c);
    s.style.background = c;
    s.title = c;
    s.onclick = () => set(c);
    grid.appendChild(s);
  }
  native.oninput = () => set(native.value, 'native');
  hex.oninput = () => {
    const v = hex.value.startsWith('#') ? hex.value : `#${hex.value}`;
    if (isValidHex(v) && (v.length === 7 || v.length === 4)) set(v, 'hex');
  };

  wrap.append(grid, tools);
  return wrap;
}
