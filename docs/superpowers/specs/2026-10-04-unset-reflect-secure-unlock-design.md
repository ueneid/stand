# Design: `unset`, Shell Reflect, and Secure Unlock

Date: 2026-10-04
Status: Accepted

## 1. Goals

1. Add `stand unset <environment> <key>` to remove a variable from `.stand.toml`.
2. Make `stand set` / `stand unset` take effect immediately inside a running `stand shell`.
3. Guard variable names that could plant hooks in the human's shell.
4. Make every decryption a deliberate human act (Touch ID or passphrase on a real terminal), without changing the encrypted value format.
5. Stop `stand env` from decrypting.

Non-goals: defeating a determined same-user attacker without OS isolation; supporting reflect in shells other than bash, zsh, fish; a key caching agent.

## 2. Threat Model

Config values become environment variables of the human's interactive shell after secrets are unlocked. Anyone who can edit `.stand.toml` (for example an AI agent) can plant hooks, and anyone who can read the private key can decrypt.

- **Tier A (curious or accidental agent):** handled.
- **Tier B (prompt-injected or adversarial agent):** bar raised, not defeated by Stand alone. The real wall is OS isolation (sandbox).

Measurements on macOS:

- Claude Code's Bash tool has no controlling terminal. `/dev/tty` fails with "device not configured", so it cannot answer the prompt.
- Agents driving a real terminal (tmux and similar) can see the prompt but do not know the passphrase. With Touch ID they can trigger a system dialog. The banner names the requesting command so the human can refuse prompts they did not start.
- Once secrets are in an environment, any same-user process can read them while the process lives (`ps -wwE -p <pid>` showed the environment of a Homebrew-installed fish).

## 3. Decisions

### 3.1 Envelope model, not per-value plugin recipients

Values stay encrypted to the project X25519 key (`[encryption].public_key`). The private key is wrapped separately.

Why not encrypt each value to a plugin recipient (for example Secure Enclave)?

- A per-value plugin recipient needs one hardware unlock **per value**. With the envelope, one unlock per command decrypts any number of values.
- The value format and `public_key` format stay unchanged. No migration of values. `stand set --encrypt` still needs only the public key.
- The passphrase wrap is standard age, so `age -d -i` can recover it. This is the backup if the Mac is lost.

### 3.2 No caching agent

A long-lived agent that holds the unwrapped key would turn "each decryption is a human act" into "the first decryption is a human act". Any same-user process could then ask the agent. We accept one prompt per command instead.

### 3.3 No temp file for reflect

The `stand` shell function captures fd 3 from the binary and evals the output. Secrets never touch the disk on the data path, so there is no file to leak, race or clean up. Output lines are strictly quoted (`export NAME='...'` / `unset NAME`; fish: `set -gx` / `set -e`).

### 3.4 Name guards

Names must match `^[A-Za-z_][A-Za-z0-9_]*$`.

- Refused: `BASH_ENV`, `ENV`, `PROMPT_COMMAND`, `PS0`-`PS4`, `SHELLOPTS`, `BASHOPTS`, `ZDOTDIR`, `HOME`, `XDG_CONFIG_HOME`, `XDG_DATA_DIRS`, `XDG_CONFIG_DIRS`, `IFS`; prefixes `LD_`, `DYLD_`, `BASH_FUNC_`, `STAND_`.
- Warned: `PATH`, `NODE_OPTIONS`, `PYTHONSTARTUP`, `PYTHONPATH`, `PERL5OPT`, `PERL5LIB`, `RUBYOPT`, `RUBYLIB`, `GIT_SSH_COMMAND`, `GIT_EXEC_PATH`, `JAVA_TOOL_OPTIONS`, `_JAVA_OPTIONS`, `NODE_PATH`, `PYTHONHOME`.
- Matching is ASCII case-insensitive.
- Applied in `stand set` and on injection in `shell` / `exec` (after the environment is resolved, before confirmation and decryption), so a hand-edited `.stand.toml` is covered too. Blocked names are an error; warned names print one warning line.

This is a speed bump, not a complete defense.

## 4. Commands

### `unset`

`stand unset <environment> <key>` removes the key from `[environments.<env>]`, preserving formatting and file permissions (toml_edit).

Errors:

- Unknown environment.
- Key not in that table. A hint is given if it lives in `[common]` or is inherited through `extends`.
- Reserved keys (`description`, `extends`, `color`, `requires_confirmation`) cannot be unset (or set).

### Shell reflect

Inside `stand shell <env>` (bash, zsh, fish) the subshell defines a `stand` shell function. For `set` / `unset` it captures fd 3 from the binary and evals the quoted lines.

- Active only when `STAND_ACTIVE=1`, `STAND_ENVIRONMENT`, `STAND_PROJECT_ROOT`, `STAND_SHELL_FD=3` and a valid `STAND_SHELL_KIND` are set and the project directory matches `STAND_PROJECT_ROOT`.
- The effective value of the key for the active environment (`common`, `extends` and `${VAR}` interpolation resolved) is captured before and after the write. Unchanged: nothing is sent. Removed: `unset`. Plain: `set`.
- Encrypted result: if it is the ciphertext this `set` just wrote, the plaintext is applied without an unlock. Otherwise (for example `unset` falls back to an inherited encrypted value) Stand asks to unlock and applies the decrypted value. If decryption fails, the variable is unset and a warning asks the user to restart the stand shell.
- A failure to write to the shell is only a warning; the configuration was already written and the exit code stays 0.
- Limits: calling the binary by full path (or from a shell started by an older stand) bypasses the function; other shells have no reflect. When stand detects it runs inside a stand shell of the same project without the reflect fd and the active environment's value changed, it prints a warning instead of failing silently.
- Temp dirs for zsh/bash init are random and `0700`.

### `env`

`stand env` reads values from the current shell's environment. It does not decrypt.

### Encryption commands

- `stand encrypt enable [--touch-id]`: asks a new passphrase twice; creates the identity files; no `.stand.keys`.
- `stand encrypt migrate`: wraps the existing `.stand.keys` with a new passphrase into the identities dir, verifies, overwrites and deletes `.stand.keys`. Values untouched. Refuses if `<public_key>.age` already exists, and if `.stand.keys` is a symlink or has extra hard links.
- `stand encrypt add-touch-id`: needs `age-plugin-se` (`brew install age-plugin-se`), macOS 14+ with Secure Enclave. Unlocks with the passphrase once and requires an exact project binding. `.se-identity` may only contain `se` plugin identities.
- `stand encrypt disable`: decrypts all values and removes `[encryption]`. Identity files are deleted only after an unlock that verified the project binding; otherwise they are kept and their paths are printed.
- `stand init --encrypt` = `enable` without Touch ID.

## 5. File Layout

Under `$XDG_CONFIG_HOME/stand/identities/` (default `~/.config/stand/identities/`, dir `0700`, files `0600`):

- `<public_key>.age`: private key, passphrase-wrapped, standard age format.
- `<public_key>.se.age`: private key wrapped to a Secure Enclave key (with Touch ID).
- `<public_key>.se-identity`: `age-plugin-se` identity (with Touch ID).

## 6. Unlock

Every decryption (`get`, `exec`, `shell`, `encrypt disable`, unset fallback) requires an interactive terminal (`/dev/tty`). Stand prints which command wants to decrypt how many secrets for which environment, then asks.

Order:

1. `STAND_PRIVATE_KEY`, only with `CI=true`/`1` or `STAND_KEY_SOURCE=env`. Set without either, Stand refuses with an error.
2. Legacy `.stand.keys` (works, prints a deprecation warning), used only when no wrapped identity exists.
3. Touch ID, if set up, `age-plugin-se` is on `PATH` and `STAND_UNLOCK` is not `passphrase`. Falls back to the passphrase on failure.
4. Passphrase, 3 attempts.

The unwrapped key must match the project's public key.

**Project binding.** The wrapped key's plaintext contains `# project: <path>`. On a mismatch, read-only decryption (`get`, `exec`, `shell`, the `unset` fallback) shows both paths and asks for explicit confirmation for that single run. Operations that write or delete key material (`add-touch-id`, `migrate`, the identity deletion of `disable`) require an exact match and never ask.

**Prompt text.** Banner and prompts are sanitized: control characters are escaped, bidirectional and zero-width characters are shown as escapes, and long fields keep their head and tail. The banner shows the resolved `age-plugin-se` path before Touch ID.

## 7. Pinning

`stand set --encrypt` warns (without blocking) if the project's public key matches no local identity and there is no `.stand.keys`, which detects a swapped key; the value may only be decryptable by someone else, so check `[encryption].public_key`. Same-user forgery is possible, so this is a speed bump.

## 8. Breaking Changes

- Decryption requires a TTY.
- `STAND_PRIVATE_KEY` requires `CI=true` or `STAND_KEY_SOURCE=env`.
- `.stand.keys` is deprecated (run `stand encrypt migrate`).
- `stand env` no longer decrypts.
- `set` refuses reserved and blocked names.

## 9. Residual Risks

- Secrets in use are readable by any same-user process. Do not start AI agents inside `stand shell`; prefer `stand exec` for short-lived commands.
- The wrapped key file is protected only by passphrase strength against a same-user reader.
- Touch ID prompts can be triggered by a terminal-driving agent; the human must refuse prompts they did not start.
- Pinning is forgeable by the same user.
- Unverified: whether the Claude Code sandbox blocks reading other processes' environments.
- Recommended wall for Tier B: sandbox with `denyRead` on `~/.config/stand/` plus a `Read(~/.config/stand/**)` deny rule (see `docs/security.md`). `excludedCommands` run unsandboxed.

## 10. Testing Strategy

Automated:

- Unit tests: name guards (valid, refused, warned, prefixes), reserved keys, `unset` edit with formatting and permission preservation, error hints for `common` / `extends`.
- Unit tests: quoting of reflect output for bash/zsh/fish, including values with quotes, newlines and `$`.
- Unit tests: unlock order and gating (`CI`, `STAND_KEY_SOURCE`, `STAND_UNLOCK`), public-key mismatch rejection.
- Integration tests: `encrypt enable/migrate/disable` with a temp `XDG_CONFIG_HOME`; passphrase supplied through a test hook; identity file modes `0700` / `0600`; `age -d -i` compatibility of `<public_key>.age`.
- Integration tests: reflect in real bash, zsh and fish subshells where installed.
- Use `#[serial]` for tests that touch environment variables.

Not automatable:

- **Touch ID decryption.** It needs a human biometric prompt and a Secure Enclave. Verify manually on macOS 14+: `add-touch-id`, then `get`, `exec`, `shell` with Touch ID, cancel (fallback to passphrase), and `STAND_UNLOCK=passphrase`.
- Behavior of agent tools without a TTY across versions; re-measure manually when upgrading them.
