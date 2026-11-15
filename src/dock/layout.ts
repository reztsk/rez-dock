/**
 * Positionnement du dock : bord (bas/haut/gauche/droite) ou placement libre.
 * La fenêtre couvre tout l'écran ; tout se joue en CSS via des classes + variables.
 */
import type { DockEdge } from '../shared/types';
import { store } from './store';

export interface Insets {
  top: number;
  right: number;
  bottom: number;
  left: number;
}

export const layout = {
  edge: 'bottom' as DockEdge,
  insets: { top: 0, right: 0, bottom: 0, left: 0 } as Insets,
};

const dockEl = document.getElementById('dock') as HTMLElement;
const root = document.documentElement;

export function vertical(): boolean {
  return layout.edge === 'left' || layout.edge === 'right';
}

/** Coordonnée le long de l'axe principal (x si horizontal, y si vertical). */
export function axis(x: number, y: number): number {
  return vertical() ? y : x;
}

/** Centre fixe du dock sur son axe principal (la barre grandit symétriquement autour). */
export function anchor(): number {
  const i = layout.insets;
  const b = store.config.behavior;
  switch (layout.edge) {
    case 'left':
    case 'right':
      return (i.top + window.innerHeight - i.bottom) / 2;
    case 'free':
      return b.freeX * window.innerWidth;
    default:
      return (i.left + window.innerWidth - i.right) / 2;
  }
}

export function applyLayout(): void {
  const b = store.config.behavior;
  layout.edge = b.position;
  const i = layout.insets;
  for (const e of ['bottom', 'top', 'left', 'right', 'free']) dockEl.classList.toggle(`edge-${e}`, e === layout.edge);
  dockEl.classList.toggle('vertical', vertical());
  root.dataset.edge = layout.edge;

  const ax = layout.edge === 'free' ? b.freeX * window.innerWidth : (i.left + window.innerWidth - i.right) / 2;
  const ay = layout.edge === 'free' ? b.freeY * window.innerHeight : (i.top + window.innerHeight - i.bottom) / 2;
  setAnchor(ax, ay);
  root.style.setProperty('--inset-t', `${i.top}px`);
  root.style.setProperty('--inset-r', `${i.right}px`);
  root.style.setProperty('--inset-b', `${i.bottom}px`);
  root.style.setProperty('--inset-l', `${i.left}px`);
}

export function setAnchor(x: number, y: number): void {
  root.style.setProperty('--ax', `${x}px`);
  root.style.setProperty('--ay', `${y}px`);
}

/** Bord le plus proche si le point est à moins de `snap` px d'un bord de l'écran. */
export function nearestEdge(x: number, y: number, snap = 64): DockEdge | null {
  const d: [DockEdge, number][] = [
    ['bottom', window.innerHeight - y],
    ['top', y],
    ['left', x],
    ['right', window.innerWidth - x],
  ];
  d.sort((a, b) => a[1] - b[1]);
  return d[0][1] <= snap ? d[0][0] : null;
}
