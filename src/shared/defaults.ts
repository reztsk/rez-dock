import type { AppInfo, Appearance, Behavior, DockItem, ModuleType, RezConfig } from './types';
import { uid } from './api';
import type { GlyphName } from './icons';

export const CONFIG_VERSION = 1;

export interface ModuleMeta {
  type: ModuleType;
  label: string;
  description: string;
  glyph: GlyphName | null;
  unique: boolean;
}

export const MODULES: Record<ModuleType, ModuleMeta> = {
  start: { type: 'start', label: 'Démarrer', description: 'Ouvre le menu Démarrer', glyph: 'windows', unique: true },
  search: { type: 'search', label: 'Recherche', description: 'Recherche Windows (Win+S)', glyph: 'search', unique: true },
  taskview: { type: 'taskview', label: 'Vue des tâches', description: 'Bureaux virtuels (Win+Tab)', glyph: 'taskview', unique: true },
  pinned: { type: 'pinned', label: 'Application', description: 'Application épinglée', glyph: null, unique: false },
  running: { type: 'running', label: 'Apps ouvertes', description: 'Fenêtres ouvertes non épinglées', glyph: null, unique: true },
  separator: { type: 'separator', label: 'Séparateur', description: 'Trait vertical', glyph: null, unique: false },
  tray: { type: 'tray', label: 'Icônes cachées', description: 'Zone de notification + apps en arrière-plan', glyph: 'tray', unique: true },
  clock: { type: 'clock', label: 'Horloge', description: 'Heure et date (clic : calendrier)', glyph: null, unique: true },
  desktop: { type: 'desktop', label: 'Bureau', description: 'Afficher le bureau (Win+D)', glyph: 'desktop', unique: true },
  trash: { type: 'trash', label: 'Corbeille', description: 'Ouvre la corbeille', glyph: 'trash', unique: true },
  notifications: { type: 'notifications', label: 'Notifications', description: 'Centre de notifications (Win+N)', glyph: 'notifications', unique: true },
  settings: { type: 'settings', label: 'Réglages REZ', description: 'Panneau de configuration REZ', glyph: 'settings', unique: true },
};

export const DEFAULT_APPEARANCE: Appearance = {
  // Préréglage "ultra-discret" : barre fine (44 px), centrée en bas, masquée automatiquement.
  scale: 1,
  iconSize: 34,
  magnification: 1.65,
  magnifyRange: 2.8,
  spacing: 4,
  padding: 5,
  radius: 14,
  blur: 45,
  tint: '#14131f',
  opacity: 0.38,
  accent: '#8b5cff',
  glow: 0.6,
  showLabels: true,
  showIndicators: true,
  bounce: true,
};

export const DEFAULT_BEHAVIOR: Behavior = {
  position: 'bottom',
  freeX: 0.5,
  freeY: 0.85,
  autoHide: true,
  autoHideDelay: 600,
  peekLine: true,
  revealOnWinKey: true,
  hideTaskbar: true,
  reserveSpace: true,
  gamingMode: true,
  autostart: false,
  checkUpdates: true,
  bottomMargin: 6,
  clock24h: true,
  clockSeconds: false,
};

export function moduleItem(type: ModuleType, enabled = true): DockItem {
  return { id: `${type}-${uid()}`, type, enabled };
}

export function pinnedItem(info: AppInfo): DockItem {
  return {
    id: `app-${uid()}`,
    type: 'pinned',
    enabled: true,
    path: info.path,
    args: info.args ?? undefined,
    name: info.name,
    match: info.matchKey || undefined,
    icon: info.icon ?? undefined,
  };
}

export function defaultConfig(pins: AppInfo[]): RezConfig {
  return {
    version: CONFIG_VERSION,
    items: [
      moduleItem('start'),
      moduleItem('search'),
      moduleItem('taskview', false),
      moduleItem('separator'),
      ...pins.map(pinnedItem),
      moduleItem('running'),
      moduleItem('separator'),
      moduleItem('tray'),
      moduleItem('clock'),
      moduleItem('notifications', false),
      moduleItem('desktop', false),
      moduleItem('trash'),
      moduleItem('settings'),
    ],
    appearance: { ...DEFAULT_APPEARANCE },
    behavior: { ...DEFAULT_BEHAVIOR },
  };
}

/** Fusionne une config (éventuellement ancienne/partielle) avec les défauts. */
export function mergeConfig(raw: Partial<RezConfig> | null | undefined): RezConfig {
  const base = defaultConfig([]);
  if (!raw || typeof raw !== 'object') return base;
  const items = Array.isArray(raw.items)
    ? raw.items.filter((i): i is DockItem => !!i && typeof i.id === 'string' && i.type in MODULES)
    : base.items;
  return {
    version: CONFIG_VERSION,
    items,
    appearance: { ...DEFAULT_APPEARANCE, ...(raw.appearance ?? {}) },
    behavior: { ...DEFAULT_BEHAVIOR, ...(raw.behavior ?? {}) },
  };
}

/** Palette étendue (teintes de verre & accents). */
export const PALETTE: string[] = [
  '#0b0b12', '#14131f', '#1c1b2e', '#262540', '#2f2f3a', '#3d3d4d', '#6b6b80', '#a3a3b5',
  '#d9d9e3', '#ffffff', '#f5efe6', '#e8dcc8', '#3b0764', '#5b21b6', '#7c3aed', '#8b5cff',
  '#a78bfa', '#c4b5fd', '#1e3a8a', '#2563eb', '#3b82f6', '#60a5fa', '#0e7490', '#06b6d4',
  '#22d3ee', '#67e8f9', '#065f46', '#10b981', '#34d399', '#84cc16', '#a3e635', '#facc15',
  '#f59e0b', '#f97316', '#ef4444', '#e11d48', '#f43f5e', '#ec4899', '#f472d0', '#d946ef',
];

export interface ThemePreset {
  name: string;
  tint: string;
  opacity: number;
  accent: string;
  glow: number;
  blur: number;
}

export const THEMES: ThemePreset[] = [
  { name: 'Expo Glass', tint: '#14131f', opacity: 0.38, accent: '#8b5cff', glow: 0.6, blur: 45 },
  { name: 'Frost', tint: '#f4f6fb', opacity: 0.32, accent: '#3b82f6', glow: 0.45, blur: 48 },
  { name: 'Néon Violet', tint: '#1a0b2e', opacity: 0.5, accent: '#d946ef', glow: 0.95, blur: 45 },
  { name: 'Cyber Cyan', tint: '#04161c', opacity: 0.45, accent: '#22d3ee', glow: 0.9, blur: 45 },
  { name: 'Sunset', tint: '#2a1012', opacity: 0.42, accent: '#f97316', glow: 0.7, blur: 42 },
  { name: 'Emerald', tint: '#05170f', opacity: 0.42, accent: '#10b981', glow: 0.65, blur: 42 },
  { name: 'Sakura', tint: '#2b1220', opacity: 0.36, accent: '#f472d0', glow: 0.7, blur: 46 },
  { name: 'Mono Noir', tint: '#000000', opacity: 0.62, accent: '#e5e5e5', glow: 0.25, blur: 40 },
  { name: 'Clear', tint: '#ffffff', opacity: 0.08, accent: '#ffffff', glow: 0.5, blur: 48 },
];
