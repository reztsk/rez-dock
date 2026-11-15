/**
 * Panneau "Icônes cachées" (équivalent de la flèche ^ de Windows) :
 *  - onglet Zone de notification : apps ayant une icône de notification ET en cours d'exécution
 *  - onglet Arrière-plan : processus utilisateur sans fenêtre
 */
import { api, icon } from '../shared/api';
import { GLYPHS } from '../shared/icons';
import type { BgProcess, TrayApp } from '../shared/types';
import { axis } from './layout';
import { closePopover, div, isOpen, openMenu, openPopover, repositionPopover } from './popover';

type Tab = 'tray' | 'bg';
let tab: Tab = 'tray';

export function toggleTray(anchor: HTMLElement): void {
  if (isOpen('tray')) {
    closePopover();
    return;
  }
  const root = div('tray-root');
  root.style.display = 'contents';
  openPopover(root, anchor, { className: 'tray', owner: 'tray' });
  void render(root, anchor);
}

async function render(root: HTMLElement, anchor: HTMLElement): Promise<void> {
  root.replaceChildren();

  const head = div('tray-head');
  const tabs = div('tabs');
  const tTray = tabBtn('Notification', tab === 'tray');
  const tBg = tabBtn('Arrière-plan', tab === 'bg');
  tTray.onclick = () => {
    tab = 'tray';
    void render(root, anchor);
  };
  tBg.onclick = () => {
    tab = 'bg';
    void render(root, anchor);
  };
  tabs.append(tTray, tBg);
  const refresh = div('icon-btn');
  refresh.title = 'Actualiser';
  refresh.innerHTML = GLYPHS.refresh;
  refresh.onclick = () => void render(root, anchor);
  head.append(tabs, refresh);
  root.appendChild(head);

  const body = div('popover-scroll');
  body.appendChild(div('spinner'));
  root.appendChild(body);

  const foot = div('tray-foot');
  const settings = div('link-btn');
  settings.textContent = 'Paramètres des icônes Windows';
  settings.onclick = () => {
    closePopover();
    void api.shellAction('tray-settings');
  };
  const taskmgr = div('link-btn');
  taskmgr.textContent = 'Gestionnaire des tâches';
  taskmgr.onclick = () => {
    closePopover();
    void api.shellAction('taskmgr');
  };
  foot.append(settings, taskmgr);
  root.appendChild(foot);

  if (tab === 'tray') {
    const apps = await api.trayApps().catch(() => [] as TrayApp[]);
    setCount(tTray, apps.length);
    body.replaceChildren(apps.length ? trayGrid(apps) : empty('Aucune application dans la zone de notification.'));
  } else {
    const procs = await api.backgroundProcesses().catch(() => [] as BgProcess[]);
    setCount(tBg, procs.length);
    body.replaceChildren(procs.length ? bgList(procs, root, anchor) : empty('Aucun processus en arrière-plan.'));
  }
  repositionPopover(anchor);
}

function tabBtn(label: string, on: boolean): HTMLElement {
  const t = div(`tab${on ? ' on' : ''}`);
  t.textContent = label;
  return t;
}

function setCount(t: HTMLElement, n: number): void {
  const c = document.createElement('span');
  c.className = 'count';
  c.textContent = String(n);
  t.appendChild(c);
}

function empty(text: string): HTMLElement {
  const e = div('tray-empty');
  e.textContent = text;
  return e;
}

function letter(name: string): HTMLElement {
  const l = div('letter');
  l.textContent = (name.trim()[0] ?? '?').toUpperCase();
  return l;
}

function trayGrid(apps: TrayApp[]): HTMLElement {
  const grid = div('tray-grid');
  for (const app of apps) {
    const cell = div(`tray-app${app.promoted ? ' promoted' : ''}`);
    cell.title = app.tooltip || app.name;
    if (app.icon) {
      const im = document.createElement('img');
      im.src = app.icon;
      cell.appendChild(im);
    } else {
      cell.appendChild(letter(app.name));
    }
    const label = document.createElement('span');
    label.textContent = app.name;
    cell.appendChild(label);

    // Relancer l'exe d'une app tray ramène sa fenêtre (instance unique) — aucun accès au processus.
    cell.onclick = () => {
      closePopover();
      void api.launch(app.exePath);
    };
    cell.oncontextmenu = (e) => {
      e.preventDefault();
      openMenu(
        [
          { header: app.name },
          { label: 'Ouvrir', icon: 'window', action: () => api.launch(app.exePath) },
          { label: "Ouvrir l'emplacement", icon: 'folder', action: () => api.openLocation(app.exePath) },
          'sep',
          { label: 'Forcer la fermeture', icon: 'kill', danger: true, action: () => void api.killProcesses(app.pids) },
        ],
        axis(e.clientX, e.clientY),
      );
    };
    grid.appendChild(cell);
  }
  return grid;
}

function bgList(procs: BgProcess[], root: HTMLElement, anchor: HTMLElement): HTMLElement {
  const list = div('tray-list');
  for (const p of procs) {
    const row = div('proc-row');
    row.title = p.path;
    const ph = div('ph');
    row.appendChild(ph);
    void icon(p.path, 48).then((src) => {
      if (!src) return;
      const im = document.createElement('img');
      im.src = src;
      ph.replaceWith(im);
    });
    const name = div('name');
    name.textContent = p.name;
    const count = div('count');
    count.textContent = p.pids.length > 1 ? `×${p.pids.length}` : '';
    const kill = div('kill-btn');
    kill.textContent = 'Terminer';
    kill.onclick = async (e) => {
      e.stopPropagation();
      if (!kill.classList.contains('confirm')) {
        kill.classList.add('confirm');
        kill.textContent = 'Confirmer ?';
        setTimeout(() => {
          kill.classList.remove('confirm');
          kill.textContent = 'Terminer';
        }, 2500);
        return;
      }
      await api.killProcesses(p.pids);
      setTimeout(() => void render(root, anchor), 400);
    };
    row.append(name, count, kill);
    row.ondblclick = () => void api.openLocation(p.path);
    list.appendChild(row);
  }
  return list;
}
