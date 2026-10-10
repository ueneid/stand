# Configuration Format

## Overview

Stand uses a single TOML configuration file named `.stand.toml` located at the project root. This file contains all environment definitions, settings, and variable configurations.

## Configuration File Structure

### Basic Structure
```toml
version = "2.0"

[settings]
show_env_in_prompt = true  # Optional: customize shell behavior

[common]
# Shared variables across all environments

[environments.env_name]
# Environment-specific configuration
```

### Settings Section (Optional)
The `[settings]` section is optional and used to customize Stand's behavior:
```toml
[settings]
show_env_in_prompt = true         # Show current env in shell prompt
nested_shell_behavior = "warn"    # How to handle nested shells: "prevent", "warn", "allow"
auto_exit_on_dir_change = true    # Auto-exit shell when leaving project directory
```

#### auto_exit_on_dir_change
Controls whether the Stand subshell prevents navigation to directories outside the project root (the directory containing `.stand.toml`). This helps maintain clear boundaries between project environments.

**Default: `true` (enabled)**

When enabled and you attempt to navigate outside the project directory:
- The directory change is automatically reverted to your previous location
- A warning message is displayed explaining why
- You can type `exit` to leave the Stand shell and navigate freely

Behavior:
- Child directories within the project are allowed
- Navigating to parent or sibling directories is reverted
- Works with bash, zsh, and fish shells (any directory-changing command)
- Set to `false` to disable this behavior

### Common Variables Section
Variables defined in `[common]` are inherited by all environments:
```toml
[common]
APP_NAME = "MyApp"
LOG_FORMAT = "json"
BASE_URL = "https://api.example.com"
```

### Environment Definitions
Each environment is defined under `[environments.name]`:
```toml
[environments.dev]
description = "Development environment"
color = "green"
DATABASE_URL = "postgres://localhost:5432/dev"
DEBUG = "true"

[environments.prod]
description = "Production environment"
color = "red"
extends = "dev"                # Inherits from dev environment
requires_confirmation = true   # Prompt before switching
DATABASE_URL = "postgres://prod.example.com/myapp"
DEBUG = "false"
```

## Environment Properties

### Standard Properties
- **`description`**: Human-readable description of the environment
- **`color`**: Display color for the environment (used in CLI output)
- **`extends`**: Inherit variables from another environment
- **`requires_confirmation`**: Prompt user before switching to this environment

### Variable Definitions
All other keys in an environment section are treated as environment variables.

### Variable Name Rules
- Names must match `^[A-Za-z_][A-Za-z0-9_]*$`.
- Reserved keys (`description`, `extends`, `color`, `requires_confirmation`) cannot be used as variables.
- **Refused** (they can plant hooks in your interactive shell): `BASH_ENV`, `ENV`, `PROMPT_COMMAND`, `PS0`-`PS4`, `SHELLOPTS`, `BASHOPTS`, `ZDOTDIR`, `HOME`, `XDG_CONFIG_HOME`, `XDG_DATA_DIRS`, `XDG_CONFIG_DIRS`, `IFS`, and names starting with `LD_`, `DYLD_`, `BASH_FUNC_` or `STAND_`.
- **Warned**: `PATH`, `NODE_OPTIONS`, `PYTHONSTARTUP`, `PYTHONPATH`, `PERL5OPT`, `PERL5LIB`, `RUBYOPT`, `RUBYLIB`, `GIT_SSH_COMMAND`, `GIT_EXEC_PATH`, `JAVA_TOOL_OPTIONS`, `_JAVA_OPTIONS`, `NODE_PATH`, `PYTHONHOME`.
- Matching is ASCII case-insensitive (`bash_env` is refused like `BASH_ENV`).
- The rules apply to `stand set` and to injection in `stand shell` / `stand exec`. A hand-edited `.stand.toml` with a refused name makes `shell` and `exec` fail before anything is decrypted; a warned name prints one warning.

See the [Security Guide](security.md#variable-name-guards).

## Variable Interpolation

Use `${VAR_NAME}` syntax to reference system environment variables:
```toml
[environments.dev]
DATABASE_URL = "postgres://${DB_HOST}:5432/dev"
API_KEY = "${DEV_API_KEY}"
```

### Interpolation Rules
- Variables must exist in the system environment
- Unterminated placeholders (`${UNCLOSED`) will cause an error
- Empty variable names (`${}`) are invalid
- Non-existent variables will cause configuration loading to fail

## Environment Inheritance

Use the `extends` property to inherit from another environment:
```toml
[environments.base]
APP_NAME = "MyApp"
LOG_LEVEL = "info"

[environments.dev]
extends = "base"
LOG_LEVEL = "debug"     # Overrides base value
DEBUG = "true"          # Additional variable

[environments.prod]
extends = "dev"
LOG_LEVEL = "warn"      # Overrides dev value
DEBUG = "false"         # Overrides dev value
```

### Inheritance Rules
- Child environments inherit all variables from parent
- Child variables override parent variables with the same name
- Multiple inheritance levels are supported
- Circular references are not allowed
- Common variables are inherited by all environments

## Complete Example

```toml
version = "2.0"

[settings]
show_env_in_prompt = true

[common]
APP_NAME = "MyStandApp"
LOG_FORMAT = "json"
PORT = "3000"

[environments.dev]
description = "Development environment"
color = "green"
DATABASE_URL = "postgres://localhost:5432/dev"
DEBUG = "true"
LOG_LEVEL = "debug"

[environments.staging]
description = "Staging environment"
color = "yellow"
extends = "dev"
DATABASE_URL = "postgres://staging.example.com/myapp"
DEBUG = "false"
LOG_LEVEL = "info"

[environments.prod]
description = "Production environment"
color = "red"
extends = "staging"
requires_confirmation = true
DATABASE_URL = "postgres://prod.example.com/myapp"
LOG_LEVEL = "warn"
```

## Security Considerations

- **Do not commit secrets**: Add `.stand.toml` to `.gitignore` if it contains sensitive data
- **File permissions**: Set appropriate permissions (0600) for files containing secrets
- **Variable interpolation**: Use system environment variables for sensitive values
- **Masked logging**: Sensitive values should never be printed in plain text
- **Encryption**: `[encryption].public_key` holds the project's public key. The private key is not stored in the project. It lives under `$XDG_CONFIG_HOME/stand/identities/` (default `~/.config/stand/identities/`), wrapped by a passphrase and optionally by Touch ID. A legacy `.stand.keys` file is deprecated; run `stand encrypt migrate`.

See the [Security Guide](security.md) for the threat model, setup, CI use and residual risks.
