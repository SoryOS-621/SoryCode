# CLI — REPL, providers et commandes

Le binaire `sorycode` (crate `crates/sorycode-cli`) est un client du Core. Il
n'embarque aucune logique d'agent : il envoie des `CoreCommand`, consomme les
`EngineEvent` et affiche le résultat.

## Démarrage

```bash
cargo build -p sorycode-cli
./target/debug/sorycode                 # REPL
./target/debug/sorycode --provider opencode-zen
./target/debug/sorycode run --prompt "explain this repo"
./target/debug/sorycode providers
./target/debug/sorycode connect
./target/debug/sorycode doctor
```

Options globales : `--dir`, `--provider`, `--model`, `--api-key`, `--base-url`.
La variable d'environnement du provider reste prioritaire sur le fichier de
credentials.

## Providers

Presets intégrés (`PROVIDER_PRESETS`) :

| Provider | Clé d'environnement | Modèles |
| --- | --- | --- |
| `openai` | `OPENAI_API_KEY` | GPT-4o / 4.1 / o3-mini |
| `gemini` | `GEMINI_API_KEY` | Gemini 2.5 / 2.0 |
| `mistral` | `MISTRAL_API_KEY` | Mistral Large / Small / Codestral |
| `groq` | `GROQ_API_KEY` | Llama 3.3, GPT-OSS, Qwen |
| `nvidia` | `NVIDIA_API_KEY` | Nemotron, Llama, DeepSeek, Kimi |
| `openrouter` | `OPENROUTER_API_KEY` | catalogue OpenRouter |
| `openrouter-free` | `OPENROUTER_API_KEY` | modèles `:free` |
| `opencode-zen` | `OPENCODE_ZEN_API_KEY` | modèles `*-free` (accès anonyme) |

`openrouter` et `openrouter-free` partagent la même URL de base mais sont deux
presets distincts : le second ne propose que des modèles gratuits et fonctionne
sans clé payante.

Les catalogues sont exposés par `model_catalog(name)` dans
`sorycode-providers`. Ils servent de repli statique ; un client peut les
rafraîchir depuis l'API du provider.

## Sélection interactive

Dans le REPL :

- `/provider` affiche la liste numérotée des providers ; on choisit par numéro
  ou par nom ;
- `/model` affiche tous les couples `provider/model` disponibles ; on choisit
  par numéro ou par identifiant complet ;
- `/connect` guide la saisie d'une clé API et l'enregistre dans
  `~/.sorycode/credentials.json` (permissions `0600`).

Le prompt affiche en permanence `provider / model · session`. Changer de
provider reconstruit l'orchestrateur avec le modèle par défaut du preset.

## Menu de commandes

L'éditeur de ligne (`crates/sorycode-cli/src/input.rs`) passe le terminal en
mode raw et affiche un menu filtré dès que la saisie commence par `/`.

- `↑` / `↓` : naviguer dans les suggestions ;
- `Tab` : compléter la commande sélectionnée ;
- `Entrée` : exécuter la commande sélectionnée ;
- `Échap` : fermer le menu sans quitter l'édition ;
- `Ctrl+C` / `Ctrl+D` : quitter le REPL.

Commandes disponibles : `/help`, `/exit`, `/quit`, `/clear`, `/new`,
`/sessions`, `/history`, `/resume <id>`, `/cleanhistory`, `/providers`,
`/connect`, `/model [provider/model]`, `/provider [name]`.

## Credentials

`sorycode_providers::credentials` est partagé par le CLI et le shell desktop :

- `path()` : `~/.sorycode/credentials.json` ;
- `load()` / `get()` / `save()` / `remove()` ;
- `resolve(provider)` : variable d'environnement d'abord, fichier ensuite ;
- `apply_to_env()` : recopie les clés du fichier dans l'environnement au
  démarrage du CLI, sans écraser une variable déjà définie.

Le Core ne stocke jamais de secret en clair dans sa base de sessions.
