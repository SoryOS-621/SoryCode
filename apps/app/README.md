# apps/app — application métier

Toute l'expérience fonctionnelle de SoryCode : chat, code, work, projets,
workspaces, sessions, agents et settings.

```
src/
  app/          App, providers React, point d'entrée
  features/     chat, code, work, projects, workspaces, sessions, agents, settings
  pages/        une page par route
  routes/       définitions et hook de routage par hash
  services/     coreClient, tauriClient, mockClient
  stores/       store minimal + session + UI
  styles/       styles applicatifs
```

## Services

- `coreClient.ts` : contrat unique utilisé par les features.
- `tauriClient.ts` : implémentation de production via `invoke`/`listen`.
- `mockClient.ts` : aperçu navigateur sans coquille Tauri.

Aucune feature n'importe `@tauri-apps/api` directement : tout passe par
`services/`.

## Routes

`#/chat`, `#/code`, `#/work`, `#/projects`, `#/workspaces`, `#/sessions`,
`#/agents`, `#/settings`.

## Commandes

```bash
npm run dev        # Vite sur 127.0.0.1:5173
npm run build      # tsc --noEmit + vite build -> dist/
npm run typecheck
```

`apps/desktop/src-tauri/tauri.conf.json` charge `apps/app/dist`.
