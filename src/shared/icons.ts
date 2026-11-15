/** Glyphes SVG (24×24, stroke = currentColor) des modules intégrés. */
const s = (body: string) =>
  `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${body}</svg>`;

export const GLYPHS = {
  /** Bouton Démarrer : 4 vitres minimalistes aux couleurs de Windows 11 (dégradé bleu). */
  windows:
    '<svg viewBox="0 0 24 24" aria-hidden="true"><defs><linearGradient id="rez-win" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#4cc2ff"/><stop offset="1" stop-color="#0067c0"/></linearGradient></defs><g fill="url(#rez-win)"><rect x="2.5" y="2.5" width="9" height="9" rx="1.2"/><rect x="12.5" y="2.5" width="9" height="9" rx="1.2"/><rect x="2.5" y="12.5" width="9" height="9" rx="1.2"/><rect x="12.5" y="12.5" width="9" height="9" rx="1.2"/></g></svg>',
  search: s('<circle cx="11" cy="11" r="6.5"/><path d="m20 20-4.2-4.2"/>'),
  taskview: s('<rect x="3" y="6" width="12" height="12" rx="2.5"/><path d="M18 8.5h1.5A1.5 1.5 0 0 1 21 10v8a3 3 0 0 1-3 3h-8a1.5 1.5 0 0 1-1.5-1.5V18"/>'),
  desktop: s('<rect x="3" y="4" width="18" height="12" rx="2.5"/><path d="M8 20h8M12 16v4"/>'),
  trash: s('<path d="M4 7h16M9.5 7V4.8c0-.4.4-.8.8-.8h3.4c.4 0 .8.4.8.8V7"/><path d="M6 7l1 12.2A2 2 0 0 0 9 21h6a2 2 0 0 0 2-1.8L18 7"/><path d="M10 11v6M14 11v6"/>'),
  notifications: s('<path d="M6 16V11a6 6 0 1 1 12 0v5l1.5 2h-15z"/><path d="M10 20.5a2 2 0 0 0 4 0"/>'),
  settings: s('<path d="M4 6h10M18 6h2M4 12h4M12 12h8M4 18h12M20 18h0"/><circle cx="16" cy="6" r="2"/><circle cx="10" cy="12" r="2"/><circle cx="18" cy="18" r="2"/>'),
  tray: s('<path d="m6 15 6-6 6 6"/>'),
  app: s('<rect x="4" y="4" width="16" height="16" rx="4"/><path d="M9 12h6M12 9v6"/>'),
  close: s('<path d="M6 6l12 12M18 6 6 18"/>'),
  pin: s('<path d="M15 4.5 19.5 9l-3 1.5-3.5 3.5.5 4-1.5 1.5-3-3L5 20.5 3.5 19l4-4-3-3L6 10.5l4 .5L13.5 7.5z"/>'),
  unpin: s('<path d="M15 4.5 19.5 9l-3 1.5-3.5 3.5.5 4-1.5 1.5-3-3L5 20.5 3.5 19l4-4-3-3L6 10.5l4 .5L13.5 7.5z"/><path d="M3 3l18 18"/>'),
  folder: s('<path d="M3 7.5A2.5 2.5 0 0 1 5.5 5H9l2 2h7.5A2.5 2.5 0 0 1 21 9.5v8a2.5 2.5 0 0 1-2.5 2.5h-13A2.5 2.5 0 0 1 3 17.5z"/>'),
  plus: s('<path d="M12 5v14M5 12h14"/>'),
  window: s('<rect x="3" y="5" width="18" height="14" rx="2.5"/><path d="M3 9h18"/>'),
  minimize: s('<path d="M6 12h12"/>'),
  kill: s('<circle cx="12" cy="12" r="8.5"/><path d="m9 9 6 6M15 9l-6 6"/>'),
  refresh: s('<path d="M20 11a8 8 0 1 0-2.3 5.7"/><path d="M20 4v7h-7"/>'),
  power: s('<path d="M12 3v8"/><path d="M6.3 6.8a8 8 0 1 0 11.4 0"/>'),
  eyeOff: s('<path d="M3 3l18 18"/><path d="M10.6 6.1A9.8 9.8 0 0 1 12 6c5 0 8.5 4.5 9.5 6a15 15 0 0 1-2.6 3.2M6.4 7.6A15.5 15.5 0 0 0 2.5 12c1 1.5 4.5 6 9.5 6a9.6 9.6 0 0 0 4.3-1"/>'),
  grip: s('<circle cx="9" cy="6" r="1"/><circle cx="15" cy="6" r="1"/><circle cx="9" cy="12" r="1"/><circle cx="15" cy="12" r="1"/><circle cx="9" cy="18" r="1"/><circle cx="15" cy="18" r="1"/>'),
  palette: s('<path d="M12 3a9 9 0 1 0 0 18c1.1 0 1.8-.9 1.5-1.9-.3-1 .4-2.1 1.5-2.1H17a4 4 0 0 0 4-4c0-5.5-4-10-9-10z"/><circle cx="7.5" cy="11" r="1.2"/><circle cx="10.5" cy="7" r="1.2"/><circle cx="15" cy="7.5" r="1.2"/>'),
  layers: s('<path d="m12 3 9 5-9 5-9-5z"/><path d="m3 13 9 5 9-5"/>'),
  sliders: s('<path d="M4 21v-7M4 10V3M12 21v-9M12 8V3M20 21v-5M20 12V3M1 14h6M9 8h6M17 16h6"/>'),
  download: s('<path d="M12 4v11M7 10l5 5 5-5M5 20h14"/>'),
  info: s('<circle cx="12" cy="12" r="9"/><path d="M12 11v5M12 8h0"/>'),
  shield: s('<path d="M12 3 4.5 6v6c0 4.5 3.2 8 7.5 9 4.3-1 7.5-4.5 7.5-9V6z"/><path d="m9 12 2 2 4-4"/>'),
  gamepad: s('<path d="M7 9h10a4 4 0 0 1 4 4v1.5a2.5 2.5 0 0 1-4.6 1.4L15 14H9l-1.4 1.9A2.5 2.5 0 0 1 3 14.5V13a4 4 0 0 1 4-4z"/><path d="M8 11.5v2M7 12.5h2M15.5 12h0M17.5 13h0"/>'),
} as const;

export type GlyphName = keyof typeof GLYPHS;
