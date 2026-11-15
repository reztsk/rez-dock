import { icon } from '../shared/api';
import { brandIcon } from '../shared/brands';
import { MODULES } from '../shared/defaults';
import { GLYPHS } from '../shared/icons';
import { measure, setLabel } from './engine';
import { buildSlots, type Slot } from './slots';
import { store } from './store';

const itemsEl = document.getElementById('items') as HTMLElement;
const els = new Map<string, HTMLElement>();
const slotOf = new WeakMap<HTMLElement, Slot>();
let firstRender = true;

export function getSlot(el: HTMLElement): Slot | undefined {
  return slotOf.get(el);
}

export function slotElement(node: EventTarget | null): HTMLElement | null {
  return node instanceof Element ? (node.closest('.slot') as HTMLElement | null) : null;
}

export function render(): void {
  const slots = buildSlots(store.config, store.windows);
  const keep = new Set(slots.map((s) => s.key));

  for (const [key, el] of els) {
    if (!keep.has(key)) {
      el.remove();
      els.delete(key);
    }
  }

  slots.forEach((slot, i) => {
    let el = els.get(slot.key);
    if (!el) {
      el = create(slot);
      els.set(slot.key, el);
      if (!firstRender) {
        el.classList.add('entering');
        setTimeout(() => el?.classList.remove('entering'), 450);
      }
    }
    update(el, slot);
    const at = itemsEl.children[i];
    if (at !== el) itemsEl.insertBefore(el, at ?? null);
  });

  firstRender = false;
  measure();
}

function create(slot: Slot): HTMLElement {
  const el = document.createElement('div');
  el.className = `slot ${slot.kind}`;
  el.dataset.key = slot.key;

  if (slot.kind === 'separator') {
    el.appendChild(document.createElement('i'));
    return el;
  }
  if (slot.kind === 'clock') {
    el.innerHTML = '<b class="time"></b><span class="date"></span>';
    tickClock(el);
    return el;
  }

  const tile = document.createElement('div');
  tile.className = 'tile';
  const inner = document.createElement('div');
  inner.className = 'tile-inner';
  inner.addEventListener('animationend', () => inner.classList.remove('bounce', 'pulse'));
  tile.appendChild(inner);
  el.appendChild(tile);

  const dots = document.createElement('div');
  dots.className = 'dots';
  el.appendChild(dots);

  if (slot.kind === 'module') {
    const glyph = MODULES[slot.item.type].glyph;
    if (glyph) {
      const g = document.createElement('div');
      g.className = slot.item.type === 'start' ? 'glyph start' : 'glyph';
      g.innerHTML = GLYPHS[glyph];
      inner.appendChild(g);
    }
  } else {
    setLetter(inner, slot.label);
    loadIcon(inner, slot.iconPath);
  }
  return el;
}

function setLetter(inner: HTMLElement, label: string): void {
  const l = document.createElement('div');
  l.className = 'letter';
  l.textContent = (label.trim()[0] ?? '?').toUpperCase();
  inner.replaceChildren(l);
}

function loadIcon(inner: HTMLElement, path?: string): void {
  inner.dataset.icon = path ?? '';
  if (!path) return;
  // Chrome / Explorateur : logo vectoriel intégré (net à toute taille, jamais de lettre).
  const svg = brandIcon(path);
  if (svg) {
    const b = document.createElement('div');
    b.className = 'ico brand';
    b.innerHTML = svg;
    inner.replaceChildren(b);
    return;
  }
  void icon(path, 128).then((src) => {
    if (!src || inner.dataset.icon !== path) return;
    const img = document.createElement('img');
    img.className = 'ico';
    img.src = src;
    inner.replaceChildren(img);
  });
}

function update(el: HTMLElement, slot: Slot): void {
  slotOf.set(el, slot);
  el.dataset.scalable = slot.scalable ? '1' : '0';
  setLabel(el, slot.label);
  if (slot.kind !== 'app') return;

  const inner = el.querySelector('.tile-inner') as HTMLElement;
  if (inner.dataset.icon !== (slot.iconPath ?? '')) {
    setLetter(inner, slot.label);
    loadIcon(inner, slot.iconPath);
  }

  const dots = el.querySelector('.dots') as HTMLElement;
  const n = Math.min(3, slot.windows.length);
  const active = slot.windows.some((w) => w.active);
  const sig = `${n}|${active}`;
  if (dots.dataset.sig !== sig) {
    dots.dataset.sig = sig;
    dots.replaceChildren();
    for (let i = 0; i < n; i++) {
      const d = document.createElement('i');
      if (active && i === 0) d.className = 'active';
      dots.appendChild(d);
    }
  }
}

// ------------------------------------------------------------------ Horloge

const clocks = new Set<HTMLElement>();
let clockTimer = 0;

function tickClock(el: HTMLElement): void {
  clocks.add(el);
  updateClocks();
}

export function updateClocks(): void {
  window.clearTimeout(clockTimer);
  const b = store.config?.behavior;
  const now = new Date();
  const time = now.toLocaleTimeString('fr-FR', {
    hour: '2-digit',
    minute: '2-digit',
    second: b?.clockSeconds ? '2-digit' : undefined,
    hour12: b ? !b.clock24h : false,
  });
  const date = now.toLocaleDateString('fr-FR', { weekday: 'short', day: 'numeric', month: 'short' });
  for (const el of clocks) {
    // Un slot pas encore inséré est rempli quand même ; un slot retiré du dock est oublié.
    if (!el.isConnected && el.dataset.mounted) {
      clocks.delete(el);
      continue;
    }
    if (el.isConnected) el.dataset.mounted = '1';
    (el.querySelector('.time') as HTMLElement).textContent = time;
    (el.querySelector('.date') as HTMLElement).textContent = date;
  }
  const delay = b?.clockSeconds ? 1000 - now.getMilliseconds() : 60000 - (now.getSeconds() * 1000 + now.getMilliseconds());
  clockTimer = window.setTimeout(updateClocks, delay + 15);
}

export function animate(el: HTMLElement, kind: 'bounce' | 'pulse'): void {
  const inner = el.querySelector('.tile-inner');
  if (!inner) return;
  inner.classList.remove('bounce', 'pulse');
  void (inner as HTMLElement).offsetWidth;
  inner.classList.add(kind);
}
