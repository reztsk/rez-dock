/**
 * Logos vectoriels intégrés pour les apps épinglées par défaut (Chrome, Explorateur).
 * Toujours nets, quelle que soit la taille du dock, et jamais remplacés par une lettre
 * si l'extraction d'icône Windows échoue.
 */
import { fileName } from './api';

/** Google Chrome : trois secteurs rouge / jaune / vert autour du disque bleu. */
const CHROME = `<svg viewBox="0 0 48 48" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
  <path fill="#EA4335" d="M24 14H41.32A20 20 0 0 0 6.68 14L15.34 29A10 10 0 0 1 24 14Z"/>
  <path fill="#FBBC04" d="M32.66 29L24 44A20 20 0 0 0 41.32 14H24A10 10 0 0 1 32.66 29Z"/>
  <path fill="#34A853" d="M15.34 29L6.68 14A20 20 0 0 0 24 44L32.66 29A10 10 0 0 1 15.34 29Z"/>
  <circle cx="24" cy="24" r="10" fill="#FFFFFF"/>
  <circle cx="24" cy="24" r="8" fill="#1A73E8"/>
</svg>`;

/** Explorateur de fichiers : dossier jaune, onglet arrière plus foncé, liseré bleu. */
const EXPLORER = `<svg viewBox="0 0 48 48" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
  <defs>
    <linearGradient id="rzExpBack" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#F0A81C"/>
      <stop offset="1" stop-color="#D98A06"/>
    </linearGradient>
    <linearGradient id="rzExpFront" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#FFE38A"/>
      <stop offset="1" stop-color="#FFC531"/>
    </linearGradient>
  </defs>
  <path fill="url(#rzExpBack)" d="M5 13a4 4 0 0 1 4-4h9.17a4 4 0 0 1 2.83 1.17L23.83 13H39a4 4 0 0 1 4 4v18a4 4 0 0 1-4 4H9a4 4 0 0 1-4-4Z"/>
  <path fill="url(#rzExpFront)" d="M5 21a4 4 0 0 1 4-4h30a4 4 0 0 1 4 4v14a4 4 0 0 1-4 4H9a4 4 0 0 1-4-4Z"/>
  <path fill="#FFFFFF" fill-opacity="0.45" d="M9 17h30a4 4 0 0 1 4 4v.6a4 4 0 0 0-4-3.1H9a4 4 0 0 0-4 3.1V21a4 4 0 0 1 4-4Z"/>
  <rect x="10" y="32" width="11" height="3" rx="1.5" fill="#1A73E8"/>
</svg>`;

const BY_FILE: Record<string, string> = {
  'chrome.exe': CHROME,
  'google chrome.lnk': CHROME,
  'explorer.exe': EXPLORER,
  'file explorer.lnk': EXPLORER,
  'explorateur de fichiers.lnk': EXPLORER,
};

/** SVG intégré pour ce chemin (exe ou raccourci), sinon `null`. */
export function brandIcon(path?: string): string | null {
  if (!path) return null;
  return BY_FILE[fileName(path).toLowerCase()] ?? null;
}
