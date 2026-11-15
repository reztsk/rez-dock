import { GLYPHS, type GlyphName } from '../shared/icons';
import { wake } from './engine';
import { layout, vertical } from './layout';

const layer = document.getElementById('popover-layer') as HTMLElement;
const barEl = document.querySelector('#dock .dock-bg') as HTMLElement;

interface Current {
  el: HTMLElement;
  persistent: boolean;
  owner: string;
  anchor: HTMLElement | number;
  onClose?: () => void;
}

let current: Current | null = null;

export interface PopoverOptions {
  className?: string;
  /** Ne se ferme pas quand la fenêtre perd le focus (ex : notification de mise à jour). */
  persistent?: boolean;
  /** Identifiant du déclencheur : re-cliquer le même bouton referme le popover. */
  owner?: string;
  onClose?: () => void;
}

export function isOpen(owner?: string): boolean {
  return !!current && (owner === undefined || current.owner === owner);
}

/**
 * Ouvre un popover à côté du dock (au-dessus, en dessous ou sur le côté selon le bord),
 * centré sur l'ancre : élément, ou coordonnée le long de l'axe du dock.
 */
export function openPopover(content: HTMLElement, anchor: HTMLElement | number, opts: PopoverOptions = {}): HTMLElement {
  closePopover(true);
  const el = document.createElement('div');
  el.className = `popover ${opts.className ?? ''}`;
  el.appendChild(content);
  el.addEventListener('contextmenu', (e) => e.preventDefault());
  layer.appendChild(el);
  current = { el, persistent: !!opts.persistent, owner: opts.owner ?? '', anchor, onClose: opts.onClose };
  window.dispatchEvent(new Event('rez-popover'));
  place(el, anchor);
  requestAnimationFrame(() => {
    el.classList.add('show');
    wake(450);
  });
  return el;
}

/** Recalcule la position (après changement de contenu). */
export function repositionPopover(anchor?: HTMLElement | number): void {
  if (!current) return;
  place(current.el, anchor ?? current.anchor);
  wake(300);
}

/** Côté d'ouverture : vers l'intérieur de l'écran. En mode libre, du côté le plus spacieux. */
function side(bar: DOMRect): 'above' | 'below' | 'right' | 'left' {
  switch (layout.edge) {
    case 'top':
      return 'below';
    case 'left':
      return 'right';
    case 'right':
      return 'left';
    case 'free':
      return bar.top > window.innerHeight - bar.bottom ? 'above' : 'below';
    default:
      return 'above';
  }
}

function place(el: HTMLElement, anchor: HTMLElement | number): void {
  const bar = barEl.getBoundingClientRect();
  const W = window.innerWidth;
  const H = window.innerHeight;
  const s = el.style;
  s.left = s.right = s.top = s.bottom = '';
  const where = side(bar);
  const pos = typeof anchor === 'number' ? anchor : centerOf(anchor);

  if (where === 'above' || where === 'below') {
    s.maxHeight = `${Math.max(160, (where === 'above' ? bar.top : H - bar.bottom) - 24)}px`;
    const w = el.offsetWidth;
    const left = Math.max(12, Math.min(W - w - 12, pos - w / 2));
    s.left = `${left}px`;
    if (where === 'above') s.bottom = `${H - bar.top + 12}px`;
    else s.top = `${bar.bottom + 12}px`;
    s.transformOrigin = `${Math.max(10, Math.min(w - 10, pos - left))}px ${where === 'above' ? '100%' : '0%'}`;
  } else {
    s.maxHeight = `${H - 24}px`;
    const h = el.offsetHeight;
    const top = Math.max(12, Math.min(H - h - 12, pos - h / 2));
    s.top = `${top}px`;
    if (where === 'right') s.left = `${bar.right + 12}px`;
    else s.right = `${W - bar.left + 12}px`;
    s.transformOrigin = `${where === 'right' ? '0%' : '100%'} ${Math.max(10, Math.min(h - 10, pos - top))}px`;
  }
  el.dataset.side = where;
}

function centerOf(el: HTMLElement): number {
  const tile = (el.querySelector('.tile') as HTMLElement | null) ?? el;
  const r = tile.getBoundingClientRect();
  return vertical() ? r.top + r.height / 2 : r.left + r.width / 2;
}

export function closePopover(immediate = false, onlyTransient = false): void {
  if (!current) return;
  if (onlyTransient && current.persistent) return;
  const { el, onClose } = current;
  current = null;
  onClose?.();
  el.classList.add('closing');
  el.classList.remove('show');
  if (immediate) {
    el.remove();
  } else {
    setTimeout(() => el.remove(), 200);
  }
  wake(300);
}

// ------------------------------------------------------------------ Menus

export type MenuEntry =
  | { header: string }
  | 'sep'
  | {
      label: string;
      icon?: GlyphName;
      img?: string;
      hint?: string;
      danger?: boolean;
      active?: boolean;
      action: () => void | Promise<void>;
      keepOpen?: boolean;
    };

export function buildMenu(entries: MenuEntry[]): HTMLElement {
  const root = document.createElement('div');
  root.className = 'popover-scroll';
  for (const e of entries) {
    if (e === 'sep') {
      root.appendChild(div('menu-sep'));
    } else if ('header' in e) {
      const h = div('menu-header');
      h.textContent = e.header;
      root.appendChild(h);
    } else {
      const row = div(`menu-item${e.danger ? ' danger' : ''}${e.active ? ' active' : ''}`);
      if (e.img) {
        const im = document.createElement('img');
        im.src = e.img;
        row.appendChild(im);
      } else if (e.icon) {
        row.insertAdjacentHTML('beforeend', GLYPHS[e.icon]);
      }
      const label = document.createElement('span');
      label.className = 'label';
      label.textContent = e.label;
      row.appendChild(label);
      if (e.hint) {
        const hint = document.createElement('span');
        hint.className = 'hint';
        hint.textContent = e.hint;
        row.appendChild(hint);
      }
      row.addEventListener('click', (ev) => {
        ev.stopPropagation();
        if (!e.keepOpen) closePopover();
        void e.action();
      });
      root.appendChild(row);
    }
  }
  return root;
}

export function openMenu(entries: MenuEntry[], anchor: HTMLElement | number, owner = 'menu'): void {
  openPopover(buildMenu(entries), anchor, { className: 'menu', owner });
}

export function div(className: string): HTMLDivElement {
  const d = document.createElement('div');
  d.className = className;
  return d;
}

// Fermeture : perte de focus (clic ailleurs) ou Échap.
window.addEventListener('blur', () => closePopover(false, true));
window.addEventListener('keydown', (e) => {
  if (e.key === 'Escape') closePopover();
});
