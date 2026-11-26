# apps/desktop — coquille native Tauri

Cette application contient **uniquement** le backend natif Tauri et son
intégration système. Elle ne contient aucun composant React.

- `src-tauri/src/lib.rs` : builder Tauri, enregistrement des commandes IPC.
- `src-tauri/src/state.rs` : `EngineState` (provider, modèle, workspace).
- `src-tauri/src/commands/` : providers, sessions, chat, projets, système.
- `src-tauri/src/system/` : chemins, settings, inspection confinée du workspace.
- `src-tauri/src/ipc/` : DTO camelCase exposés au frontend.
- `src-tauri/tauri.conf.json` : fenêtre, `frontendDist: ../../app/dist`.

Le frontend chargé est le build de `apps/app`. Le design system vit dans
`apps/ui`.

```bash
npm run dev          # tauri dev (lance aussi apps/app)
npm run build        # tauri build
npm run typecheck    # cargo check du crate Tauri
```

Le crate est volontairement exclu du workspace Cargo racine
(`exclude = ["apps/desktop/src-tauri"]`) pour que le build du Core/CLI ne
dépende pas des bibliothèques webview.

Dépendances système : `webkit2gtk-4.1`, `gtk+-3.0`, `javascriptcoregtk-4.1`
(plus les paquets de bundling `deb`/`appimage`).
