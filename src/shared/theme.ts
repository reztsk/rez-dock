import type { Appearance, RezConfig } from './types';

export function hexToRgb(hex: string): [number, number, number] {
  let h = hex.trim().replace('#', '');
  if (h.length === 3) h = h.split('').map((c) => c + c).join('');
  const n = parseInt(h.slice(0, 6), 16);
  if (Number.isNaN(n)) return [20, 19, 31];
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

export function luminance([r, g, b]: [number, number, number]): number {
  const f = (c: number) => {
    const v = c / 255;
    return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b);
}

export function isValidHex(v: string): boolean {
  return /^#?([0-9a-f]{3}|[0-9a-f]{6})$/i.test(v.trim());
}

export function normalizeHex(v: string): string {
  let h = v.trim().replace('#', '').toLowerCase();
  if (h.length === 3) h = h.split('').map((c) => c + c).join('');
  return `#${h}`;
}

/** Dimensions effectives (échelle globale appliquée). */
export function eff(a: Appearance): { icon: number; gap: number; pad: number; radius: number } {
  const k = Math.min(3, Math.max(0.4, a.scale || 1));
  return { icon: a.iconSize * k, gap: a.spacing * k, pad: a.padding * k, radius: a.radius * k };
}

/** Applique l'apparence en variables CSS (instantané, sans re-render). */
export function applyAppearance(cfg: RezConfig, root: HTMLElement = document.documentElement): void {
  const a = cfg.appearance;
  const e = eff(a);
  const tint = hexToRgb(a.tint);
  const accent = hexToRgb(a.accent);
  // Contraste auto du texte selon la teinte perçue (teinte mélangée au fond supposé sombre).
  const light = luminance(tint) * Math.min(1, a.opacity * 1.6) > 0.35;
  const st = root.style;
  st.setProperty('--icon', `${e.icon}px`);
  st.setProperty('--gap', `${e.gap}px`);
  st.setProperty('--pad', `${e.pad}px`);
  st.setProperty('--radius', `${e.radius}px`);
  st.setProperty('--tint-rgb', tint.join(','));
  st.setProperty('--opacity', String(a.opacity));
  st.setProperty('--accent-rgb', accent.join(','));
  st.setProperty('--accent', a.accent);
  st.setProperty('--glow', String(a.glow));
  st.setProperty('--margin', `${cfg.behavior.bottomMargin}px`);
  st.setProperty('--css-blur', `${Math.round(a.blur * 0.4)}px`);
  st.setProperty('--fg', light ? '20,20,28' : '255,255,255');
  root.classList.toggle('native-blur-off', a.blur <= 0.5);
  root.classList.toggle('light-glass', light);
  root.classList.toggle('no-indicators', !a.showIndicators);
  root.classList.toggle('no-labels', !a.showLabels);
}
