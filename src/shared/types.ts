export type ModuleType =
  | 'start'
  | 'search'
  | 'taskview'
  | 'pinned'
  | 'running'
  | 'separator'
  | 'tray'
  | 'clock'
  | 'desktop'
  | 'trash'
  | 'notifications'
  | 'settings';

export interface DockItem {
  id: string;
  type: ModuleType;
  enabled: boolean;
  /** pinned : fichier à lancer (.exe, .lnk, .url, shell:…) */
  path?: string;
  args?: string;
  name?: string;
  /** pinned : nom d'exe (minuscule) servant à associer les fenêtres ouvertes */
  match?: string;
  /** pinned : icône personnalisée (.ico/.png/.exe) */
  icon?: string;
}

export interface Appearance {
  /** Échelle globale du dock (0.5 = moitié, 2 = double) */
  scale: number;
  iconSize: number;
  magnification: number;
  magnifyRange: number;
  spacing: number;
  padding: number;
  radius: number;
  blur: number;
  tint: string;
  opacity: number;
  accent: string;
  glow: number;
  showLabels: boolean;
  showIndicators: boolean;
  bounce: boolean;
}

export type DockEdge = 'bottom' | 'top' | 'left' | 'right' | 'free';

export interface Behavior {
  position: DockEdge;
  /** Mode libre : centre du dock en fraction de l'écran (0..1) */
  freeX: number;
  freeY: number;
  /** Masquage auto façon macOS : le dock glisse hors de l'écran, un petit trait reste visible */
  autoHide: boolean;
  /** Délai (ms) avant masquage quand la souris quitte le dock */
  autoHideDelay: number;
  /** Petit trait discret affiché au bord quand le dock est masqué */
  peekLine: boolean;
  /** Touche Windows / menu Démarrer ouvert → le dock remonte */
  revealOnWinKey: boolean;
  /** Un dock sur chaque écran connecté (barre Windows masquée partout) */
  allMonitors: boolean;
  /** Langue de l'interface */
  language: 'auto' | 'fr' | 'en';
  hideTaskbar: boolean;
  reserveSpace: boolean;
  gamingMode: boolean;
  autostart: boolean;
  checkUpdates: boolean;
  bottomMargin: number;
  clock24h: boolean;
  clockSeconds: boolean;
}

export interface RezConfig {
  version: number;
  items: DockItem[];
  appearance: Appearance;
  behavior: Behavior;
}

export interface WinInfo {
  hwnd: number;
  title: string;
  exe: string;
  pid: number;
  minimized: boolean;
  active: boolean;
}

export interface AppInfo {
  path: string;
  args: string | null;
  name: string;
  target: string;
  matchKey: string;
  icon: string | null;
}

export interface TrayApp {
  name: string;
  exePath: string;
  exeName: string;
  icon: string | null;
  tooltip: string;
  pids: number[];
  promoted: boolean;
}

export interface BgProcess {
  name: string;
  path: string;
  pids: number[];
}

export interface Surface {
  x: number;
  y: number;
  w: number;
  h: number;
  r: number;
}

export interface ConfigChanged {
  config: RezConfig;
  source: string;
}
