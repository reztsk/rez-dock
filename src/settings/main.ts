import { getVersion } from '@tauri-apps/api/app';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { open } from '@tauri-apps/plugin-dialog';
import { api, icon, fileStem } from '../shared/api';
import { defaultConfig, mergeConfig, MODULES, moduleItem, pinnedItem, THEMES } from '../shared/defaults';
import { GLYPHS, type GlyphName } from '../shared/icons';
import { applyAppearance, normalizeHex } from '../shared/theme';
import type { ConfigChanged, DockEdge, DockItem, ModuleType, RezConfig } from '../shared/types';
import { findUpdate, installUpdate, type Update } from '../shared/update';
import { button, card, colorPicker, el, h2, iconButton, page, row, slider, stackRow, toggle } from './ui';

const REPO_URL = 'https://github.com/reztsk/rez-dock';

const content = document.getElementById('content') as HTMLElement;
const nav = document.getElementById('nav') as HTMLElement;
const win = getCurrentWindow();

let cfg: RezConfig;
let section: Section = 'appearance';
let persistTimer = 0;

type Section = 'appearance' | 'items' | 'behavior' | 'updates' | 'about';

const SECTIONS: { id: Section; label: string; icon: GlyphName }[] = [
  { id: 'appearance', label: 'Apparence', icon: 'palette' },
  { id: 'items', label: 'Éléments du dock', icon: 'layers' },
  { id: 'behavior', label: 'Comportement', icon: 'sliders' },
  { id: 'updates', label: 'Mises à jour', icon: 'download' },
  { id: 'about', label: 'À propos', icon: 'info' },
];

// ------------------------------------------------------------------ Persistance

/** Aperçu immédiat dans le dock + écriture disque/natif différée (anti-spam). */
function commit(rerender = false): void {
  applyAppearance(cfg);
  void api.saveConfig(cfg, 'settings', false);
  window.clearTimeout(persistTimer);
  persistTimer = window.setTimeout(() => void api.saveConfig(cfg, 'settings', true), 380);
  if (rerender) renderSection();
}

// ------------------------------------------------------------------ Navigation

function renderNav(): void {
  nav.replaceChildren();
  for (const s of SECTIONS) {
    const item = el('div', `nav-item${s.id === section ? ' on' : ''}`);
    item.innerHTML = GLYPHS[s.icon];
    item.appendChild(document.createTextNode(s.label));
    item.onclick = () => {
      section = s.id;
      renderNav();
      renderSection();
    };
    nav.appendChild(item);
  }
  const foot = el('div', 'nav-foot');
  void getVersion().then((v) => (foot.textContent = `REZ v${v}`));
  nav.appendChild(foot);
}

function renderSection(): void {
  const scroll = content.scrollTop;
  const builders: Record<Section, () => HTMLElement> = {
    appearance: appearancePage,
    items: itemsPage,
    behavior: behaviorPage,
    updates: updatesPage,
    about: aboutPage,
  };
  content.replaceChildren(builders[section]());
  content.scrollTop = scroll;
}

// ------------------------------------------------------------------ Apparence

function appearancePage(): HTMLElement {
  const a = cfg.appearance;
  const p = page('Apparence', 'Expo-Glass : verre flouté natif, teinte, lueur et animations. Tout s’applique en direct sur le dock.');

  p.appendChild(h2('Position & taille'));
  const b = cfg.behavior;
  const seg = el('div', 'segmented');
  const EDGES: { id: DockEdge; label: string }[] = [
    { id: 'bottom', label: 'Bas' },
    { id: 'top', label: 'Haut' },
    { id: 'left', label: 'Gauche' },
    { id: 'right', label: 'Droite' },
    { id: 'free', label: 'Libre' },
  ];
  for (const e of EDGES) {
    const btn = el('button', `seg${b.position === e.id ? ' on' : ''}`, e.label);
    btn.onclick = () => {
      cfg.behavior.position = e.id;
      commit(true);
    };
    seg.appendChild(btn);
  }
  const rows: HTMLElement[] = [
    row('Emplacement', 'Astuce : glissez le fond du dock pour le déplacer ; relâché près d’un bord, il s’y ancre.', seg),
    row(
      'Échelle globale',
      'Agrandit ou réduit tout le dock (icônes, marges, arrondis)',
      slider({ value: a.scale, min: 0.5, max: 2.5, step: 0.05, format: pct, onInput: (v) => ((cfg.appearance.scale = v), commit()) }),
    ),
  ];
  if (b.position === 'free') {
    rows.push(
      row('Position horizontale', null, slider({ value: b.freeX, min: 0.02, max: 0.98, step: 0.005, format: pct, onInput: (v) => ((cfg.behavior.freeX = v), commit()) })),
      row('Position verticale', null, slider({ value: b.freeY, min: 0.02, max: 0.98, step: 0.005, format: pct, onInput: (v) => ((cfg.behavior.freeY = v), commit()) })),
    );
  }
  rows.push(
    row(
      'Préréglage compact',
      'Revenir au format discret par défaut, centré en bas',
      button('Appliquer', {
        icon: 'refresh',
        onClick: () => {
          Object.assign(cfg.appearance, { scale: 1, iconSize: 38, spacing: 5, padding: 6, radius: 16, magnification: 1.6, magnifyRange: 2.8 });
          Object.assign(cfg.behavior, { position: 'bottom', bottomMargin: 8 });
          commit(true);
        },
      }),
    ),
  );
  p.appendChild(card(...rows));

  p.appendChild(h2('Thèmes'));
  const themes = el('div', 'themes');
  for (const t of THEMES) {
    const on = normalizeHex(t.tint) === normalizeHex(a.tint) && normalizeHex(t.accent) === normalizeHex(a.accent);
    const th = el('button', `theme${on ? ' on' : ''}`);
    const prev = el('div', 'theme-preview');
    const bar = el('div', 'bar');
    bar.style.background = `rgba(${hexRgb(t.tint)}, ${Math.max(0.35, t.opacity)})`;
    bar.style.boxShadow = `0 0 ${12 * t.glow}px ${t.accent}, inset 0 1px 0 rgba(255,255,255,.4)`;
    bar.style.setProperty('--t-accent', t.accent);
    bar.append(el('i'), el('i'), el('i'));
    prev.appendChild(bar);
    th.append(prev, el('span', '', t.name));
    th.onclick = () => {
      Object.assign(cfg.appearance, { tint: t.tint, opacity: t.opacity, accent: t.accent, glow: t.glow, blur: t.blur });
      commit(true);
    };
    themes.appendChild(th);
  }
  p.appendChild(themes);

  p.appendChild(h2('Verre'));
  p.appendChild(
    card(
      stackRow('Teinte du verre', null, colorPicker(a.tint, (hex) => ((cfg.appearance.tint = hex), commit()))),
      row(
        'Opacité de la teinte',
        'Transparence de la couleur posée sur le flou',
        slider({ value: a.opacity, min: 0, max: 1, step: 0.01, format: pct, onInput: (v) => ((cfg.appearance.opacity = v), commit()) }),
      ),
      row(
        'Intensité du flou',
        '0 = désactivé · 1–49 = flou DWM · 50–100 = acrylique (flou intense + grain)',
        slider({ value: a.blur, min: 0, max: 100, step: 1, format: blurLabel, onInput: (v) => ((cfg.appearance.blur = v), commit()) }),
      ),
      row(
        'Lueur des bordures',
        'Reflets et halo néon autour du dock',
        slider({ value: a.glow, min: 0, max: 1, step: 0.01, format: pct, onInput: (v) => ((cfg.appearance.glow = v), commit()) }),
      ),
      row(
        'Rayon des arrondis',
        null,
        slider({ value: a.radius, min: 0, max: 40, step: 1, format: px, onInput: (v) => ((cfg.appearance.radius = v), commit()) }),
      ),
    ),
  );

  p.appendChild(h2('Couleur d’accent'));
  p.appendChild(card(stackRow('Accent', 'Indicateurs, halos, modules intégrés', colorPicker(a.accent, (hex) => ((cfg.appearance.accent = hex), commit())))));

  p.appendChild(h2('Icônes & magnification'));
  p.appendChild(
    card(
      row('Taille des icônes', null, slider({ value: a.iconSize, min: 32, max: 96, step: 1, format: px, onInput: (v) => ((cfg.appearance.iconSize = v), commit()) })),
      row(
        'Magnification',
        'Agrandissement maximal au survol (1× = désactivé)',
        slider({ value: a.magnification, min: 1, max: 2.6, step: 0.05, format: (v) => `${v.toFixed(2)}×`, onInput: (v) => ((cfg.appearance.magnification = v), commit()) }),
      ),
      row(
        'Largeur de la vague',
        'Nombre d’icônes voisines affectées',
        slider({ value: a.magnifyRange, min: 1.5, max: 5, step: 0.1, format: (v) => v.toFixed(1), onInput: (v) => ((cfg.appearance.magnifyRange = v), commit()) }),
      ),
      row('Espacement', null, slider({ value: a.spacing, min: 0, max: 24, step: 1, format: px, onInput: (v) => ((cfg.appearance.spacing = v), commit()) })),
      row('Marge intérieure', null, slider({ value: a.padding, min: 4, max: 20, step: 1, format: px, onInput: (v) => ((cfg.appearance.padding = v), commit()) })),
      row(
        'Distance au bord de l’écran',
        'Espace flottant sous le dock',
        slider({ value: cfg.behavior.bottomMargin, min: 0, max: 40, step: 1, format: px, onInput: (v) => ((cfg.behavior.bottomMargin = v), commit()) }),
      ),
      row('Noms au survol', null, toggle(a.showLabels, (v) => ((cfg.appearance.showLabels = v), commit()))),
      row('Indicateurs d’apps ouvertes', null, toggle(a.showIndicators, (v) => ((cfg.appearance.showIndicators = v), commit()))),
      row('Rebond au lancement', null, toggle(a.bounce, (v) => ((cfg.appearance.bounce = v), commit()))),
    ),
  );
  return p;
}

// ------------------------------------------------------------------ Éléments

let dragId: string | null = null;

function itemsPage(): HTMLElement {
  const p = page('Éléments du dock', 'Glissez pour réorganiser, activez/désactivez, renommez (double-clic) ou ajoutez des applications et modules. Astuce : déposez un .exe/.lnk directement sur le dock.');
  const list = el('div', 'items-list');
  cfg.items.forEach((item) => list.appendChild(itemRow(item)));
  p.appendChild(card(list));

  const bar = el('div', 'add-bar');
  bar.appendChild(button('Ajouter une application', { icon: 'plus', kind: 'primary', onClick: () => void addApp() }));
  bar.appendChild(
    button('Séparateur', {
      icon: 'plus',
      onClick: () => {
        cfg.items.push(moduleItem('separator'));
        commit(true);
      },
    }),
  );
  const select = el('select');
  const missing = (Object.keys(MODULES) as ModuleType[]).filter(
    (t) => MODULES[t].unique && !cfg.items.some((i) => i.type === t),
  );
  select.appendChild(new Option(missing.length ? '+ Ajouter un module…' : 'Tous les modules sont présents', ''));
  for (const t of missing) select.appendChild(new Option(MODULES[t].label, t));
  select.disabled = !missing.length;
  select.onchange = () => {
    const t = select.value as ModuleType;
    if (!t) return;
    cfg.items.push(moduleItem(t));
    commit(true);
  };
  bar.appendChild(select);
  bar.appendChild(
    button('Réinitialiser', {
      icon: 'refresh',
      kind: 'danger',
      onClick: async () => {
        if (!confirm('Réinitialiser l’agencement du dock (les réglages d’apparence sont conservés) ?')) return;
        const fresh = defaultConfig(await api.defaultPins());
        cfg.items = fresh.items;
        commit(true);
      },
    }),
  );
  p.appendChild(bar);
  return p;
}

function itemRow(item: DockItem): HTMLElement {
  const meta = MODULES[item.type];
  const r = el('div', `item-row${item.enabled ? '' : ' disabled'}`);
  r.draggable = true;
  r.dataset.id = item.id;

  const grip = el('div', 'grip');
  grip.innerHTML = GLYPHS.grip;

  const main = el('div', 'item-main');
  const ic = el('div', 'item-icon');
  if (item.type === 'pinned') {
    const src = item.icon || item.path;
    if (src) {
      void icon(src, 64).then((data) => {
        if (!data) return;
        const im = el('img');
        im.src = data;
        ic.replaceChildren(im);
      });
    }
  } else if (item.type === 'separator') {
    ic.classList.add('sep');
  } else {
    ic.classList.add('glyph');
    ic.innerHTML = meta.glyph ? GLYPHS[meta.glyph] : GLYPHS.window;
  }

  const text = el('div', 'item-text');
  const name = el('div', 'item-name', item.type === 'pinned' ? item.name || fileStem(item.path ?? '') : meta.label);
  const type = el('div', 'item-type', item.type === 'pinned' ? (item.path ?? '') : meta.description);
  text.append(name, type);
  main.append(ic, text);

  if (item.type === 'pinned') {
    name.ondblclick = () => {
      const input = el('input', 'text-input');
      input.value = item.name ?? '';
      name.replaceChildren(input);
      input.focus();
      input.select();
      const done = () => {
        item.name = input.value.trim() || fileStem(item.path ?? '');
        commit(true);
      };
      input.onkeydown = (e) => {
        if (e.key === 'Enter') done();
        if (e.key === 'Escape') renderSection();
      };
      input.onblur = done;
    };
  }

  const actions = el('div', 'item-actions');
  if (item.type === 'pinned') {
    actions.appendChild(iconButton('palette', 'Changer l’icône', () => void changeIcon(item)));
    if (item.path) actions.appendChild(iconButton('window', 'Lancer', () => void api.launch(item.path!, item.args)));
  }
  actions.appendChild(toggle(item.enabled, (v) => ((item.enabled = v), r.classList.toggle('disabled', !v), commit()), true));
  actions.appendChild(
    iconButton(
      'close',
      'Retirer',
      () => {
        cfg.items = cfg.items.filter((i) => i.id !== item.id);
        commit(true);
      },
      true,
    ),
  );

  r.append(grip, main, actions);

  // --- Glisser-déposer HTML5 (le gestionnaire de fichiers Tauri est désactivé pour cette fenêtre)
  r.addEventListener('dragstart', (e) => {
    dragId = item.id;
    r.classList.add('dragging');
    e.dataTransfer?.setData('text/plain', item.id);
    if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move';
  });
  r.addEventListener('dragend', () => {
    dragId = null;
    r.classList.remove('dragging');
    document.querySelectorAll('.drop-before, .drop-after').forEach((n) => n.classList.remove('drop-before', 'drop-after'));
  });
  r.addEventListener('dragover', (e) => {
    if (!dragId || dragId === item.id) return;
    e.preventDefault();
    const rect = r.getBoundingClientRect();
    const after = e.clientY > rect.top + rect.height / 2;
    r.classList.toggle('drop-after', after);
    r.classList.toggle('drop-before', !after);
  });
  r.addEventListener('dragleave', () => r.classList.remove('drop-before', 'drop-after'));
  r.addEventListener('drop', (e) => {
    e.preventDefault();
    if (!dragId || dragId === item.id) return;
    const after = r.classList.contains('drop-after');
    const moving = cfg.items.find((i) => i.id === dragId);
    if (!moving) return;
    const rest = cfg.items.filter((i) => i.id !== dragId);
    const idx = rest.findIndex((i) => i.id === item.id);
    rest.splice(after ? idx + 1 : idx, 0, moving);
    cfg.items = rest;
    commit(true);
  });
  return r;
}

async function addApp(): Promise<void> {
  const picked = await open({
    multiple: true,
    title: 'Ajouter des applications au dock',
    filters: [{ name: 'Applications et raccourcis', extensions: ['exe', 'lnk', 'url', 'bat', 'cmd', 'appref-ms'] }],
  });
  if (!picked) return;
  const paths = Array.isArray(picked) ? picked : [picked];
  const run = cfg.items.findIndex((i) => i.type === 'running');
  let at = run >= 0 ? run : cfg.items.length;
  for (const path of paths) {
    const info = await api.appInfo(path);
    cfg.items.splice(at++, 0, pinnedItem(info));
  }
  commit(true);
}

async function changeIcon(item: DockItem): Promise<void> {
  const picked = await open({
    multiple: false,
    title: 'Choisir une icône',
    filters: [{ name: 'Icônes', extensions: ['ico', 'png', 'jpg', 'jpeg', 'webp', 'exe', 'dll'] }],
  });
  if (!picked || Array.isArray(picked)) return;
  item.icon = picked;
  commit(true);
}

// ------------------------------------------------------------------ Comportement

function behaviorPage(): HTMLElement {
  const b = cfg.behavior;
  const p = page('Comportement', 'Remplacement de la barre des tâches, mode gaming et démarrage.');

  p.appendChild(h2('Barre des tâches'));
  p.appendChild(
    card(
      row(
        'Masquer la barre des tâches Windows',
        'REZ la remplace. Elle est restaurée automatiquement en quittant REZ (même après un crash, au lancement suivant).',
        toggle(b.hideTaskbar, (v) => ((cfg.behavior.hideTaskbar = v), commit())),
      ),
      row(
        'Réserver l’espace du dock',
        'Les fenêtres maximisées s’arrêtent au-dessus du dock',
        toggle(b.reserveSpace, (v) => ((cfg.behavior.reserveSpace = v), commit())),
      ),
    ),
  );

  p.appendChild(h2('Mode gaming sécurisé'));
  const security = el('div', 'row stack');
  security.innerHTML = `
    <div class="text"><div class="title">${GLYPHS.shield.replace('<svg', '<svg style="width:16px;height:16px;vertical-align:-3px;margin-right:6px"')}100 % passif — compatible anti-cheat</div>
    <div class="desc">REZ n’ouvre jamais le processus d’un jeu et ne lit/écrit aucune mémoire.</div></div>
    <ul class="info-list">
      <li><b>Détection</b> : <span class="mono">GetForegroundWindow</span>, <span class="mono">GetClientRect</span>, <span class="mono">MonitorFromWindow</span>, <span class="mono">SHQueryUserNotificationState</span> (API fenêtrées publiques).</li>
      <li><b>Aucun</b> hook clavier/souris, aucune injection DLL, aucun <span class="mono">ReadProcessMemory</span>, aucun overlay dans le jeu.</li>
      <li><b>En plein écran</b> : le dock glisse hors de l’écran puis ses fenêtres sont réellement masquées ; toute énumération de fenêtres est suspendue.</li>
    </ul>`;
  p.appendChild(
    card(
      row(
        'Retrait automatique en plein écran',
        'Jeux, vidéos et présentations plein écran (exclusif ou fenêtré sans bordure)',
        toggle(b.gamingMode, (v) => ((cfg.behavior.gamingMode = v), commit())),
      ),
      security,
    ),
  );

  p.appendChild(h2('Horloge'));
  p.appendChild(
    card(
      row('Format 24 heures', null, toggle(b.clock24h, (v) => ((cfg.behavior.clock24h = v), commit()))),
      row('Afficher les secondes', null, toggle(b.clockSeconds, (v) => ((cfg.behavior.clockSeconds = v), commit()))),
    ),
  );

  p.appendChild(h2('Système'));
  p.appendChild(
    card(
      row('Lancer REZ au démarrage de Windows', null, toggle(b.autostart, (v) => ((cfg.behavior.autostart = v), commit()))),
      row('Quitter REZ', 'Ferme le dock et restaure la barre des tâches Windows', button('Quitter', { icon: 'power', kind: 'danger', onClick: () => void api.quit() })),
    ),
  );
  return p;
}

// ------------------------------------------------------------------ Mises à jour

let pendingUpdate: Update | null = null;

function updatesPage(): HTMLElement {
  const p = page('Mises à jour', 'REZ vérifie discrètement les releases GitHub au lancement. Installation en un clic, signature vérifiée.');
  const status = el('div', 'desc', 'Cliquez sur « Rechercher » pour vérifier.');
  const versionBadge = el('span', 'badge', '…');
  void getVersion().then((v) => (versionBadge.textContent = `v${v}`));

  const notes = el('div', 'notes');
  notes.style.display = 'none';
  const progress = el('div', 'progress');
  progress.style.display = 'none';
  const bar = el('i');
  progress.appendChild(bar);

  const install = button('Mettre à jour', {
    icon: 'download',
    kind: 'primary',
    onClick: async () => {
      if (!pendingUpdate) return;
      install.disabled = true;
      check.disabled = true;
      progress.style.display = '';
      status.textContent = 'Téléchargement…';
      try {
        await installUpdate(pendingUpdate, (ratio) => {
          bar.style.width = ratio === null ? '50%' : `${Math.round(ratio * 100)}%`;
          if (ratio === 1) status.textContent = 'Installation puis redémarrage…';
        });
      } catch (e) {
        status.textContent = `Échec : ${String(e)}`;
        install.disabled = false;
        check.disabled = false;
      }
    },
  });
  install.style.display = 'none';

  const check = button('Rechercher', {
    icon: 'refresh',
    onClick: async () => {
      check.disabled = true;
      status.textContent = 'Recherche en cours…';
      pendingUpdate = await findUpdate();
      check.disabled = false;
      if (pendingUpdate) {
        status.textContent = `Nouvelle version disponible : v${pendingUpdate.version}`;
        install.style.display = '';
        if (pendingUpdate.body?.trim()) {
          notes.textContent = pendingUpdate.body.trim();
          notes.style.display = '';
        }
      } else {
        status.innerHTML = '<span class="badge ok">À jour</span> Vous utilisez la dernière version.';
        install.style.display = 'none';
        notes.style.display = 'none';
      }
    },
  });

  const controls = el('div', 'btn-row');
  controls.append(check, install);
  const head = row('Version installée', null, versionBadge);
  p.appendChild(
    card(
      head,
      stackRow('État', null, status, progress, notes, controls),
      row('Vérifier au lancement', 'Recherche silencieuse 6 s après le démarrage', toggle(cfg.behavior.checkUpdates, (v) => ((cfg.behavior.checkUpdates = v), commit()))),
      row('Releases GitHub', REPO_URL.replace('https://', ''), button('Ouvrir', { icon: 'folder', onClick: () => void api.launch(`${REPO_URL}/releases`) })),
    ),
  );
  return p;
}

// ------------------------------------------------------------------ À propos

function aboutPage(): HTMLElement {
  const p = page('À propos', '');
  p.querySelector('.lead')?.remove();
  const hero = el('div', 'about-hero');
  const logo = el('img');
  logo.src = '/rez-logo.png';
  const txt = el('div');
  const name = el('div', 'name', 'REZ');
  const v = el('div', 'desc', 'Dock Expo-Glass pour Windows');
  void getVersion().then((ver) => (v.textContent = `Dock Expo-Glass pour Windows · v${ver}`));
  txt.append(name, v);
  hero.append(logo, txt);
  p.appendChild(card(hero));

  p.appendChild(h2('Technologie'));
  const info = el('div', 'row stack');
  info.innerHTML = `<ul class="info-list">
    <li><b>Tauri 2 + Rust</b> : binaire natif léger, WebView2 système.</li>
    <li><b>Verre natif</b> : fenêtre DWM (blur/acrylique) dont la forme suit la barre en temps réel.</li>
    <li><b>Icônes cachées</b> : registre <span class="mono">NotifyIconSettings</span> + liste des processus — aucune lecture mémoire d’explorer.</li>
    <li><b>Mises à jour</b> : releases GitHub signées (minisign), installeur NSIS sans droits admin.</li>
  </ul>`;
  p.appendChild(card(info));
  return p;
}

// ------------------------------------------------------------------ Utils

function pct(v: number): string {
  return `${Math.round(v * 100)} %`;
}
function px(v: number): string {
  return `${Math.round(v)} px`;
}
function blurLabel(v: number): string {
  if (v <= 0.5) return 'Off';
  return v < 50 ? `Flou ${Math.round(v)}` : `Acryl. ${Math.round(v)}`;
}
function hexRgb(hex: string): string {
  const h = normalizeHex(hex).slice(1);
  const n = parseInt(h, 16);
  return `${(n >> 16) & 255}, ${(n >> 8) & 255}, ${n & 255}`;
}

// ------------------------------------------------------------------ Init

document.getElementById('btn-min')!.onclick = () => void win.minimize();
document.getElementById('btn-close')!.onclick = () => void win.close();
document.addEventListener('contextmenu', (e) => {
  if (!(e.target instanceof HTMLInputElement)) e.preventDefault();
});

async function init(): Promise<void> {
  cfg = mergeConfig(await api.getConfig());
  applyAppearance(cfg);
  renderNav();
  renderSection();
  await listen<ConfigChanged>('config-changed', (e) => {
    if (e.payload.source === 'settings') return;
    cfg = mergeConfig(e.payload.config);
    applyAppearance(cfg);
    if (section === 'items') renderSection();
  });
}

void init();
