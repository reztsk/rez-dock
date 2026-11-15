import { api } from '../shared/api';
import type { DockItem, RezConfig, WinInfo } from '../shared/types';

/** État du dock (source de vérité côté fenêtre dock). */
export const store = {
  config: null as unknown as RezConfig,
  windows: [] as WinInfo[],
  hidden: true,
  /** Masquage automatique en cours (dock replié, petit trait visible) */
  autoHidden: false,
  dragging: false,
};

const listeners = new Set<() => void>();

export function onChange(fn: () => void): void {
  listeners.add(fn);
}

export function notify(): void {
  listeners.forEach((fn) => fn());
}

/** Modifie la config, persiste et redessine. */
export function updateConfig(mutate: (c: RezConfig) => void): void {
  const next: RezConfig = structuredClone(store.config);
  mutate(next);
  store.config = next;
  void api.saveConfig(next, 'dock', true);
  notify();
}

export function findItem(id: string): DockItem | undefined {
  return store.config.items.find((i) => i.id === id);
}

/** Position d'insertion par défaut d'une nouvelle app : juste avant le module "Apps ouvertes". */
export function defaultInsertIndex(items: DockItem[]): number {
  const run = items.findIndex((i) => i.type === 'running');
  if (run >= 0) return run;
  const tray = items.findIndex((i) => i.type === 'tray');
  return tray >= 0 ? tray : items.length;
}
