import { invoke } from '@tauri-apps/api/core';
import type { AppInfo, BgProcess, RezConfig, Surface, TrayApp, WinInfo } from './types';

export const api = {
  getConfig: () => invoke<RezConfig | null>('get_config'),
  saveConfig: (config: RezConfig, source: string, persist = true) =>
    invoke<void>('save_config', { config, source, persist }),

  getWindows: () => invoke<WinInfo[]>('get_windows'),
  activateWindow: (hwnd: number) => invoke<void>('activate_window', { hwnd }),
  focusWindow: (hwnd: number) => invoke<void>('focus_window', { hwnd }),
  minimizeWindow: (hwnd: number) => invoke<void>('minimize_window', { hwnd }),
  closeWindow: (hwnd: number) => invoke<void>('close_window', { hwnd }),

  launch: (path: string, args?: string | null) => invoke<void>('launch', { path, args: args ?? null }),
  openLocation: (path: string) => invoke<void>('open_location', { path }),
  shellAction: (action: string) => invoke<void>('shell_action', { action }),

  getIcon: (path: string, size = 128) => invoke<string | null>('get_icon', { path, size }),
  appInfo: (path: string) => invoke<AppInfo>('app_info', { path }),
  defaultPins: () => invoke<AppInfo[]>('default_pins'),

  trayApps: () => invoke<TrayApp[]>('get_tray_apps'),
  backgroundProcesses: () => invoke<BgProcess[]>('get_background_processes'),
  killProcesses: (pids: number[]) => invoke<number>('kill_processes', { pids }),

  setSurfaces: (glass: Surface[], hit: Surface[], reveal: Surface[] = []) => invoke<void>('set_surfaces', { glass, hit, reveal }),
  isGaming: () => invoke<boolean>('is_gaming'),
  openSettings: () => invoke<void>('open_settings'),
  prepareUpdate: () => invoke<void>('prepare_update'),
  quit: () => invoke<void>('quit_app'),
};

/** Cache d'icônes (une seule requête par chemin). */
const iconCache = new Map<string, Promise<string | null>>();
export function icon(path: string, size = 128): Promise<string | null> {
  const key = `${path.toLowerCase()}|${size}`;
  let p = iconCache.get(key);
  if (!p) {
    p = api.getIcon(path, size).catch(() => null);
    iconCache.set(key, p);
  }
  return p;
}

/** Cache des noms lisibles d'apps (FileDescription). */
const nameCache = new Map<string, Promise<string>>();
export function appName(exe: string): Promise<string> {
  const key = exe.toLowerCase();
  let p = nameCache.get(key);
  if (!p) {
    p = api
      .appInfo(exe)
      .then((i) => i.name)
      .catch(() => fileStem(exe));
    nameCache.set(key, p);
  }
  return p;
}

export function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

export function fileStem(path: string): string {
  return fileName(path).replace(/\.[^.]+$/, '');
}

export function uid(): string {
  return Math.random().toString(36).slice(2, 10) + Date.now().toString(36).slice(-4);
}
