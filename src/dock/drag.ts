/**
 * Réorganisation par glisser-déposer directement dans la barre (pointer events + animation FLIP).
 * Glisser une app épinglée hors du dock (> 90 px vers l'intérieur de l'écran) puis relâcher = la retirer.
 */
import type { DockItem } from '../shared/types';
import { freeze, measure, wake } from './engine';
import { layout, vertical } from './layout';
import { getSlot } from './render';
import { store, updateConfig } from './store';

const itemsEl = document.getElementById('items') as HTMLElement;

interface DragState {
  el: HTMLElement;
  pointerId: number;
  startX: number;
  startY: number;
  startOffset: number;
  active: boolean;
  remove: boolean;
}

/** Sens "vers l'intérieur" de l'écran sur l'axe secondaire (retrait d'une app). */
const INWARD: Record<string, number> = { bottom: -1, free: -1, top: 1, left: 1, right: -1 };

let drag: DragState | null = null;
let suppressClickUntil = 0;

export function clickSuppressed(): boolean {
  return performance.now() < suppressClickUntil;
}

export function onPointerDown(e: PointerEvent, el: HTMLElement): void {
  const slot = getSlot(el);
  if (e.button !== 0 || !slot?.draggable) return;
  drag = {
    el,
    pointerId: e.pointerId,
    startX: e.clientX,
    startY: e.clientY,
    startOffset: vertical() ? el.offsetTop : el.offsetLeft,
    active: false,
    remove: false,
  };
}

window.addEventListener('pointermove', (e) => {
  if (!drag || e.pointerId !== drag.pointerId) return;
  const dx0 = e.clientX - drag.startX;
  const dy0 = e.clientY - drag.startY;
  if (!drag.active) {
    if (Math.hypot(dx0, dy0) < 7) return;
    drag.active = true;
    store.dragging = true;
    drag.el.setPointerCapture(e.pointerId);
    drag.el.classList.remove('pressed');
    drag.el.classList.add('dragging');
    freeze(true);
  }

  const el = drag.el;
  const vert = vertical();
  const p = vert ? e.clientY : e.clientX;
  const others = Array.from(itemsEl.children).filter((c) => c !== el) as HTMLElement[];
  let idx = others.findIndex((o) => {
    const r = o.getBoundingClientRect();
    return vert ? p < r.top + r.height / 2 : p < r.left + r.width / 2;
  });
  if (idx < 0) idx = others.length;
  const currentIdx = Array.from(itemsEl.children).indexOf(el);
  if (idx !== currentIdx) flip(others, () => itemsEl.insertBefore(el, others[idx] ?? null));

  // Axe principal : suit la souris. Axe secondaire : seulement vers l'intérieur de l'écran.
  const main = (vert ? dy0 : dx0) - ((vert ? el.offsetTop : el.offsetLeft) - drag.startOffset);
  const inward = INWARD[layout.edge];
  const cross = Math.max(0, (vert ? dx0 : dy0) * inward) * inward;
  el.style.transform = vert ? `translate(${cross}px, ${main}px)` : `translate(${main}px, ${cross}px)`;
  const slot = getSlot(el);
  drag.remove = !!slot?.pinned && Math.abs(cross) > 90;
  el.classList.toggle('remove-intent', drag.remove);
  wake(200);
});

window.addEventListener('pointerup', (e) => finish(e));
window.addEventListener('pointercancel', (e) => finish(e, true));

function finish(e: PointerEvent, cancelled = false): void {
  if (!drag || e.pointerId !== drag.pointerId) return;
  const d = drag;
  drag = null;
  if (!d.active) return;

  suppressClickUntil = performance.now() + 250;
  store.dragging = false;
  d.el.classList.remove('dragging', 'remove-intent');
  d.el.style.transform = '';
  freeze(false);

  const slot = getSlot(d.el);
  if (!cancelled && d.remove && slot) {
    d.el.classList.add('poof');
    setTimeout(() => updateConfig((c) => (c.items = c.items.filter((i) => i.id !== slot.item.id))), 260);
    return;
  }
  commitOrder();
  measure();
}

/** Anime les voisins de leur ancienne position vers la nouvelle. */
function flip(nodes: HTMLElement[], mutate: () => void): void {
  const before = new Map(nodes.map((n) => [n, n.getBoundingClientRect().left]));
  mutate();
  for (const n of nodes) {
    const delta = (before.get(n) ?? 0) - n.getBoundingClientRect().left;
    if (Math.abs(delta) < 0.5) continue;
    n.style.transition = 'none';
    n.style.transform = `translateX(${delta}px)`;
    void n.offsetWidth;
    n.style.transition = 'transform 0.22s cubic-bezier(0.2, 0.9, 0.25, 1.05)';
    n.style.transform = '';
    setTimeout(() => (n.style.transition = ''), 240);
  }
}

/** Ordre DOM → ordre de config (les éléments désactivés gardent leur voisin). */
function commitOrder(): void {
  const visibleIds: string[] = [];
  for (const node of Array.from(itemsEl.children)) {
    const s = getSlot(node as HTMLElement);
    if (s && !visibleIds.includes(s.item.id)) visibleIds.push(s.item.id);
  }
  const items = store.config.items;
  const byId = new Map(items.map((i) => [i.id, i]));
  const result: DockItem[] = visibleIds.map((id) => byId.get(id)).filter((i): i is DockItem => !!i);
  items.forEach((it, idx) => {
    if (visibleIds.includes(it.id)) return;
    // Insère après son voisin précédent d'origine.
    let pos = 0;
    for (let j = idx - 1; j >= 0; j--) {
      const at = result.indexOf(items[j]);
      if (at >= 0) {
        pos = at + 1;
        break;
      }
    }
    result.splice(pos, 0, it);
  });
  const changed = result.some((it, i) => it.id !== items[i]?.id);
  if (changed) updateConfig((c) => (c.items = result.map((r) => c.items.find((x) => x.id === r.id) as DockItem)));
}
