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

**Examples:**
```bash
stand exec dev -- npm start
stand exec prod -- ./deploy.sh
stand exec staging -- python manage.py migrate
```

**Implementation Status:** ✅ Implemented

---

### `show`
Display environment variables for the specified environment.

```bash
stand show <ENVIRONMENT> [OPTIONS]

Arguments:
  <ENVIRONMENT>  Environment name to show

Options:
  -v, --values   Show variable values (default: names only)
```

**Output Examples:**
```bash
# stand show dev
Environment: dev
Variables:
  DATABASE_URL
  DEBUG
  LOG_LEVEL
  APP_NAME (from common)

# stand show dev --values
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

**Implementation Status:** ✅ Implemented

---

### `set`
Set an environment variable in the specified or current environment.

```bash
stand set <NAME> <VALUE> [OPTIONS]

Arguments:
  <NAME>   Variable name
  <VALUE>  Variable value

Options:
  -e, --env <ENVIRONMENT>  Target environment (default: current)
```

**Implementation Status:** 🚧 Planned

---

### `unset`
Remove an environment variable from the specified or current environment.

```bash
stand unset <NAME> [OPTIONS]

Arguments:
  <NAME>  Variable name to remove

Options:
  -e, --env <ENVIRONMENT>  Target environment (default: current)
```

**Implementation Status:** 🚧 Planned

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
- `1`: General error (invalid arguments, missing files, etc.)
- `2`: Configuration validation error
- `3`: Permission error
- `130`: Interrupted by user (Ctrl+C)

## Environment Variables

Stand respects the following environment variables:

- `STAND_CONFIG`: Override default configuration file path
- `STAND_NO_COLOR`: Disable colored output
- `STAND_QUIET`: Suppress non-error output