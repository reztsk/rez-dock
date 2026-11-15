# REZ — Identité visuelle

**Fichier source** : `rez-icon.svg` (vectoriel, 1024×1024). Toutes les icônes de l'app sont générées depuis lui (`npm run icons`).

## Concept

- **Plaque de verre** : squircle sombre (`#1B1838 → #07070F`), reflet supérieur, liseré lumineux blanc→violet. Même langage que le dock Expo-Glass.
- **Monogramme « R »** : un seul trait géométrique (fût + panse en demi-cercle + jambe diagonale), terminaisons arrondies. Lisible à 16 px.
- **Néon signature** : dégradé violet `#8B5CFF` → cyan `#22D3EE` → rose `#F472D0`, doublé d'un halo flou (bloom) et d'un filet blanc spéculaire sur la panse.
- **Barre « dock »** : trait lumineux sous le R + trois points = les icônes du dock. Le logo dit ce que fait l'app.

## Palette

| Rôle | Hex |
|---|---|
| Violet néon (accent par défaut) | `#8B5CFF` |
| Cyan | `#22D3EE` |
| Rose | `#F472D0` |
| Verre profond | `#0E0D1E` |
| Verre clair | `#1B1838` |

## Déclinaisons

- **Wordmark** : « REZ » en Segoe UI Variable Display Black, interlettrage +0.14em, remplissage du dégradé néon.
- **Mono** : R blanc sur fond transparent (zones claires/sombres).
