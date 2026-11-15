/**
 * Traduction FR / EN. Le français est la langue source : les chaînes françaises du code servent
 * de clés. `autoTranslate(root)` traduit un sous-arbre DOM (textes + title/placeholder) et
 * continue de traduire tout ce qui y est ajouté ensuite (menus, popovers, listes chargées).
 * `t()` sert pour les chaînes interpolées ou hors DOM (tooltips, dialogues).
 */
export type LangPref = 'auto' | 'fr' | 'en';
export type Lang = 'fr' | 'en';

let current: Lang = 'fr';

export function resolveLang(pref: LangPref | undefined): Lang {
  if (pref === 'fr' || pref === 'en') return pref;
  return (navigator.language || 'fr').toLowerCase().startsWith('fr') ? 'fr' : 'en';
}

/** Applique la langue ; retourne true si elle a changé. */
export function setLang(pref: LangPref | undefined): boolean {
  const next = resolveLang(pref);
  const changed = next !== current;
  current = next;
  document.documentElement.lang = next;
  return changed;
}

export function lang(): Lang {
  return current;
}

export function t(fr: string, vars?: Record<string, string | number>): string {
  let s = current === 'en' ? (EN[fr] ?? fr) : fr;
  if (vars) for (const [k, v] of Object.entries(vars)) s = s.split(`{${k}}`).join(String(v));
  return s;
}

const ATTRS = ['title', 'placeholder', 'aria-label'];

function translateNode(n: Node): void {
  if (n.nodeType === Node.TEXT_NODE) {
    const raw = n.nodeValue ?? '';
    const key = raw.trim();
    const en = key && EN[key];
    if (en && en !== key) n.nodeValue = raw.replace(key, en);
  } else if (n instanceof Element) {
    for (const a of ATTRS) {
      const v = n.getAttribute(a);
      const en = v && EN[v];
      if (en && en !== v) n.setAttribute(a, en);
    }
  }
}

export function translateTree(root: Node): void {
  if (current === 'fr') return;
  translateNode(root);
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT | NodeFilter.SHOW_ELEMENT);
  for (let n = walker.nextNode(); n; n = walker.nextNode()) translateNode(n);
}

/** Traduit `root` maintenant et tout ce qui y sera ajouté/modifié ensuite. */
export function autoTranslate(root: Node): void {
  translateTree(root);
  new MutationObserver((muts) => {
    if (current === 'fr') return;
    for (const m of muts) {
      if (m.type === 'childList') m.addedNodes.forEach(translateTree);
      else translateNode(m.target);
    }
  }).observe(root, { subtree: true, childList: true, characterData: true, attributes: true, attributeFilter: ATTRS });
}

/** Français → anglais. */
const EN: Record<string, string> = {
  // --- Réglages : navigation
  Apparence: 'Appearance',
  'Éléments du dock': 'Dock items',
  Comportement: 'Behavior',
  'Mises à jour': 'Updates',
  'À propos': 'About',
  Réglages: 'Settings',
  'REZ Dock — Réglages': 'REZ Dock — Settings',
  Réduire: 'Minimize',
  Fermer: 'Close',
  Langue: 'Language',
  Automatique: 'Automatic',
  'Profil GitHub de REZ': 'REZ GitHub profile',

  // --- Apparence
  'Expo-Glass : verre flouté natif, teinte, lueur et animations. Tout s’applique en direct sur le dock.':
    'Expo-Glass: native blurred glass, tint, glow and animations. Everything applies live to the dock.',
  'Position & taille': 'Position & size',
  Bas: 'Bottom',
  Haut: 'Top',
  Gauche: 'Left',
  Droite: 'Right',
  Libre: 'Free',
  Emplacement: 'Placement',
  'Astuce : glissez le fond du dock pour le déplacer ; relâché près d’un bord, il s’y ancre.':
    'Tip: drag the dock background to move it; release it near an edge to snap it there.',
  'Échelle globale': 'Overall scale',
  'Agrandit ou réduit tout le dock (icônes, marges, arrondis)': 'Enlarges or shrinks the whole dock (icons, margins, corners)',
  'Position horizontale': 'Horizontal position',
  'Position verticale': 'Vertical position',
  'Préréglage compact': 'Compact preset',
  'Revenir au format discret par défaut, centré en bas': 'Go back to the discreet default format, centered at the bottom',
  Appliquer: 'Apply',
  Thèmes: 'Themes',
  Verre: 'Glass',
  'Teinte du verre': 'Glass tint',
  'Opacité de la teinte': 'Tint opacity',
  'Transparence de la couleur posée sur le flou': 'Transparency of the color laid over the blur',
  'Intensité du flou': 'Blur intensity',
  '0 = désactivé · 1–49 = flou DWM · 50–100 = acrylique (flou intense + grain)': '0 = off · 1–49 = DWM blur · 50–100 = acrylic (strong blur + grain)',
  'Lueur des bordures': 'Border glow',
  'Reflets et halo néon autour du dock': 'Reflections and neon halo around the dock',
  'Rayon des arrondis': 'Corner radius',
  'Couleur d’accent': 'Accent color',
  'Indicateurs, halos, modules intégrés': 'Indicators, halos, built-in modules',
  'Icônes & magnification': 'Icons & magnification',
  'Taille des icônes': 'Icon size',
  'Agrandissement maximal au survol (1× = désactivé)': 'Maximum enlargement on hover (1× = off)',
  'Largeur de la vague': 'Wave width',
  'Nombre d’icônes voisines affectées': 'Number of neighboring icons affected',
  Espacement: 'Spacing',
  'Marge intérieure': 'Inner padding',
  'Distance au bord de l’écran': 'Distance from screen edge',
  'Espace flottant sous le dock': 'Floating space below the dock',
  'Noms au survol': 'Names on hover',
  'Indicateurs d’apps ouvertes': 'Open app indicators',
  'Rebond au lancement': 'Bounce on launch',
  Flou: 'Blur',
  'Néon Violet': 'Neon Violet',

  // --- Éléments
  'Glissez pour réorganiser, activez/désactivez, renommez (double-clic) ou ajoutez des applications et modules. Astuce : déposez un .exe/.lnk directement sur le dock.':
    'Drag to reorder, enable/disable, rename (double-click) or add apps and modules. Tip: drop an .exe/.lnk straight onto the dock.',
  'Ajouter une application': 'Add an application',
  '+ Ajouter un module…': '+ Add a module…',
  'Tous les modules sont présents': 'All modules are present',
  Réinitialiser: 'Reset',
  'Réinitialiser l’agencement du dock (les réglages d’apparence sont conservés) ?': 'Reset the dock layout (appearance settings are kept)?',
  'Changer l’icône': 'Change icon',
  Lancer: 'Launch',
  Retirer: 'Remove',
  'Ajouter des applications au dock': 'Add applications to the dock',
  'Applications et raccourcis': 'Applications and shortcuts',
  'Choisir une icône': 'Choose an icon',
  Icônes: 'Icons',

  // --- Comportement
  'Remplacement de la barre des tâches, mode gaming et démarrage.': 'Taskbar replacement, gaming mode and startup.',
  'Masquage automatique': 'Auto-hide',
  'Mode du dock': 'Dock mode',
  'Masquage auto': 'Auto-hide',
  Épinglé: 'Pinned',
  'Style macOS : le dock se replie quand la souris s’en va et remonte dès qu’elle touche le bord':
    'macOS style: the dock tucks away when the mouse leaves and comes back as soon as it touches the edge',
  'Épinglé : le dock reste toujours visible': 'Pinned: the dock always stays visible',
  'Indisponible en placement libre (ancre le dock sur un bord)': 'Unavailable in free placement (snap the dock to an edge)',
  'Afficher le trait discret': 'Show the discreet line',
  'Petit trait lumineux au bord de l’écran quand le dock est replié': 'Small glowing line at the screen edge when the dock is tucked away',
  'Délai avant masquage': 'Delay before hiding',
  'Touche Windows': 'Windows key',
  'Appuyer sur Win (ou ouvrir le menu Démarrer) fait remonter les docks de tous les écrans':
    'Pressing Win (or opening the Start menu) brings up the docks on every screen',
  Écrans: 'Screens',
  'Dock sur tous les écrans': 'Dock on every screen',
  'Un dock sur chaque écran connecté ; la barre Windows est masquée partout': 'A dock on every connected screen; the Windows taskbar is hidden everywhere',
  'Barre des tâches': 'Taskbar',
  'Masquer la barre des tâches Windows': 'Hide the Windows taskbar',
  'REZ la remplace sur tous les écrans. Elle est restaurée automatiquement en quittant REZ (même après un crash, au lancement suivant).':
    'REZ replaces it on every screen. It is restored automatically when REZ quits (even after a crash, on next launch).',
  'Réserver l’espace du dock': 'Reserve dock space',
  'Sans effet avec le masquage automatique (les fenêtres maximisées prennent tout l’écran)': 'No effect with auto-hide (maximized windows take the full screen)',
  'Les fenêtres maximisées s’arrêtent au-dessus du dock': 'Maximized windows stop above the dock',
  'Mode gaming sécurisé': 'Safe gaming mode',
  'Retrait automatique en plein écran': 'Auto-retract in full screen',
  'Jeux, vidéos et présentations plein écran (exclusif ou fenêtré sans bordure)': 'Full-screen games, videos and presentations (exclusive or borderless windowed)',
  Horloge: 'Clock',
  'Format 24 heures': '24-hour format',
  'Afficher les secondes': 'Show seconds',
  Système: 'System',
  'Lancer REZ au démarrage de Windows': 'Launch REZ when Windows starts',
  'Quitter REZ': 'Quit REZ',
  'Ferme le dock et restaure la barre des tâches Windows': 'Closes the dock and restores the Windows taskbar',
  Quitter: 'Quit',

  // --- Mises à jour
  'REZ vérifie discrètement les releases GitHub au lancement. Installation en un clic, signature vérifiée.':
    'REZ quietly checks GitHub releases at startup. One-click install, verified signature.',
  'Cliquez sur « Rechercher » pour vérifier.': 'Click “Check” to look for updates.',
  'Mettre à jour': 'Update',
  'Téléchargement…': 'Downloading…',
  'Installation puis redémarrage…': 'Installing, then restarting…',
  Rechercher: 'Check',
  'Recherche en cours…': 'Checking…',
  'Nouvelle version disponible : v{v}': 'New version available: v{v}',
  'À jour': 'Up to date',
  'Vous utilisez la dernière version.': 'You are using the latest version.',
  'Version installée': 'Installed version',
  État: 'Status',
  'Vérifier au lancement': 'Check at startup',
  'Recherche silencieuse 6 s après le démarrage': 'Silent check 6 s after startup',
  'Releases GitHub': 'GitHub releases',
  Ouvrir: 'Open',
  'Échec : {e}': 'Failed: {e}',
  'Mise à jour disponible': 'Update available',
  'Plus tard': 'Later',
  'Installation…': 'Installing…',
  Réessayer: 'Retry',

  // --- À propos
  'Dock Expo-Glass pour Windows': 'Expo-Glass dock for Windows',
  'Dock Expo-Glass pour Windows · v{v}': 'Expo-Glass dock for Windows · v{v}',
  Technologie: 'Technology',

  // --- Modules
  Démarrer: 'Start',
  Recherche: 'Search',
  'Vue des tâches': 'Task view',
  'Apps ouvertes': 'Open apps',
  Séparateur: 'Separator',
  'Icônes cachées': 'Hidden icons',
  Bureau: 'Desktop',
  Corbeille: 'Recycle Bin',
  'Réglages REZ': 'REZ settings',
  'Ouvre le menu Démarrer': 'Opens the Start menu',
  'Recherche Windows (Win+S)': 'Windows Search (Win+S)',
  'Bureaux virtuels (Win+Tab)': 'Virtual desktops (Win+Tab)',
  'Application épinglée': 'Pinned application',
  'Fenêtres ouvertes non épinglées': 'Open windows that are not pinned',
  'Trait vertical': 'Vertical line',
  'Zone de notification + apps en arrière-plan': 'Notification area + background apps',
  'Heure et date (clic : calendrier)': 'Time and date (click: calendar)',
  'Afficher le bureau (Win+D)': 'Show desktop (Win+D)',
  'Ouvre la corbeille': 'Opens the Recycle Bin',
  'Centre de notifications (Win+N)': 'Notification center (Win+N)',
  'Panneau de configuration REZ': 'REZ control panel',

  // --- Dock : menus, fenêtres, zone de notification
  Fenêtre: 'Window',
  réduite: 'minimized',
  'Nouvelle fenêtre': 'New window',
  'Retirer du dock': 'Remove from dock',
  'Épingler au dock': 'Pin to dock',
  "Ouvrir l'emplacement": 'Open file location',
  'Fermer les {n} fenêtres': 'Close {n} windows',
  'Fermer la fenêtre': 'Close window',
  'Masquer ce module': 'Hide this module',
  'Réglages REZ…': 'REZ settings…',
  'Gestionnaire des tâches': 'Task Manager',
  'Épingler le dock (toujours visible)': 'Pin the dock (always visible)',
  'Masquer automatiquement le dock': 'Auto-hide the dock',
  'Arrière-plan': 'Background',
  Actualiser: 'Refresh',
  'Paramètres des icônes Windows': 'Windows icon settings',
  'Aucune application dans la zone de notification.': 'No apps in the notification area.',
  'Aucun processus en arrière-plan.': 'No background processes.',
  'Forcer la fermeture': 'Force close',
  Terminer: 'End',
  'Confirmer ?': 'Confirm?',
};
