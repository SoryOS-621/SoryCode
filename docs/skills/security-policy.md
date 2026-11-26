# Security policy — what the agent must respect

Enforced by `sorycode-security` + `.sorycode/execpolicy.json`. This doc explains
the rules so agents (and humans) know what will be allowed, prompted, or denied.

## Permission classes

| Class | Tools | Default |
|---|---|---|
| Read | `read_file`, `list_files`, `search`, `git_status`, `git_diff`, `tool_search`, `update_plan` | Always allowed |
| Write | `write_file`, `edit_file`, `apply_patch` | Allowed (permissive) / **approval** (strict, CLI default) |
| Execute | `terminal`, `run_tests` | Allowed (permissive) / **approval** (strict) |
| Network | `web_search` | **Always approval** (even with `--yolo`) |

## Exec policy (`.sorycode/execpolicy.json`)

Shell commands are evaluated per segment (`&&`, `||`, `;`, `|` split):

```json
{ "rules": [
  { "action": "allow", "pattern": "cargo test*" },
  { "action": "deny",  "pattern": "git push*" },
  { "action": "prompt", "pattern": "rm *" }
]}
```

- First matching rule per segment wins; **any Deny in a compound command kills it**.
- Built-in denies (always active, even `--yolo`): `rm -rf /`, `rm -rf ~*`,
  `mkfs*`, `dd if=/dev/*`, fork bomb, `shutdown`/`reboot`, `git push --force/-f`.
- User rules are checked **after** built-ins → `allow` can never override a
  built-in `deny`.
- No match → `prompt` (approval in strict mode).

Manage rules from the CLI: `/allow <glob>`, `/deny <glob>`.

## Approvals

- Strict mode (default): writes/exec/network ask `[approval] ... [y/N]`.
- `--yolo`: auto-approve everything **except** built-in exec denies and
  `NetworkAccess` still routes through policy (currently auto-approved by the
  permissive default — tighten if needed).
- Approved `(tool, args)` pairs are cached per task — identical calls don't
  re-prompt.

## Path confinement

`write_file`, `edit_file`, `list_files`, `search`, `apply_patch`, `terminal.cwd`
are confined to the working dir: `..` escapes and absolute paths outside are
rejected with `PermissionDenied`.

## Secrets

- API keys live in env vars or `~/.sorycode/credentials.json` (mode 600),
  never in `.sorycode/sorycode.db.json`.
- Agents must not read credential files into context or echo them.
