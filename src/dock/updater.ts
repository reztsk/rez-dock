import { findUpdate, installUpdate, type Update } from '../shared/update';
import { anchor } from './layout';
import { closePopover, div, openPopover } from './popover';

/** Vérification discrète au lancement ; notification glass au-dessus du dock si une version existe. */
export async function checkAtStartup(): Promise<void> {
  const update = await findUpdate();
  if (update) showUpdate(update);
}

function showUpdate(update: Update): void {
  const root = div('update-root');
  root.style.display = 'contents';

  const head = div('update-head');
  head.innerHTML = '<img src="/rez-logo.png" alt="" />';
  const txt = div('');
  const title = document.createElement('b');
  title.textContent = 'Mise à jour disponible';
  const sub = document.createElement('span');
  sub.textContent = `REZ ${update.currentVersion} → ${update.version}`;
  txt.append(title, sub);
  head.appendChild(txt);
  root.appendChild(head);

  if (update.body?.trim()) {
    const notes = div('update-notes popover-scroll');
    notes.textContent = update.body.trim();
    root.appendChild(notes);
  }

  const progress = div('progress');
  progress.style.display = 'none';
  progress.appendChild(document.createElement('i'));
  root.appendChild(progress);

  const actions = div('update-actions');
  const later = document.createElement('button');
  later.className = 'btn';
  later.textContent = 'Plus tard';
  later.onclick = () => closePopover();
  const go = document.createElement('button');
  go.className = 'btn primary';
  go.textContent = 'Mettre à jour';
  go.onclick = async () => {
    go.disabled = true;
    later.disabled = true;
    go.textContent = 'Téléchargement…';
    progress.style.display = '';
    const bar = progress.firstElementChild as HTMLElement;
    try {
      await installUpdate(update, (ratio) => {
        progress.classList.toggle('indeterminate', ratio === null);
        if (ratio !== null) bar.style.width = `${Math.round(ratio * 100)}%`;
        if (ratio === 1) go.textContent = 'Installation…';
      });
    } catch (e) {
      console.error(e);
      go.disabled = false;
      later.disabled = false;
      go.textContent = 'Réessayer';
      sub.textContent = `Échec : ${String(e).slice(0, 80)}`;
    }
  };
  actions.append(later, go);
  root.appendChild(actions);

  openPopover(root, anchor(), { className: 'update', persistent: true, owner: 'update' });
}
