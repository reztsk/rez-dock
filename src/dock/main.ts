import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { api } from '../shared/api';
import { defaultConfig, mergeConfig, pinnedItem } from '../shared/defaults';
import { autoTranslate, setLang } from '../shared/i18n';
import { applyAppearance } from '../shared/theme';
import type { ConfigChanged, WinInfo } from '../shared/types';
import { context, dockContext, middle, primary } from './actions';
import * as autohide from './autohide';
import { clickSuppressed, onPointerDown } from './drag';
import { hoverZone, measure, setHover, syncSurfaces, wake } from './engine';
import { applyLayout, axis, layout, nearestEdge, setAnchor, vertical, type Insets } from './layout';
import { closePopover } from './popover';
import { render, slotElement, updateClocks } from './render';
import { setLabelListener } from './slots';
import { defaultInsertIndex, IS_PRIMARY, LABEL, notify, onChange, store, updateConfig } from './store';
import { checkAtStartup } from './updater';

const dockEl = document.getElementById('dock') as HTMLElement;
const itemsEl = document.getElementById('items') as HTMLElement;

let renderQueued = false;
function scheduleRender(): void {
  if (renderQueued) return;
  renderQueued = true;
  requestAnimationFrame(() => {
    renderQueued = false;
    if (store.dragging) {
      setTimeout(scheduleRender, 120);
      return;
    }
    render();
  });
}

function setHidden(hidden: boolean, intro = false): void {
  // Retour de plein écran : le dock reste replié (seul le trait réapparaît).
  if (!hidden && !intro) autohide.hideNow();
  store.hidden = hidden;
  dockEl.classList.toggle('hidden', hidden);
  if (hidden) {
    closePopover(true);
    setHover(false);
  }
  autohide.syncPeek();
  wake(700);
}

function relayout(): void {
  applyLayout();
  measure();
  syncSurfaces(true);
}

async function init(): Promise<void> {
  let raw = await api.getConfig();
  // Écran secondaire au tout premier lancement : on attend la config créée par le dock principal.
  for (let i = 0; !raw && !IS_PRIMARY && i < 40; i++) {
    await new Promise((r) => setTimeout(r, 250));
    raw = await api.getConfig();
  }
  if (raw) {
    store.config = mergeConfig(raw);
  } else {
    // Premier lancement : préréglage compact centré en bas + détection des apps courantes.
    store.config = defaultConfig(await api.defaultPins().catch(() => []));
    await api.saveConfig(store.config, LABEL, true);
  }
  setLang(store.config.behavior.language);
  autoTranslate(document.getElementById('popover-layer') as HTMLElement);
  layout.insets = await invoke<Insets>('get_layout').catch(() => layout.insets);
  applyAppearance(store.config);
  applyLayout();
  store.windows = await api.getWindows();

  onChange(() => {
    setLang(store.config.behavior.language);
    applyAppearance(store.config);
    applyLayout();
    updateClocks();
    scheduleRender();
    autohide.syncPeek();
    autohide.evaluate();
  });
  setLabelListener(scheduleRender);
  render();

  await listen<ConfigChanged>('config-changed', (e) => {
    if (e.payload.source === LABEL) return;
    store.config = mergeConfig(e.payload.config);
    notify();
  });
  await listen<WinInfo[]>('windows-changed', (e) => {
    store.windows = e.payload;
    scheduleRender();
  });
  await listen<boolean>('cursor-inside', (e) => {
    if (!e.payload) setHover(false);
    autohide.setInside(e.payload);
  });
  // Masquage auto : souris collée au bord (zone de réveil) / touche Windows ou menu Démarrer.
  await listen<boolean>('reveal-zone', (e) => autohide.setZone(e.payload));
  await listen<boolean>('shell-reveal', (e) => autohide.setShell(e.payload));
  await listen<boolean>('gaming', (e) => setHidden(e.payload));
  await listen<Insets>('layout-changed', (e) => {
    layout.insets = e.payload;
    relayout();
  });

  // Dépôt de fichiers (.exe, .lnk, .url) depuis l'Explorateur = épingler.
  await getCurrentWebview().onDragDropEvent(async (e) => {
    const p = e.payload;
    if (p.type === 'over' || p.type === 'enter') {
      dockEl.classList.add('file-over');
      autohide.evaluate();
    } else if (p.type === 'leave') {
      dockEl.classList.remove('file-over');
      autohide.evaluate();
    } else if (p.type === 'drop') {
      dockEl.classList.remove('file-over');
      autohide.evaluate();
      const r = window.devicePixelRatio;
      const pos = axis(p.position.x / r, p.position.y / r);
      const infos = await Promise.all(p.paths.map((path) => api.appInfo(path)));
      updateConfig((c) => {
        let index = dropIndex(pos) ?? defaultInsertIndex(c.items);
        for (const info of infos) c.items.splice(index++, 0, pinnedItem(info));
      });
    }
  });

  // Animation d'entrée (slide + fade) une fois le premier rendu posé ; puis repli auto après 1,6 s
  // (l'utilisateur voit que le dock est là).
  const gaming = await api.isGaming().catch(() => false);
  requestAnimationFrame(() =>
    setTimeout(() => {
      setHidden(gaming, true);
      autohide.evaluate(1600);
    }, 60),
  );

  // Mise à jour : vérifiée par le dock de l'écran principal uniquement (une seule notification).
  if (IS_PRIMARY && store.config.behavior.checkUpdates && !import.meta.env.DEV) {
    setTimeout(() => void checkAtStartup(), 6000);
  }
}

/** Index de config correspondant à une position le long de l'axe du dock. */
function dropIndex(pos: number): number | null {
  const vert = vertical();
  for (const el of Array.from(itemsEl.children) as HTMLElement[]) {
    const r = el.getBoundingClientRect();
    if (pos < (vert ? r.top + r.height / 2 : r.left + r.width / 2)) {
      const key = el.dataset.key ?? '';
      const id = key.startsWith('run:') ? store.config.items.find((i) => i.type === 'running')?.id : key;
      const idx = store.config.items.findIndex((i) => i.id === id);
      return idx >= 0 ? idx : null;
    }
  }
  return null;
}

// ------------------------------------------------------------------ Déplacement libre du dock
// Glisser le FOND du dock (pas une icône) : il se détache et suit la souris.
// Relâché près d'un bord → s'y ancre ; ailleurs → placement libre (position mémorisée).

interface Move {
  id: number;
  sx: number;
  sy: number;
  ox: number;
  oy: number;
  active: boolean;
}
let move: Move | null = null;

dockEl.addEventListener('pointerdown', (e) => {
  if (e.button !== 0 || slotElement(e.target)) return;
  const r = dockEl.getBoundingClientRect();
  move = { id: e.pointerId, sx: e.clientX, sy: e.clientY, ox: e.clientX - (r.left + r.width / 2), oy: e.clientY - (r.top + r.height / 2), active: false };
});

window.addEventListener('pointermove', (e) => {
  if (!move || e.pointerId !== move.id) return;
  if (!move.active) {
    if (Math.hypot(e.clientX - move.sx, e.clientY - move.sy) < 8) return;
    move.active = true;
    store.dragging = true;
    closePopover(true);
    setHover(false);
    dockEl.setPointerCapture(e.pointerId);
    dockEl.classList.add('moving');
    // Bascule en mode libre centré sur la position actuelle (sans saut).
    const r = dockEl.getBoundingClientRect();
    setAnchor(r.left + r.width / 2, r.top + r.height / 2);
    store.config.behavior.position = 'free';
    applyLayoutKeepAnchor();
  }
  setAnchor(e.clientX - move.ox, e.clientY - move.oy);
  dockEl.classList.toggle('snap-hint', nearestEdge(e.clientX, e.clientY) !== null);
  wake(200);
});

function applyLayoutKeepAnchor(): void {
  const ax = getComputedStyle(document.documentElement).getPropertyValue('--ax');
  const ay = getComputedStyle(document.documentElement).getPropertyValue('--ay');
  applyLayout();
  document.documentElement.style.setProperty('--ax', ax);
  document.documentElement.style.setProperty('--ay', ay);
}

function endMove(e: PointerEvent): void {
  if (!move || e.pointerId !== move.id) return;
  const m = move;
  move = null;
  if (!m.active) return;
  store.dragging = false;
  autohide.evaluate();
  dockEl.classList.remove('moving', 'snap-hint');
  const snap = nearestEdge(e.clientX, e.clientY);
  const r = dockEl.getBoundingClientRect();
  updateConfig((c) => {
    if (snap) {
      c.behavior.position = snap;
    } else {
      c.behavior.position = 'free';
      c.behavior.freeX = Math.min(0.98, Math.max(0.02, (r.left + r.width / 2) / window.innerWidth));
      c.behavior.freeY = Math.min(0.98, Math.max(0.02, (r.top + r.height / 2) / window.innerHeight));
    }
  });
  requestAnimationFrame(relayout);
}
window.addEventListener('pointerup', endMove);
window.addEventListener('pointercancel', endMove);

// ------------------------------------------------------------------ Entrées

document.addEventListener('contextmenu', (e) => e.preventDefault());

document.addEventListener('mousemove', (e) => {
  if (store.hidden || store.autoHidden || move?.active) return;
  const z = hoverZone();
  const inside = e.clientX >= z.left && e.clientX <= z.right && e.clientY >= z.top && e.clientY <= z.bottom;
  setHover(inside, axis(e.clientX, e.clientY));
});
document.addEventListener('mouseleave', () => setHover(false));

itemsEl.addEventListener('pointerdown', (e) => {
  const el = slotElement(e.target);
  if (!el) return;
  if (e.button === 0) el.classList.add('pressed');
  onPointerDown(e, el);
});
window.addEventListener('pointerup', () => {
  itemsEl.querySelectorAll('.slot.pressed').forEach((n) => n.classList.remove('pressed'));
});

itemsEl.addEventListener('click', (e) => {
  if (clickSuppressed()) return;
  const el = slotElement(e.target);
  if (el) primary(el);
});

itemsEl.addEventListener('auxclick', (e) => {
  if (e.button !== 1) return;
  const el = slotElement(e.target);
  if (el) middle(el);
});

dockEl.addEventListener('contextmenu', (e) => {
  const el = slotElement(e.target);
  const pos = axis(e.clientX, e.clientY);
  if (el) context(el, pos);
  else dockContext(pos);
});

window.addEventListener('resize', relayout);

void init().catch((err) => console.error('[REZ] init', err));
