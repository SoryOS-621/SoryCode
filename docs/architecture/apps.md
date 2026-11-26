# Applications — séparation `desktop` / `ui` / `app`

SoryCode est découpé en trois applications strictement indépendantes sous
`apps/`, plus le moteur Rust sous `crates/`. Aucune de ces trois applications
ne contient de logique de moteur : le Core reste autonome, sans dépendance à
React, Tauri ou au CLI.

```
apps/
  desktop/   coquille native Tauri (Rust)
  ui/        design system React (bibliothèque)
  app/       application React métier (features, pages, routes)
```

## 1. `apps/desktop` — coquille native

Rôle : tout ce qui touche à la machine et au système d'exploitation.

- cycle de vie Tauri, fenêtres, menus natifs ;
- permissions et capacités Tauri (`capabilities/`) ;
- accès aux fichiers locaux, dialogues, notifications ;
- raccourcis globaux, terminaux, intégration OS ;
- pont IPC : commandes `#[tauri::command]` et événements `sorycode://event` ;
- initialisation de `EngineState` (providers, workspace, stockage).

Il ne contient **aucun composant React** et ne dépend ni de `apps/ui` ni de
`apps/app`. Le frontend qu'il charge est le build de `apps/app`
(`frontendDist: ../../app/dist`).

```
apps/desktop/
  package.json          scripts Tauri (dev, build, délégation UI)
  src-tauri/
    Cargo.toml          crate autonome (hors workspace racine)
    tauri.conf.json     fenêtre, build, bundle, icônes
    capabilities/       permissions Tauri
    icons/              icônes de bundle
    src/
      lib.rs            builder Tauri + enregistrement IPC
      state.rs          EngineState (provider/modèle/workspace)
      events.rs         EventSink -> émission Tauri
      ipc/dto.rs        DTO camelCase exposés au frontend
      commands/         providers, sessions, chat, projects, system
      system/           chemins, settings, inspection confinée
```

## 2. `apps/ui` — design system

Rôle : bibliothèque de composants et de styles, sans logique métier.

- primitives : `Stack`, `Text`, `Surface`, `Feedback` (Badge, Kbd, Spinner,
  Tooltip, Divider) ;
- composants : boutons, champs, dialogues, menus, palette de commandes,
  panneaux, blocs de code/diff, navigation, onglets ;
- layouts : `AppShell`, `Sidebar`, `AppHeader`, `AppContent`, `SplitLayout` ;
- thèmes : `ThemeProvider` (`light` / `dark` / `system`) ;
- hooks génériques : `useMediaQuery`, `useHotkeys`, `useControllableState` ;
- icônes SVG (`Icon`) et feuilles de style (`tokens.css`, `base.css`,
  `components.css`).

Contraintes :

- aucun import de `apps/app` ni de `apps/desktop` ;
- aucune connaissance du moteur, des providers ou des sessions ;
- consommé en source TypeScript par Vite (`exports` vers `src/index.ts`),
  validé par `tsc --noEmit`.

## 3. `apps/app` — application métier

Rôle : toute l'expérience fonctionnelle.

- features : `chat`, `code`, `work`, `projects`, `workspaces`, `sessions`,
  `agents`, `settings` ;
- pages et routes (`#/chat`, `#/code`, …) ;
- stores (`createStore`, session, UI) et providers React (`EngineProvider`,
  `AppProviders`) ;
- services : `coreClient` (contrat), `tauriClient` (production),
  `mockClient` (aperçu navigateur) ;
- styles applicatifs complémentaires.

`apps/app` importe `@sorycode/ui` pour l'apparence et `@sorycode/contracts`
pour les types wire. Il ne parle jamais directement à Tauri en dehors de
`services/tauriClient.ts`.

## Règles de dépendance

```
@sorycode/types       (domaine pur)
      ↑
@sorycode/contracts   (wire IPC + événements)
      ↑
@sorycode/ui          (design system, React uniquement)
      ↑
apps/app              (features métier)
      ↑
apps/desktop          (coquille native, charge le build de app)
```

- `desktop` fournit des capacités natives à `app` via le contrat `CoreClient`.
- `app` ne dépend jamais de `desktop` au niveau du code : uniquement des
  commandes/événements Tauri, encapsulés dans un service.
- `ui` ne dépend ni de `desktop` ni de `app`.
- Le Core Rust (`crates/`) ne dépend d'aucune application.

## Flux IPC

```
apps/app (React)
   │  invoke("send_message", …) / listen("sorycode://event")
   ▼
apps/desktop (Rust)
   │  CoreCommand
   ▼
sorycode-application -> sorycode-agent -> providers / tools
   │  EngineEvent
   ▼
EventSink (TauriSink) -> "sorycode://event" -> apps/app
```

Les DTO de commande sont en camelCase côté frontend ; Tauri convertit vers les
paramètres snake_case Rust. Les événements du moteur conservent leurs champs
snake_case, reflétés par `@sorycode/contracts`.

## Développement

```bash
# Dépendances JS (workspaces npm)
npm install

# Application web seule (client mock dans un navigateur)
npm run dev

# Application + coquille native
npm run desktop:dev

# Vérifications
npm run typecheck
CARGO_BUILD_JOBS=1 cargo build -p sorycode-cli
cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml
```

Le build Tauri exige les bibliothèques système `webkit2gtk-4.1` et `gtk+-3.0`
(plus `libayatana-appindicator3` selon la distribution).

Le `Cargo.lock` de `apps/desktop/src-tauri` épingle toute la pile runtime
Tauri (`tauri-runtime`, `tauri-runtime-wry`, `tauri-utils`, `tauri-codegen`,
`tauri-macros`) : des versions plus récentes de `tauri-runtime-wry` ne sont pas
encore compatibles avec les ajouts de trait de `tauri-runtime`. Éviter
`cargo update` sur ce crate tant que la pile upstream n'est pas alignée.
