# Rapport — restructuration de `apps/`

## Objectif

Séparer l'ancien frontend (backend Tauri dans `apps/app`, application React
complète dans `apps/ui`) en trois applications strictement indépendantes :

- `apps/desktop` : coquille native Tauri (Rust uniquement) ;
- `apps/ui` : design system React sans logique métier ;
- `apps/app` : application React métier.

Le moteur Rust (`crates/`) reste indépendant de React et de Tauri.

## Résultat

```
apps/
  desktop/                 coquille native
    README.md
    package.json
    src-tauri/
      Cargo.toml
      build.rs
      tauri.conf.json
      capabilities/default.json
      icons/{32x32,128x128,128x128@2x}.png
      src/
        main.rs, lib.rs, state.rs, events.rs
        ipc/{mod.rs,dto.rs}
        commands/{mod.rs,providers.rs,sessions.rs,chat.rs,projects.rs,system.rs}
        system/{mod.rs,paths.rs,settings.rs,workspace.rs}
  ui/                      design system
    README.md, package.json, tsconfig.json
    src/
      index.ts
      icons/{Icon.tsx,index.ts}
      primitives/{Stack,Text,Surface,Feedback}.tsx
      components/buttons/{Button,IconButton}.tsx
      components/inputs/Inputs.tsx
      components/dialogs/Dialog.tsx
      components/menus/{DropdownMenu,CommandMenu}.tsx
      components/panels/Panel.tsx
      components/editors/CodeBlock.tsx
      components/navigation/{SidebarNav,Tabs}.tsx
      layouts/AppShell.tsx
      themes/ThemeProvider.tsx
      hooks/{useMediaQuery,useControllableState,useHotkeys,useTheme}.ts
      styles/{tokens,base,components,index}.css
  app/                     application métier
    README.md, package.json, tsconfig.json, vite.config.ts, index.html
    src/
      app/{main.tsx,App.tsx}
      app/providers/{AppProviders,EngineProvider}.tsx
      features/chat/{ChatView,Composer,MessageList,useChat}
      features/code/{CodeView,useWorkspaceFiles}
      features/work/{WorkView,useTasks}
      features/projects/ProjectsView.tsx
      features/workspaces/WorkspacesView.tsx
      features/sessions/SessionsView.tsx
      features/agents/AgentsView.tsx
      features/settings/SettingsView.tsx
      pages/*.tsx
      routes/{index.ts,useRoute.ts}
      services/{coreClient,tauriClient,mockClient,index}.ts
      stores/{createStore,sessionStore,uiStore}.ts
      styles/app.css
packages/
  types/                   types domaine partagés
  contracts/               contrats wire IPC + événements
  config/                  tsconfig de base
```

## Fichiers créés

### Racine

- `package.json` : workspaces npm et scripts (`dev`, `build`, `typecheck`,
  `desktop:dev`, `desktop:build`, `core:build`).
- `.gitignore` : `node_modules`, `dist`, `target`, credentials, logs.

### `apps/desktop`

- `package.json`, `README.md` ;
- `src-tauri/Cargo.toml` (crate autonome, hors workspace racine) ;
- `src-tauri/build.rs`, `main.rs`, `lib.rs`, `state.rs`, `events.rs` ;
- `src-tauri/ipc/{mod.rs,dto.rs}` ;
- `src-tauri/commands/{mod.rs,providers.rs,sessions.rs,chat.rs,projects.rs,system.rs}` ;
- `src-tauri/system/{mod.rs,paths.rs,settings.rs,workspace.rs}` ;
- `src-tauri/tauri.conf.json`, `capabilities/default.json` ;
- `src-tauri/icons/32x32.png`, `128x128.png`, `128x128@2x.png`.

### `apps/ui`

- `package.json`, `tsconfig.json`, `README.md`, `src/index.ts` ;
- icônes, primitives, boutons, champs, dialogues, menus, panneaux, éditeurs,
  navigation, layouts, thèmes, hooks, styles (voir l'arborescence).

### `apps/app`

- `package.json`, `tsconfig.json`, `vite.config.ts`, `index.html`, `README.md` ;
- application, providers React, features, pages, routes, services, stores,
  styles (voir l'arborescence).

### `packages/`

- `types` : `package.json`, `tsconfig.json`, `src/index.ts` ;
- `contracts` : `package.json`, `tsconfig.json`, `src/index.ts` ;
- `config` : `package.json`, `tsconfig.base.json`.

### Documentation

- `docs/architecture/apps.md` : séparation, dépendances, flux IPC, commandes ;
- `docs/architecture/cli.md` : providers, sélection interactive, menu `/` ;
- `docs/migration/apps-restructure.md` : ce rapport.

## Fichiers modifiés

- `Cargo.toml` : exclusion de `apps/desktop/src-tauri` du workspace racine.
- `crates/sorycode-providers/src/presets.rs` : `create_provider()` et
  `model_catalog()` (fabrique et catalogue partagés).
- `crates/sorycode-providers/src/credentials.rs` : stockage de credentials
  partagé CLI/desktop (créé lors de la refonte providers).
- `crates/sorycode-cli/src/main.rs` : utilise `credentials::apply_to_env`,
  `credentials::save` et `model_catalog` ; suppression des catalogues et du
  stockage de credentials dupliqués.
- `crates/sorycode-providers/src/gemini.rs` : imports inutilisés retirés.
- `crates/sorycode-tools/src/terminal.rs` : correction du timeout (le `Child`
  était déplacé par `wait_with_output` avant le `kill`) et drainage
  concurrent de `stdout`/`stderr` pour éviter les blocages sur gros volumes.
- `crates/sorycode-tools/src/search.rs` : import `Path` inutilisé retiré.
- `apps/desktop/src-tauri/Cargo.toml` + `Cargo.lock` : pile Tauri alignée
  (`tauri 2.8.5`, `tauri-runtime 2.8.0`, `tauri-runtime-wry 2.8.1`,
  `tauri-utils 2.7.0`, `tauri-codegen 2.4.0`, `tauri-macros 2.4.0`) pour
  corriger une incompatibilité upstream entre `tauri-runtime 2.11.3` et
  `tauri-runtime-wry 2.9.3`.
- `apps/desktop/README.md` : description de la nouvelle architecture.
- `AGENTS.md`, `ARCHITECTURE.md`, `ROADMAP.md` : arborescence, interfaces et
  phase 8 mises à jour.

## Fichiers supprimés (legacy mal placés)

- `apps/app/src/main.rs` ;
- `apps/app/Cargo.toml` ;
- `apps/app/tauri.conf.json` ;
- `apps/ui/src/App.tsx` ;
- `apps/ui/src/main.tsx` ;
- `apps/ui/index.html` ;
- `apps/ui/vite.config.ts`.

`apps/ui/package.json` a été remplacé par le manifeste du design system.

## Vérifications

| Commande | Résultat |
| --- | --- |
| `CARGO_BUILD_JOBS=1 cargo build -p sorycode-cli` | OK |
| `./target/debug/sorycode providers` | OK (8 presets) |
| `npm install` | OK (75 paquets) |
| `npm run typecheck` | OK (`types`, `contracts`, `ui`, `app`) |
| `npm run build -w @sorycode/app` | OK (88 modules ; ~206 kB JS, ~14 kB CSS) |
| `cargo check` (`apps/desktop/src-tauri`) | OK (`EXIT:0`, après alignement du lock Tauri et correction du timeout terminal) |

Le build Tauri complet (`tauri build`) n'a pas été exécuté : il produit un
bundle `deb`/`appimage` et demande plus de temps que le budget de la machine.

## Limitations connues

- Les catalogues de modèles sont statiques (`model_catalog`). Le
  rafraîchissement dynamique via l'API des providers reste à implémenter.
- `npm` 12 bloque par défaut les scripts d'installation : exécuter
  `npm install-scripts approve esbuild` avant le premier build Vite.
- L'aperçu navigateur (`npm run dev`) utilise `mockClient` ; les réponses du
  moteur ne sont disponibles que dans la coquille Tauri.
- `apps/desktop/src-tauri` est un workspace Cargo séparé : il recompile les
  crates du Core dans son propre `target/`, ce qui isole le build CLI mais
  allonge le premier build desktop.
- Aucun dépôt git n'est présent : les changements ne sont ni commités ni
  poussés.
