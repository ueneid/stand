# CLI Commands

## Overview

Stand provides a comprehensive set of commands for managing environment variables across different environments. All commands operate on the `.stand.toml` configuration file in the current project.

## Global Options

```
stand [OPTIONS] <COMMAND>

Options:
  -h, --help     Print help
  -V, --version  Print version
```

## Commands

### `list`
List all available environments defined in the configuration.

```bash
stand list
```

**Output Example:**
```
Available environments:
  dev     Development environment [green]
  staging Staging environment [yellow]
  prod    Production environment [red] 確認要
```

**Implementation Status:** ✅ Implemented

---

### `validate`
Validate the configuration file syntax and structure.

```bash
stand validate
```

**Output Examples:**
```
✓ Configuration is valid
```

```
✗ Configuration validation failed:
  - Missing required field: version
  - Invalid environment reference in 'prod.extends': 'nonexistent'
```

**Implementation Status:** ✅ Implemented

---

### `current`
Show information about the current environment state.

```bash
stand current
```

**Output Example:**
```
Current environment: dev
Configuration: .stand.toml
Status: active
```

**Implementation Status:** ✅ Implemented

---

### `init`
Initialize a new `.stand.toml` configuration file in the current directory.

```bash
stand init [OPTIONS]

Options:
  -f, --force    Overwrite existing configuration file
      --encrypt  Also enable encryption (same as `stand encrypt enable`, without Touch ID)
```

**Behavior:**
- Creates a basic `.stand.toml` with dev/prod environments
- Includes commented `[common]` section with usage examples
- Fails if file already exists (unless `--force` is used)
- Non-interactive (no prompts)

**Implementation Status:** ✅ Implemented

---

### `shell`
Start a new shell session with the specified environment loaded.

```bash
stand shell [OPTIONS] <ENVIRONMENT>

Arguments:
  <ENVIRONMENT>  Environment name to activate

Options:
  -y, --yes            Skip confirmation prompt for environments that require it
      --shell <SHELL>  Shell to use (defaults to $SHELL)
```

**Behavior:**
- Loads all variables from the specified environment
- Starts a new shell session with variables set
- Shows environment indicator in prompt (if configured)
- Environment variables persist only within the shell session
- Detects and prevents nested shells by default (configurable via `nested_shell_behavior`)
- Encrypted values are decrypted once at start (requires [unlock](#unlocking-encrypted-values))
- `stand set` / `stand unset` update the running shell (see [Shell Reflect](#shell-reflect))
- Refuses to inject blocked variable names, before any confirmation or decryption; warned names print one warning (see [`set`](#set))
- Secrets in the shell are readable by any process of your user. Do not start AI agents inside it; prefer `stand exec`

**Implementation Status:** ✅ Implemented

---

### `exec`
Execute a command with the specified environment loaded.

```bash
stand exec [OPTIONS] <ENVIRONMENT> [COMMAND]...

Arguments:
  <ENVIRONMENT>  Environment name to use
  [COMMAND]...   Command to execute

Options:
  -y, --yes      Skip confirmation prompt for environments that require it
```

**Notes:**
- Encrypted values are decrypted once per command (requires [unlock](#unlocking-encrypted-values)).
- Blocked variable names are refused before any confirmation or decryption; warned names print one warning (see [`set`](#set)).

**Examples:**
```bash
stand exec dev -- npm start
stand exec prod -- ./deploy.sh
stand exec staging -- python manage.py migrate
```

**Implementation Status:** ✅ Implemented

---

### `inspect`
Display environment variables for the specified environment.

```bash
stand inspect <ENVIRONMENT> [OPTIONS]

Arguments:
  <ENVIRONMENT>  Environment name to show

Options:
  -v, --values   Show variable values (default: names only)
```

**Output Examples:**
```bash
# stand inspect dev
Environment: dev
Variables:
  DATABASE_URL
  DEBUG
  LOG_LEVEL
  APP_NAME (from common)

# stand inspect dev --values
Environment: dev
Variables:
  DATABASE_URL=postgres://localhost:5432/dev
  DEBUG=true
  LOG_LEVEL=debug
  APP_NAME=MyApp (from common)
```

**Variable Source Attribution:**
- Variables marked with `(from common)` are inherited from the `[common]` section
- Variables marked with `(inherited from <env>)` come from an extended environment
- Variables without annotation are defined locally in the environment

**Security Note:**
The `--values` flag displays actual values of environment variables. Be cautious when using this flag in shared environments or when sensitive data might be exposed.

**Implementation Status:** ✅ Implemented

---

### `env`
Show environment variables in the current Stand subshell.

```bash
stand env [OPTIONS]

Options:
      --json        Output in JSON format
      --stand-only  Show only Stand marker variables (STAND_*)
      --user-only   Show only user-defined variables
```

**Output Example:**
```bash
# stand env
# Stand Environment
STAND_ACTIVE=1
STAND_ENVIRONMENT=dev
STAND_PROJECT_ROOT=/path/to/project

# User Variables
API_KEY=dev-key
DATABASE_URL=postgres://localhost/dev
```

**Behavior:**
- Must be run inside a Stand subshell (started with `stand shell`)
- Shows both Stand marker variables and user-defined variables by default
- Use `--stand-only` or `--user-only` to filter output
- JSON output available for scripting
- Reads values from the current shell's environment. It does not decrypt anything and needs no unlock

**Implementation Status:** ✅ Implemented

---

### `set`
Set a variable in an environment of `.stand.toml`.

```bash
stand set <ENVIRONMENT> <KEY> [VALUE] [OPTIONS]

Arguments:
  <ENVIRONMENT>  Environment name
  <KEY>          Variable name
  [VALUE]        Variable value (if omitted with --encrypt, prompts for input)

Options:
  -e, --encrypt  Encrypt the value before storing
```

**Behavior:**
- Names must match `^[A-Za-z_][A-Za-z0-9_]*$`.
- Refuses reserved keys (`description`, `extends`, `color`, `requires_confirmation`).
- Refuses names that could plant hooks in your shell (`BASH_ENV`, `ENV`, `PROMPT_COMMAND`, `PS0`-`PS4`, `SHELLOPTS`, `BASHOPTS`, `ZDOTDIR`, `HOME`, `XDG_CONFIG_HOME`, `XDG_DATA_DIRS`, `XDG_CONFIG_DIRS`, `IFS`, and prefixes `LD_`, `DYLD_`, `BASH_FUNC_`, `STAND_`).
- Matching is ASCII case-insensitive.
- Warns for `PATH`, `NODE_OPTIONS`, `PYTHONSTARTUP`, `PYTHONPATH`, `PERL5OPT`, `PERL5LIB`, `RUBYOPT`, `RUBYLIB`, `GIT_SSH_COMMAND`, `GIT_EXEC_PATH`, `JAVA_TOOL_OPTIONS`, `_JAVA_OPTIONS`, `NODE_PATH`, `PYTHONHOME`.
- `--encrypt` needs only the project's public key. It does not unlock anything. It warns (and continues) if the public key matches no local identity and there is no `.stand.keys`: the value may then be decryptable only by someone else, so check `[encryption].public_key`.
- Inside `stand shell`, the change is applied to the current shell immediately (see [Shell Reflect](#shell-reflect)). With `--encrypt` the plaintext you just typed is applied without an unlock.

See [Security Guide](security.md#variable-name-guards) for the reasons.

**Implementation Status:** ✅ Implemented

---

### `unset`
Remove a variable from an environment of `.stand.toml`.

```bash
stand unset <ENVIRONMENT> <KEY>

Arguments:
  <ENVIRONMENT>  Environment name
  <KEY>          Variable name to remove
```

**Behavior:**
- Removes the key from `[environments.<ENVIRONMENT>]`. Formatting and file permissions are preserved.
- Fails if the environment does not exist.
- Fails if the key is not in that table. The error hints when the key lives in `[common]` or is inherited through `extends`.
- Reserved keys (`description`, `extends`, `color`, `requires_confirmation`) cannot be unset.
- Inside `stand shell`, the change is applied to the current shell immediately. If the key falls back to an inherited encrypted value, Stand asks you to unlock and applies the decrypted value; if that fails, the variable is removed from the shell and a warning asks you to restart the stand shell.

**Examples:**
```bash
stand unset dev DEBUG
```

**Implementation Status:** ✅ Implemented

---

### `get`
Print the value of a variable. Encrypted values are decrypted (requires [unlock](#unlocking-encrypted-values)).

```bash
stand get <ENVIRONMENT> <KEY>
```

---

### `encrypt`
Manage encryption of values. See the [Security Guide](security.md) for the threat model.

```bash
stand encrypt enable [--touch-id]
stand encrypt migrate
stand encrypt add-touch-id
stand encrypt disable
```

- **`enable`**: Creates a project key pair and stores the private key wrapped by a new passphrase (asked twice) in the identities directory. No `.stand.keys` is created. With `--touch-id` it also wraps the key to a Secure Enclave key (see `add-touch-id` for requirements). `stand init --encrypt` is the same as `enable` without Touch ID.
- **`migrate`**: Moves an existing `.stand.keys` into the identities directory, wrapped by a new passphrase. It verifies the result, then overwrites and deletes `.stand.keys`. Encrypted values are not changed. Refuses if `<public_key>.age` already exists, or if `.stand.keys` is a symlink or has extra hard links.
- **`add-touch-id`**: Adds Touch ID unlock. Requires `brew install age-plugin-se` and macOS 14+ with a Secure Enclave. Unlocks with the passphrase once. Needs an exact project binding (see below). `.se-identity` may only contain `se` plugin identities.
- **`disable`**: Asks for confirmation, decrypts all values and removes `[encryption]` from `.stand.toml`. Requires unlock. The identity files are deleted only after an unlock that verified the project binding; otherwise they are kept and their paths are printed.

The passphrase identity is your backup if the Mac is lost. The Secure Enclave key cannot leave the device.

Identity files live under `$XDG_CONFIG_HOME/stand/identities/` (default `~/.config/stand/identities/`, directory `0700`, files `0600`):
`<public_key>.age`, and with Touch ID also `<public_key>.se.age` and `<public_key>.se-identity`.

---

### Unlocking Encrypted Values

Every decryption (`get`, `exec`, `shell`, `encrypt disable`, and the `unset` fallback) needs an interactive terminal (`/dev/tty`). Stand prints which command wants to decrypt how many secrets for which environment, then asks for Touch ID or the passphrase. Never approve a prompt you did not start.

Unlock order:

1. `STAND_PRIVATE_KEY` (only with `CI=true`/`1` or `STAND_KEY_SOURCE=env`; set without either, Stand refuses with an error)
2. Legacy `.stand.keys` (works, prints a deprecation warning; run `stand encrypt migrate`; used only when no wrapped identity exists)
3. Touch ID (if set up, `age-plugin-se` is on `PATH`, and `STAND_UNLOCK` is not `passphrase`; falls back to the passphrase on failure)
4. Passphrase (3 attempts)

The unwrapped key must match the project's public key.

**Project binding:** the wrapped key's plaintext contains `# project: <path>`. On a mismatch, read-only decryption (`get`, `exec`, `shell`, the `unset` fallback) shows both paths and asks for explicit confirmation for that single run. Operations that write or delete key material (`add-touch-id`, `migrate`, the identity deletion of `disable`) require an exact match.

**Prompt text:** the banner and prompts are sanitized (control characters are escaped; bidirectional and zero-width characters are shown as escapes; long fields keep head and tail). Before Touch ID the banner shows the resolved path of `age-plugin-se`.

See [CI](security.md#ci) for non-interactive use.

---

### Shell Reflect

Inside `stand shell <env>` (bash, zsh, fish), `stand set` and `stand unset` update the current shell immediately. The subshell defines a `stand` shell function that reads the changes from the binary through fd 3 and applies them with strictly quoted `export`/`unset` (fish: `set -gx`/`set -e`) lines. No temp files hold the values.

- Only runs when `STAND_ACTIVE=1`, `STAND_ENVIRONMENT` and `STAND_PROJECT_ROOT` are set and the project directory is the one the shell was started for.
- Only changes that affect the active environment are applied. Stand compares the effective value (`common`, `extends` and `${VAR}` interpolation resolved) of the key for the active environment before and after the write. Unchanged means nothing is sent; a removed value becomes `unset`; a new value is exported.
- `set --encrypt` applies the plaintext you typed without an unlock. An inherited encrypted value (for example after `unset`) is decrypted with the normal unlock; if that fails, the variable is unset and a warning asks you to restart the stand shell.
- If the shell cannot be updated, Stand prints a warning. The configuration was already written, so the exit code stays 0.
- Calling the binary by its full path bypasses the function, so nothing is reflected. The same happens in a shell started by an older stand that didn't define the function. In both cases, if the change affects the active environment, stand prints a warning (`this shell was not updated`) telling you to run `stand set`/`stand unset` without a path or to restart the shell.
- Other shells have no reflect. Restart `stand shell` to see changes.

## Error Handling

### Common Error Scenarios

1. **Configuration file not found:**
   ```
   Error: Configuration file '.stand.toml' not found in current directory
   ```

2. **Invalid environment name:**
   ```
   Error: Environment 'nonexistent' not found. Available: dev, staging, prod
   ```

3. **Configuration validation errors:**
   ```
   Error: Invalid configuration:
     - Line 5: Unknown field 'invalid_field'
     - Line 12: Environment 'prod' extends 'missing' which does not exist
   ```

4. **Permission errors:**
   ```
   Error: Permission denied reading configuration file '.stand.toml'
   ```

## Exit Codes

- `0`: Success
- `1`: Error (invalid arguments, missing files, configuration or decryption errors, etc.)
- `shell` and `exec` return the exit code of the shell or command they ran

## Environment Variables

Stand respects the following environment variables:

- `STAND_PRIVATE_KEY`: Private key for non-interactive unlock. Used only when `CI=true`/`1` or `STAND_KEY_SOURCE=env`. Never export it in an interactive shell profile
- `STAND_KEY_SOURCE`: Set to `env` to allow `STAND_PRIVATE_KEY` outside CI
- `STAND_UNLOCK`: Set to `passphrase` to skip Touch ID
- `XDG_CONFIG_HOME`: Base for the identities directory (default `~/.config`)