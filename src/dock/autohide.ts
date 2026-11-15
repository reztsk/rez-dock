/**
 * Masquage automatique façon macOS.
 *
 *  - La souris quitte le dock → après `autoHideDelay` ms, le dock glisse hors de l'écran ;
 *    un petit trait lumineux reste au bord pour signaler sa présence.
 *  - La souris touche le bord (zone de réveil gérée côté Rust, qui n'intercepte AUCUN clic)
 *    → le dock remonte instantanément.
 *  - Touche Windows enfoncée / menu Démarrer ouvert → le dock remonte et reste visible
 *    tant que le menu est ouvert.
 *  - Jamais de masquage pendant un menu contextuel, un popover, un glisser ou un dépôt de fichier.
 */
import type { Surface } from '../shared/types';
import { wake } from './engine';
import { anchor, layout, vertical } from './layout';
import { isOpen } from './popover';
import { store } from './store';

const dockEl = document.getElementById('dock') as HTMLElement;
const peekEl = document.getElementById('peek') as HTMLElement;

const sig = { inside: false, zone: false, shell: false };
let timer = 0;

export function enabled(): boolean {
  const b = store.config.behavior;
  return b.autoHide && b.position !== 'free';
}

/** Souris sur le dock / au bord, ou touche Windows. */
function signaled(): boolean {
  return sig.inside || sig.zone || (sig.shell && store.config.behavior.revealOnWinKey);
}

/** Interaction en cours qui interdit le repli (sans événement de fin fiable → ré-essai). */
function blocked(): boolean {
  return store.dragging || isOpen() || dockEl.classList.contains('file-over');
}

function wantVisible(): boolean {
  return signaled() || blocked();
}

function setAutoHidden(on: boolean): void {
  if (store.autoHidden === on) return;
  store.autoHidden = on;
  dockEl.classList.toggle('autohidden', on);
  syncPeek();
  wake(700);
}

/** Trait discret : visible seulement quand le dock est replié (et pas en plein écran). */
export function syncPeek(): void {
  const show = store.autoHidden && !store.hidden && store.config.behavior.peekLine;
  peekEl.classList.toggle('show', show);
}

function clear(): void {
  window.clearTimeout(timer);
  timer = 0;
}

/** Ré-évalue l'état. `delay` : délai avant repli (défaut : réglage utilisateur). */
export function evaluate(delay?: number): void {
  if (!enabled()) {
    clear();
    setAutoHidden(false);
    return;
  }
  if (signaled()) {
    clear();
    setAutoHidden(false);
    return;
  }
  if (blocked()) setAutoHidden(false);
  if (store.autoHidden || timer) return;
  timer = window.setTimeout(tick, delay ?? store.config.behavior.autoHideDelay);
}

function tick(): void {
  timer = 0;
  if (!enabled() || signaled()) return;
  if (blocked()) {
    // Popover / glisser en cours : on ré-essaie (pas d'événement fiable à leur fermeture).
    timer = window.setTimeout(tick, 400);
    return;
  }
  setAutoHidden(true);
}

export function setInside(v: boolean): void {
  sig.inside = v;
  evaluate();
}

export function setZone(v: boolean): void {
  sig.zone = v;
  evaluate();
}

export function setShell(v: boolean): void {
  sig.shell = v;
  // Touche Win / menu Démarrer : remontée éclair, en phase avec l'ouverture du menu natif.
  if (v && store.autoHidden && store.config.behavior.revealOnWinKey) {
    dockEl.classList.add('snap');
    window.setTimeout(() => dockEl.classList.remove('snap'), 260);
  }
  // Menu Démarrer refermé : on laisse un peu plus de temps avant de replier.
  evaluate(v ? undefined : Math.max(900, store.config.behavior.autoHideDelay));
}

/** Après un retour de plein écran : repli direct, sans réapparition parasite. */
export function hideNow(): void {
  if (!enabled() || wantVisible()) return;
  clear();
  setAutoHidden(true);
}

/**
 * Zones de réveil (CSS px, relatives à la fenêtre) : bande au bord de l'écran sur la longueur
 * du dock (≥ 160 px). Débordent légèrement de l'écran pour attraper le curseur collé au bord.
 */
export function revealZones(): Surface[] {
  if (!store.autoHidden || store.hidden) return [];
  const barEl = dockEl.querySelector('.dock-bg') as HTMLElement;
  const len = Math.max(160, (vertical() ? barEl.offsetHeight : barEl.offsetWidth) + 24);
  const c = anchor();
  const i = layout.insets;
  const D = 8; // profondeur de la zone (le trait est à 3–7 px du bord)
  const W = window.innerWidth;
  const H = window.innerHeight;
  switch (layout.edge) {
    case 'top':
      return [{ x: c - len / 2, y: -4, w: len, h: i.top + D + 4, r: 0 }];
    case 'left':
      return [{ x: -4, y: c - len / 2, w: i.left + D + 4, h: len, r: 0 }];
    case 'right':
      return [{ x: W - i.right - D, y: c - len / 2, w: i.right + D + 4, h: len, r: 0 }];
    case 'bottom':
      return [{ x: c - len / 2, y: H - i.bottom - D, w: len, h: i.bottom + D + 4, r: 0 }];
    default:
      return [];
  }
}

// Un popover s'ouvre (ex : notification de mise à jour) → le dock remonte pour qu'on le voie.
window.addEventListener('rez-popover', () => evaluate());
