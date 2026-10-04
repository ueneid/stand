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

1. `STAND_PRIVATE_KEY` (only with `CI=true`/`CI=1` or `STAND_KEY_SOURCE=env`)
2. Legacy `.stand.keys` (works, prints a deprecation warning)
3. Touch ID (see conditions above)
4. Passphrase (3 attempts)

The unwrapped key must match the project's public key.

### Disabling

```bash
stand encrypt disable
```

Decrypts all values, removes `[encryption]` from `.stand.toml` and deletes the identity files. This needs an unlock.

## Variable Name Guards

Names must match `^[A-Za-z_][A-Za-z0-9_]*$`. The guards apply to `stand set` and to injection in `stand shell` / `stand exec`.

**Refused:**
`BASH_ENV`, `ENV`, `PROMPT_COMMAND`, `PS0` to `PS4`, `SHELLOPTS`, `BASHOPTS`, `ZDOTDIR`, `HOME`, `XDG_CONFIG_HOME`, `IFS`, and any name starting with `LD_`, `DYLD_`, `BASH_FUNC_` or `STAND_`.

**Warned:**
`PATH`, `NODE_OPTIONS`, `PYTHONSTARTUP`, `PYTHONPATH`, `PERL5OPT`, `PERL5LIB`, `RUBYOPT`, `RUBYLIB`, `GIT_SSH_COMMAND`, `GIT_EXEC_PATH`.

Reserved keys (`description`, `extends`, `color`, `requires_confirmation`) cannot be set or unset as variables.

## CI

Set `CI=true` (most CI systems do this already) and `STAND_PRIVATE_KEY`:

```yaml
env:
  CI: "true"
  STAND_PRIVATE_KEY: ${{ secrets.STAND_PRIVATE_KEY }}
```

Never export `STAND_PRIVATE_KEY` in an interactive shell profile. Outside CI it is ignored unless `STAND_KEY_SOURCE=env` is set, and a profile export would put the key in every process you start.

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
- **Key pinning is a speed bump.** `stand set --encrypt` warns if the project's public key matches no local identity, which detects a swapped key. A same-user attacker can forge a local identity.
- **Reflect in `stand shell`** evals quoted lines produced by the `stand` binary. Calling the binary by full path bypasses the shell function (no reflect).
- **Same-user file access.** An agent that can read your files can read the passphrase-wrapped key. It is protected by the passphrase strength only. Use a strong passphrase and the sandbox above.
