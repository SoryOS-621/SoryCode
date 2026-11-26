# apps/ui — design system

Bibliothèque de composants React de SoryCode. Elle ne contient **aucune**
logique métier, aucun accès au Core et aucun import de `apps/app` ou
`apps/desktop`.

- `src/primitives/` : `Stack`, `Text`, `Surface`, `Badge`, `Kbd`, `Spinner`,
  `Tooltip`, `Divider` ;
- `src/components/` : boutons, champs, dialogues, menus, palette de commandes,
  panneaux, blocs de code/diff, navigation ;
- `src/layouts/` : `AppShell`, `Sidebar`, `AppHeader`, `AppContent`,
  `SplitLayout` ;
- `src/themes/` : `ThemeProvider` (`light` / `dark` / `system`) ;
- `src/hooks/` : `useMediaQuery`, `useHotkeys`, `useControllableState` ;
- `src/icons/` : icônes SVG typées (`IconName`) ;
- `src/styles/` : tokens, base et composants CSS.

Consommation depuis `apps/app` :

```ts
import { Button, Panel, useTheme } from "@sorycode/ui";
import "@sorycode/ui/styles.css";
```

Le package est consommé en source TypeScript (`exports` vers `src/index.ts`),
ce qui permet à Vite de le bundler sans étape de build intermédiaire.

```bash
npm run typecheck
```
