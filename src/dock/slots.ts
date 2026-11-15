import { appName, fileName, fileStem } from '../shared/api';
import { MODULES } from '../shared/defaults';
import type { DockItem, RezConfig, WinInfo } from '../shared/types';

export type SlotKind = 'app' | 'module' | 'separator' | 'clock';

export interface Slot {
  key: string;
  item: DockItem;
  kind: SlotKind;
  label: string;
  iconPath?: string;
  windows: WinInfo[];
  exe?: string;
  launchPath?: string;
  launchArgs?: string;
  pinned: boolean;
  draggable: boolean;
  scalable: boolean;
}

/** Ordre stable des groupes "apps ouvertes" (ordre d'apparition). */
const runOrder: string[] = [];
const labels = new Map<string, string>();
let onLabel: (() => void) | null = null;

export function setLabelListener(fn: () => void): void {
  onLabel = fn;
}

function prettyStem(exe: string): string {
  const s = fileStem(exe);
  return s ? s.charAt(0).toUpperCase() + s.slice(1) : 'Application';
}

function labelFor(key: string, exe: string): string {
  const known = labels.get(key);
  if (known) return known;
  if (exe) {
    labels.set(key, prettyStem(exe));
    void appName(exe).then((name) => {
      if (name && name !== labels.get(key)) {
        labels.set(key, name);
        onLabel?.();
      }
    });
    return prettyStem(exe);
  }
  return 'Application';
}

export function groupKey(w: WinInfo): string {
  return fileName(w.exe).toLowerCase() || `hwnd:${w.hwnd}`;
}

export function buildSlots(cfg: RezConfig, windows: WinInfo[]): Slot[] {
  const groups = new Map<string, WinInfo[]>();
  for (const w of windows) {
    const k = groupKey(w);
    const list = groups.get(k);
    if (list) list.push(w);
    else groups.set(k, [w]);
  }

  const claimed = new Set<string>();
  for (const it of cfg.items) {
    if (it.enabled && it.type === 'pinned' && it.match) claimed.add(it.match.toLowerCase());
  }

  for (const k of groups.keys()) if (!runOrder.includes(k)) runOrder.push(k);
  for (let i = runOrder.length - 1; i >= 0; i--) if (!groups.has(runOrder[i])) runOrder.splice(i, 1);

  const slots: Slot[] = [];
  for (const item of cfg.items) {
    if (!item.enabled) continue;
    switch (item.type) {
      case 'pinned': {
        const wins = item.match ? (groups.get(item.match.toLowerCase()) ?? []) : [];
        slots.push({
          key: item.id,
          item,
          kind: 'app',
          label: item.name || (item.path ? fileStem(item.path) : 'Application'),
          iconPath: item.icon || item.path,
          windows: wins,
          exe: wins[0]?.exe,
          launchPath: item.path,
          launchArgs: item.args,
          pinned: true,
          draggable: true,
          scalable: true,
        });
        break;
      }
      case 'running': {
        for (const k of runOrder) {
          if (claimed.has(k)) continue;
          const wins = groups.get(k);
          if (!wins?.length) continue;
          const exe = wins[0].exe;
          slots.push({
            key: `run:${k}`,
            item,
            kind: 'app',
            label: labelFor(k, exe),
            iconPath: exe || undefined,
            windows: wins,
            exe,
            launchPath: exe || undefined,
            pinned: false,
            draggable: false,
            scalable: true,
          });
        }
        break;
      }
      case 'separator':
        slots.push(base(item, 'separator', '', false));
        break;
      case 'clock':
        slots.push(base(item, 'clock', MODULES.clock.label, false));
        break;
      default:
        slots.push(base(item, 'module', MODULES[item.type].label, true));
    }
  }
  return slots;
}

function base(item: DockItem, kind: SlotKind, label: string, scalable: boolean): Slot {
  return { key: item.id, item, kind, label, windows: [], pinned: false, draggable: true, scalable };
}
