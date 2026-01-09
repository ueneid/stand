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
show_env_in_prompt = true      # Show current env in shell prompt
nested_shell_behavior = "warn" # How to handle nested shells: "prevent", "warn", "allow"
```

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
