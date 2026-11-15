/**
 * Boucle d'animation unique (requestAnimationFrame) :
 *  - magnification "vague" façon macOS (cosinus, lissage exponentiel indépendant du framerate),
 *    horizontale ou verticale selon le bord
 *  - tooltip de l'icône survolée
 *  - synchronisation des surfaces natives (verre DWM + zones cliquables)
 * La boucle s'arrête d'elle-même au repos → 0 % CPU quand on ne touche pas au dock.
 */
import { api } from '../shared/api';
import { eff } from '../shared/theme';
import type { Surface } from '../shared/types';
import { revealZones } from './autohide';
import { anchor, layout, vertical } from './layout';
import { store } from './store';

const dockEl = document.getElementById('dock') as HTMLElement;
const barEl = dockEl.querySelector('.dock-bg') as HTMLElement;
const itemsEl = document.getElementById('items') as HTMLElement;
const tooltipEl = document.getElementById('tooltip') as HTMLElement;
const popLayer = document.getElementById('popover-layer') as HTMLElement;

interface Rest {
  el: HTMLElement;
  center: number;
  scalable: boolean;
}

let rest: Rest[] = [];
let restLength = 0;
let mouse = -1;
let hovering = false;
let frozen = false;
let raf = 0;
let awakeUntil = 0;
let lastTime = 0;
let lastSurfaces = '';
const scales = new WeakMap<HTMLElement, number>();
const labelOf = new WeakMap<HTMLElement, string>();

export function setLabel(el: HTMLElement, label: string): void {
  labelOf.set(el, label);
}

/** Positions au repos le long de l'axe (indépendantes de la magnification → pas d'oscillation). */
export function measure(): void {
  const e = eff(store.config.appearance);
  const unit = e.icon + e.gap;
  const vert = vertical();
  let acc = 0;
  rest = [];
  for (const node of Array.from(itemsEl.children)) {
    const el = node as HTMLElement;
    const scalable = el.dataset.scalable === '1';
    const len = scalable ? unit : vert ? el.offsetHeight : el.offsetWidth;
    rest.push({ el, center: acc + len / 2, scalable });
    acc += len;
  }
  restLength = acc;
  wake(300);
}

export function wake(ms = 400): void {
  awakeUntil = Math.max(awakeUntil, performance.now() + ms);
  if (!raf) raf = requestAnimationFrame(frame);
}

export function freeze(on: boolean): void {
  frozen = on;
  wake(500);
}

export function isHovering(): boolean {
  return hovering;
}

/** `pos` = coordonnée de la souris le long de l'axe principal. */
export function setHover(on: boolean, pos?: number): void {
  if (pos !== undefined) mouse = pos;
  if (on !== hovering) {
    hovering = on;
    if (!on) tooltipEl.classList.remove('show');
  }
  wake(on ? 250 : 600);
}

function frame(t: number): void {
  raf = 0;
  const dt = lastTime ? Math.min(64, t - lastTime) : 16.7;
  lastTime = t;
  const moving = stepMagnify(dt);
  updateTooltip();
  syncSurfaces();
  if (moving || t < awakeUntil || hovering) {
    raf = requestAnimationFrame(frame);
  } else {
    lastTime = 0;
  }
}

function stepMagnify(dt: number): boolean {
  const a = store.config.appearance;
  const max = a.magnification;
  const range = eff(a).icon * a.magnifyRange;
  const m = mouse - (anchor() - restLength / 2);
  const k = 1 - Math.pow(1 - 0.26, dt / 16.7);
  let moving = false;

  for (const r of rest) {
    let target = 1;
    if (hovering && !frozen && r.scalable && max > 1.001) {
      const d = Math.abs(m - r.center);
      if (d < range) target = 1 + (max - 1) * ((Math.cos((Math.PI * d) / range) + 1) / 2);
    }
    const cur = scales.get(r.el) ?? 1;
    let next = cur + (target - cur) * k;
    if (Math.abs(next - target) < 0.0015) next = target;
    if (next !== cur) {
      r.el.style.setProperty('--s', next.toFixed(4));
      scales.set(r.el, next);
    }
    if (next !== target) moving = true;
  }
  return moving;
}

function updateTooltip(): void {
  if (!hovering || store.dragging || !store.config.appearance.showLabels) {
    tooltipEl.classList.remove('show');
    return;
  }
  const vert = vertical();
  let best: HTMLElement | null = null;
  for (const r of rest) {
    const rect = r.el.getBoundingClientRect();
    const lo = vert ? rect.top : rect.left;
    const hi = vert ? rect.bottom : rect.right;
    if (mouse >= lo && mouse < hi) {
      best = r.el;
      break;
    }
  }
  const label = best ? labelOf.get(best) : '';
  if (!best || !label) {
    tooltipEl.classList.remove('show');
    return;
  }
  const t = ((best.querySelector('.tile') as HTMLElement | null) ?? best).getBoundingClientRect();
  if (tooltipEl.textContent !== label) tooltipEl.textContent = label;
  const s = tooltipEl.style;
  s.left = s.right = s.top = s.bottom = '';
  switch (layout.edge) {
    case 'top':
      s.left = `${t.left + t.width / 2}px`;
      s.top = `${t.bottom + 10}px`;
      break;
    case 'left':
      s.left = `${t.right + 10}px`;
      s.top = `${t.top + t.height / 2}px`;
      break;
    case 'right':
      s.right = `${window.innerWidth - t.left + 10}px`;
      s.top = `${t.top + t.height / 2}px`;
      break;
    default:
      s.left = `${t.left + t.width / 2}px`;
      s.bottom = `${window.innerHeight - t.top + 10}px`;
  }
  tooltipEl.classList.add('show');
}

/** Zone de survol : barre + icônes agrandies, prolongée jusqu'au bord de l'écran (comme macOS). */
export function hoverZone(): DOMRect {
  const bar = barEl.getBoundingClientRect();
  let { left, top, right, bottom } = bar;
  if (hovering) {
    for (const r of rest) {
      const tile = r.el.querySelector('.tile');
      if (!tile) continue;
      const t = tile.getBoundingClientRect();
      left = Math.min(left, t.left - 6);
      top = Math.min(top, t.top - 6);
      right = Math.max(right, t.right + 6);
      bottom = Math.max(bottom, t.bottom + 6);
    }
  }
  const W = window.innerWidth;
  const H = window.innerHeight;
  switch (layout.edge) {
    case 'bottom':
      return new DOMRect(bar.left - 2, top, bar.width + 4, H - top);
    case 'top':
      return new DOMRect(bar.left - 2, 0, bar.width + 4, bottom);
    case 'left':
      return new DOMRect(0, bar.top - 2, right, bar.height + 4);
    case 'right':
      return new DOMRect(left, bar.top - 2, W - left, bar.height + 4);
    default:
      return new DOMRect(left - 2, top, right - left + 4, bottom - top + 2);
  }
}

const round = (s: Surface): Surface => ({
  x: Math.round(s.x),
  y: Math.round(s.y),
  w: Math.round(s.w),
  h: Math.round(s.h),
  r: Math.round(s.r),
});

function onScreen(r: DOMRect): boolean {
  return r.right > 1 && r.bottom > 1 && r.left < window.innerWidth - 1 && r.top < window.innerHeight - 1;
}

export function syncSurfaces(force = false): void {
  const glass: Surface[] = [];
  const hit: Surface[] = [];
  if (store.dragging) {
    // Pendant un glisser, toute la fenêtre capte la souris.
    hit.push({ x: 0, y: 0, w: window.innerWidth, h: window.innerHeight, r: 0 });
  }
  const bar = barEl.getBoundingClientRect();
  if (onScreen(bar) && Number(getComputedStyle(dockEl).opacity) > 0.05) {
    const r = Math.min(eff(store.config.appearance).radius, bar.height / 2, bar.width / 2);
    glass.push({ x: bar.left, y: bar.top, w: bar.width, h: bar.height, r });
    if (!store.hidden && !store.autoHidden) {
      const z = hoverZone();
      hit.push({ x: z.x, y: z.y, w: z.width, h: z.height, r: 0 });
    }
  }
  for (const node of Array.from(popLayer.children)) {
    const el = node as HTMLElement;
    if (el.classList.contains('closing')) continue;
    const rect = el.getBoundingClientRect();
    glass.push({ x: rect.left, y: rect.top, w: rect.width, h: rect.height, r: 18 });
    hit.push({ x: rect.left - 4, y: rect.top - 4, w: rect.width + 8, h: rect.height + 8, r: 0 });
  }
  const g = glass.map(round);
  const h = hit.map(round);
  const v = revealZones().map(round);
  const key = JSON.stringify([g, h, v]);
  if (!force && key === lastSurfaces) return;
  lastSurfaces = key;
  void api.setSurfaces(g, h, v);
}
