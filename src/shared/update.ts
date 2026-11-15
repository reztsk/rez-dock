import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { api } from './api';

export type { Update };

/** Vérifie les releases GitHub (latest.json). Retourne null si à jour ou hors-ligne. */
export async function findUpdate(): Promise<Update | null> {
  try {
    return await check({ timeout: 15000 });
  } catch (e) {
    console.warn('[REZ] vérification des mises à jour impossible :', e);
    return null;
  }
}

/**
 * Télécharge (signature vérifiée par le plugin), restaure la barre Windows,
 * lance l'installeur NSIS en mode passif puis redémarre REZ.
 */
export async function installUpdate(update: Update, onProgress: (ratio: number | null) => void): Promise<void> {
  let total = 0;
  let done = 0;
  await update.download((ev) => {
    if (ev.event === 'Started') {
      total = ev.data.contentLength ?? 0;
      onProgress(total ? 0 : null);
    } else if (ev.event === 'Progress') {
      done += ev.data.chunkLength;
      onProgress(total ? Math.min(1, done / total) : null);
    } else if (ev.event === 'Finished') {
      onProgress(1);
    }
  });
  await api.prepareUpdate();
  await update.install();
  await relaunch();
}
