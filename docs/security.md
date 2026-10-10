# Security Guide

## Overview

Stand can encrypt secret values in `.stand.toml` with [age](https://age-encryption.org). This guide explains what Stand protects, what it does not, and how to set it up safely.

Short version:

- Secrets are protected **at rest**. Every decryption is a **human act** (Touch ID or passphrase on a real terminal).
- Secrets **in use** (inside a running shell or command) are readable by any process of the same user.
- Stand raises the bar for AI agents. It does not defeat a determined, adversarial agent by itself. For that, use OS isolation (see [Sandbox Recommendation](#sandbox-recommendation)).

## Threat Model

| Tier | Who | Status |
|------|-----|--------|
| A | Curious or accidental agent (reads files, runs commands by mistake) | Handled |
| B | Prompt-injected or adversarial agent | Bar raised, **not** defeated by Stand alone |

What Stand does:

- Values are encrypted to a project X25519 public key (`[encryption].public_key`).
- The private key is **not** stored in the project. It lives in your user config directory, wrapped by a passphrase and optionally by a Secure Enclave key (Touch ID).
- Every decryption needs an interactive terminal (`/dev/tty`). It prints which command wants to decrypt how many secrets for which environment, then asks for Touch ID or the passphrase.
- Variable names that could plant shell hooks are refused (see [Variable Name Guards](#variable-name-guards)).

### Why name guards exist

After secrets are unlocked, config values become environment variables of your interactive shell. Anyone who can edit `.stand.toml` (for example an AI agent) could otherwise plant hooks such as `BASH_ENV`, `PROMPT_COMMAND` or `LD_PRELOAD`. The guards are a speed bump, not a complete defense.

## Setup

### New project

```bash
stand init --encrypt                 # passphrase only
stand encrypt enable --touch-id      # alternative: passphrase + Touch ID (macOS)
```

`stand encrypt enable` asks for a new passphrase twice. No `.stand.keys` file is created.

Then store a secret:

```bash
stand set dev API_KEY --encrypt      # prompts for the value
```

`stand set --encrypt` needs only the public key. It does not unlock anything.

### Migrating from `.stand.keys`

`.stand.keys` is deprecated. It still works, but prints a warning. To migrate:

```bash
stand encrypt migrate
```

This wraps the existing private key with a new passphrase, stores it in the identities directory, verifies the result, then overwrites and deletes `.stand.keys`. Encrypted values in `.stand.toml` are **not** touched.

`migrate` refuses to run if `<public_key>.age` already exists (it never overwrites an identity), and if `.stand.keys` is a symlink or has extra hard links (overwriting it would destroy another file).

Afterwards, check that no copy of `.stand.keys` remains in backups or in git history.

### Touch ID setup (macOS 14+ with Secure Enclave)

```bash
brew install age-plugin-se
stand encrypt add-touch-id           # unlocks with the passphrase once
```

Touch ID is used when all of these hold: it is set up, `age-plugin-se` is on `PATH`, and `STAND_UNLOCK` is not `passphrase`. If Touch ID fails, Stand falls back to the passphrase.

**Backup:** the Secure Enclave key cannot leave the device. The passphrase identity is your backup if you lose the Mac. Keep the passphrase safe.

### Where keys live

Under `$XDG_CONFIG_HOME/stand/identities/` (default `~/.config/stand/identities/`). Directory mode `0700`, files `0600`.

| File | Content |
|------|---------|
| `<public_key>.age` | Private key wrapped by passphrase (standard age format, works with `age -d -i`) |
| `<public_key>.se.age` | Private key wrapped to a Secure Enclave key (Touch ID only) |
| `<public_key>.se-identity` | `age-plugin-se` identity (Touch ID only) |

### Unlock order

1. `STAND_PRIVATE_KEY`. Only with `CI=true`/`CI=1` or `STAND_KEY_SOURCE=env`. If it is set without one of these, Stand **refuses** with an error (it does not silently ignore it).
2. Legacy `.stand.keys` (works, prints a deprecation warning). Used only when no wrapped identity for the project exists.
3. Touch ID (see conditions above)
4. Passphrase (3 attempts)

Steps 3 and 4 need an interactive terminal. The unwrapped key must match the project's public key.

#### Project binding

The wrapped key's plaintext contains a line `# project: <path>` with the project directory it was created for. When you unlock, Stand compares it with the current project directory.

- **Read-only decryption** (`get`, `exec`, `shell`, the `unset` fallback): on a mismatch Stand shows both paths and asks for explicit confirmation, for that single run only.
- **Operations that write or delete key material** (`add-touch-id`, `migrate`, and the identity deletion of `disable`) require an exact match. They refuse on a mismatch and do not ask.

#### What the prompt shows

The banner names the command, the number of secrets and the environment. Before Touch ID it also shows the resolved path of the `age-plugin-se` binary that will run, so you can cancel if it is not the plugin you installed. `.se-identity` files may only contain `se` plugin identities; any other plugin name is refused.

All text taken from files, environment or plugins (command lines, paths, plugin messages) is sanitized before it is printed: control characters are escaped, and bidirectional and zero-width characters are shown as `\u{...}` escapes. Long fields keep their head and tail, with the middle summarized, so neither end can hide content.

### Disabling

```bash
stand encrypt disable
```

Asks for confirmation, decrypts all values and removes `[encryption]` from `.stand.toml`. This needs an unlock.

The identity files are deleted only after an unlock that verified the project binding. If the binding was not verified (for example you confirmed a mismatch), Stand keeps the files and prints their paths so you can delete them yourself.

## Variable Name Guards

Names must match `^[A-Za-z_][A-Za-z0-9_]*$`. Matching is ASCII case-insensitive (`bash_env` is refused like `BASH_ENV`). The guards apply to `stand set` and to injection in `stand shell` / `stand exec`. `shell` and `exec` check the resolved variable names before any confirmation or decryption: a blocked name stops the command with an error that lists the names; a warned name prints one warning line to stderr. Removing a risky name with `stand unset` is always allowed.

**Refused:**
`BASH_ENV`, `ENV`, `PROMPT_COMMAND`, `PS0` to `PS4`, `SHELLOPTS`, `BASHOPTS`, `ZDOTDIR`, `HOME`, `XDG_CONFIG_HOME`, `XDG_DATA_DIRS`, `XDG_CONFIG_DIRS`, `IFS`, and any name starting with `LD_`, `DYLD_`, `BASH_FUNC_` or `STAND_`.

**Warned:**
`PATH`, `NODE_OPTIONS`, `PYTHONSTARTUP`, `PYTHONPATH`, `PERL5OPT`, `PERL5LIB`, `RUBYOPT`, `RUBYLIB`, `GIT_SSH_COMMAND`, `GIT_EXEC_PATH`, `JAVA_TOOL_OPTIONS`, `_JAVA_OPTIONS`, `NODE_PATH`, `PYTHONHOME`.

Reserved keys (`description`, `extends`, `color`, `requires_confirmation`) cannot be set or unset as variables.

## CI

Set `CI=true` (most CI systems do this already) and `STAND_PRIVATE_KEY`:

```yaml
env:
  CI: "true"
  STAND_PRIVATE_KEY: ${{ secrets.STAND_PRIVATE_KEY }}
```

Never export `STAND_PRIVATE_KEY` in an interactive shell profile. Outside CI, Stand refuses to run with it set unless `STAND_KEY_SOURCE=env` is also set, and a profile export would put the key in every process you start.

## Sandbox Recommendation

OS isolation is the real wall against Tier B. Example for Claude Code (see [Sandboxing](https://code.claude.com/docs/en/sandboxing)):

```json
{
  "sandbox": {
    "enabled": true,
    "allowUnsandboxedCommands": false,
    "filesystem": { "denyRead": ["~/.config/stand/"] }
  },
  "permissions": { "deny": ["Read(~/.config/stand/**)"] }
}
```

- `denyRead` covers Bash commands and their child processes. It is enforced by the OS (Seatbelt on macOS, bubblewrap on Linux).
- The `Read(...)` permission rule covers the Read tool.
- Commands listed in `excludedCommands` run **unsandboxed**. Do not exclude `stand` or shells.
- Whether the sandbox also blocks reading other processes' environments is **unverified**.

## Residual Risks

Stated honestly:

- **No terminal for agents.** Measured on macOS: Claude Code's Bash tool has no controlling terminal (`/dev/tty` fails with "device not configured"), so it cannot answer the passphrase prompt.
- **Agents that drive a real terminal** (tmux and similar) can see the prompt but do not know the passphrase. With Touch ID they can trigger a system dialog. **Never approve a prompt you did not start.** The prompt names the command that wants to decrypt for this reason.
- **Secrets in use are exposed.** Measured: once secrets are in an environment, any process of the same user can read them while the process lives (`ps -wwE -p <pid>` showed the environment of a Homebrew-installed fish). Stand protects secrets at rest and makes each decryption a human act. It cannot hide secrets from the process that uses them.
  - Do not start AI agents inside a `stand shell`.
  - Prefer `stand exec` for short-lived commands.
- **Key pinning is a speed bump.** `stand set --encrypt` warns (it does not block) if the project's public key matches no local identity and there is no `.stand.keys`, which detects a swapped key; the value may then be decryptable only by someone else, so check `[encryption].public_key`. A same-user attacker can forge a local identity.
- **Reflect in `stand shell`** evals quoted lines produced by the `stand` binary. Calling the binary by full path bypasses the shell function (no reflect); stand then warns that the shell was not updated. If an `unset` falls back to an inherited encrypted value and it cannot be decrypted, the variable is removed from the shell and a warning asks you to restart the stand shell.
- **Same-user file access.** An agent that can read your files can read the passphrase-wrapped key. It is protected by the passphrase strength only. Use a strong passphrase and the sandbox above.
