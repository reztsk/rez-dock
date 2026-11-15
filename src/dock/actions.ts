import { t } from '../shared/i18n';
import { api, fileStem } from '../shared/api';
import { MODULES, pinnedItem } from '../shared/defaults';
import type { WinInfo } from '../shared/types';
import { toggleTray } from './tray';
import { closePopover, div, isOpen, openMenu, openPopover, type MenuEntry } from './popover';
import { animate, getSlot } from './render';
import type { Slot } from './slots';
import { defaultInsertIndex, store, updateConfig } from './store';
import { GLYPHS } from '../shared/icons';

function launchSlot(el: HTMLElement, slot: Slot): void {
  if (!slot.launchPath) return;
  if (store.config.appearance.bounce) animate(el, 'bounce');
  void api.launch(slot.launchPath, slot.launchArgs).catch(console.warn);
}

/** Clic gauche. */
export function primary(el: HTMLElement): void {
  const slot = getSlot(el);
  if (!slot) return;

  if (slot.kind === 'app') {
    const wins = slot.windows;
    if (wins.length === 0) {
      launchSlot(el, slot);
    } else if (wins.length === 1) {
      animate(el, 'pulse');
      closePopover();
      void api.activateWindow(wins[0].hwnd);
    } else {
      animate(el, 'pulse');
      showWindowList(el, slot);
    }
    return;
  }

  animate(el, 'pulse');
  switch (slot.item.type) {
    case 'tray':
      toggleTray(el);
      return;
    case 'settings':
      closePopover();
      void api.openSettings();
      return;
    case 'clock':
      closePopover();
      void api.shellAction('notifications');
      return;
    case 'start':
    case 'search':
    case 'taskview':
    case 'desktop':
    case 'trash':
    case 'notifications':
      closePopover();
      void api.shellAction(slot.item.type);
      return;
    default:
      return;
  }
}

/** Clic molette : nouvelle instance. */
export function middle(el: HTMLElement): void {
  const slot = getSlot(el);
  if (slot?.kind === 'app') launchSlot(el, slot);
}

function showWindowList(el: HTMLElement, slot: Slot): void {
  const owner = `wins:${slot.key}`;
  if (isOpen(owner)) {
    closePopover();
    return;
  }
  const root = div('popover-scroll');
  const head = div('menu-header');
  head.textContent = slot.label;
  root.appendChild(head);
  for (const w of slot.windows) root.appendChild(windowRow(w));
  openPopover(root, el, { owner });
}

function windowRow(w: WinInfo): HTMLElement {
  const row = div(`win-row${w.minimized ? ' min' : ''}${w.active ? ' active' : ''}`);
  const label = document.createElement('span');
  label.className = 'label';
  label.textContent = w.title || 'Fenêtre';
  const min = div('icon-btn');
  min.title = 'Réduire';
  min.innerHTML = GLYPHS.minimize;
  min.onclick = (e) => {
    e.stopPropagation();
    void api.minimizeWindow(w.hwnd);
  };
  const close = div('icon-btn');
  close.title = 'Fermer';
  close.innerHTML = GLYPHS.close;
  close.onclick = (e) => {
    e.stopPropagation();
    void api.closeWindow(w.hwnd);
    row.remove();
  };
  row.append(label, min, close);
  row.onclick = () => {
    closePopover();
    void api.focusWindow(w.hwnd);
  };
  return row;
}

/** Clic droit sur un élément. */
export function context(el: HTMLElement, x: number): void {
  const slot = getSlot(el);
  if (!slot) return;
  const entries: MenuEntry[] = [];

  if (slot.kind === 'app') {
    entries.push({ header: slot.label });
    for (const w of slot.windows.slice(0, 8)) {
      entries.push({
        label: w.title || 'Fenêtre',
        icon: 'window',
        active: w.active,
        hint: w.minimized ? 'réduite' : undefined,
        action: () => api.focusWindow(w.hwnd),
      });
    }
    if (slot.windows.length) entries.push('sep');
    if (slot.launchPath) {
      entries.push({ label: slot.windows.length ? 'Nouvelle fenêtre' : 'Ouvrir', icon: 'plus', action: () => launchSlot(el, slot) });
    }
    if (slot.pinned) {
      entries.push({
        label: 'Retirer du dock',
        icon: 'unpin',
        action: () => updateConfig((c) => (c.items = c.items.filter((i) => i.id !== slot.item.id))),
      });
    } else if (slot.exe) {
      const exe = slot.exe;
      entries.push({
        label: 'Épingler au dock',
        icon: 'pin',
        action: async () => {
          const info = await api.appInfo(exe);
          if (!info.name || info.name === fileStem(exe)) info.name = slot.label;
          updateConfig((c) => c.items.splice(defaultInsertIndex(c.items), 0, pinnedItem(info)));
        },
      });
    }
    const loc = slot.exe || (slot.launchPath && /^[a-z]:\\/i.test(slot.launchPath) ? slot.launchPath : '');
    if (loc) entries.push({ label: "Ouvrir l'emplacement", icon: 'folder', action: () => api.openLocation(loc) });
    if (slot.windows.length) {
      entries.push('sep');
      const n = slot.windows.length;
      entries.push({
        label: n > 1 ? t('Fermer les {n} fenêtres', { n }) : 'Fermer la fenêtre',
        icon: 'close',
        danger: true,
        action: () => slot.windows.forEach((w) => void api.closeWindow(w.hwnd)),
      });
    }
  } else {
    const meta = MODULES[slot.item.type];
    entries.push({ header: meta.label });
    entries.push({
      label: 'Masquer ce module',
      icon: 'eyeOff',
      action: () =>
        updateConfig((c) => {
          const it = c.items.find((i) => i.id === slot.item.id);
          if (it) it.enabled = false;
        }),
    });
    entries.push('sep', ...dockEntries());
  }
  openMenu(entries, slot.kind === 'app' || slot.kind === 'module' ? el : x, `ctx:${slot.key}`);
}

function dockEntries(): MenuEntry[] {
  return [
    {
      // Mode épinglé (toujours visible) ⇄ masquage automatique.
      label: store.config.behavior.autoHide ? 'Épingler le dock (toujours visible)' : 'Masquer automatiquement le dock',
      icon: store.config.behavior.autoHide ? 'pin' : 'unpin',
      action: () =>
        updateConfig((c) => {
          c.behavior.autoHide = !c.behavior.autoHide;
        }),
    },
    { label: 'Réglages REZ…', icon: 'sliders', action: () => api.openSettings() },
    { label: 'Gestionnaire des tâches', icon: 'layers', action: () => api.shellAction('taskmgr') },
    { label: 'Quitter REZ', icon: 'power', danger: true, action: () => api.quit() },
  ];
}

/** Clic droit sur le fond de la barre. */
export function dockContext(x: number): void {
  openMenu([{ header: 'REZ' }, ...dockEntries()], x, 'ctx:dock');
}
