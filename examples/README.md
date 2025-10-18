# Stand Examples

This directory contains practical examples demonstrating various features and use cases of Stand.

## Quick Start

Build Stand first:
```bash
cd ..
cargo build --release
alias stand="../target/release/stand"
```

Then navigate to any example directory and try the commands!

## Available Examples

### 1. Basic (`basic/`)

**What it demonstrates:**
- Simple environment definitions (dev, prod)
- Basic variable configuration
- Color coding for environments
- Confirmation requirement for production

**Try it:**
```bash
cd basic/

# List available environments
stand list

# Show dev environment variables
stand show dev --values

# Execute a command with dev environment
stand exec dev sh -c 'echo "Running in $APP_ENV mode with DEBUG=$DEBUG"'

# Execute a command with prod environment
stand exec prod sh -c 'echo "Database: $DATABASE_URL"'
```

**Expected output:**
- Dev environment will have `DEBUG=true` and `LOG_LEVEL=debug`
- Prod environment will have `DEBUG=false` and `LOG_LEVEL=error`

---

### 2. Inheritance (`inheritance/`)

**What it demonstrates:**
- Common variables shared across all environments
- Environment inheritance using `extends`
- Multi-level inheritance (staging extends dev extends base)
- Variable overriding in child environments

**Try it:**
```bash
cd inheritance/

# List environments to see inheritance
stand list

# Show base environment (parent)
stand show base --values

# Show dev environment (inherits from base)
stand show dev --values

# Show staging environment (inherits from dev, which inherits from base)
stand show staging --values

# Notice how variables are inherited and overridden
stand exec staging sh -c 'echo "App: $APP_NAME, Log: $LOG_LEVEL, Port: $PORT"'
```

**Key observations:**
- `APP_NAME` from `[common]` is available in all environments
- `PORT` is defined in `base` and inherited by all children
- `LOG_LEVEL` is overridden at each level (base→dev→staging)
- Staging has variables from base, dev, and its own

---

### 3. Web App (`web-app/`)

**What it demonstrates:**
- Realistic web application configuration
- Multiple service configurations (database, redis, SMTP, S3)
- Environment-specific URLs and endpoints
- Testing environment setup

**Try it:**
```bash
cd web-app/

# Show all environments
stand list

# View complete dev configuration
stand show dev --values

# Execute a mock server start
stand exec dev sh -c 'echo "Starting server on $HOST:$PORT with NODE_ENV=$NODE_ENV"'

# Check database connection string
stand exec prod sh -c 'echo "Database: $DATABASE_URL"'

# View test environment (useful for CI/CD)
stand show test --values
```

**Use cases:**
- Local development setup
- CI/CD testing environment
- Production deployment

---

### 4. Interpolation (`interpolation/`)

**What it demonstrates:**
- Variable interpolation using `${VAR}` syntax
- Referencing system environment variables
- Building complex values from multiple variables
- Dynamic configuration based on runtime environment

**Setup:**
First, set some environment variables:
```bash
export DB_HOST=localhost
export DB_PORT=5432
export API_KEY=test_secret_key_123
export USER_NAME=myuser
```

**Try it:**
```bash
cd interpolation/

# Show how system variables are interpolated
stand show dev --values

# Execute with interpolated variables
stand exec dev sh -c 'echo "Database URL: $DATABASE_URL"'
stand exec dev sh -c 'echo "Connection: $CONNECTION_STRING"'
stand exec dev sh -c 'echo "Home: $HOME_DIR, User: $CURRENT_USER"'

# Without setting env vars, it will error
unset DB_HOST
stand exec dev sh -c 'echo $DATABASE_HOST'  # Will fail with error
```

**Key features:**
- `${HOME}` and `${USER}` use system environment variables
- Multiple variables can be used in a single value
- Useful for secrets management (reference from env, not hardcode)

---

## Common Commands

### Listing Environments
```bash
stand list
```
Shows all available environments with colors and descriptions.

### Showing Variables
```bash
# Show variable names only
stand show <env>

# Show variable names and values
stand show <env> --values
```

### Executing Commands
```bash
# Simple command
stand exec <env> echo "Hello"

# Command with arguments
stand exec <env> sh -c 'echo $MY_VAR'

# With explicit separator
stand exec <env> -- npm start
stand exec <env> -- python script.py --verbose
```

### Validating Configuration
```bash
stand validate
```
Checks configuration file syntax and structure.

---

## Creating Your Own Configuration

1. **Start with basic**: Copy `basic/.stand.toml` and modify variables
2. **Add inheritance**: Use `extends` when environments share common settings
3. **Use interpolation**: Reference secrets from environment variables
4. **Organize by use case**: Group related variables together

### Best Practices

1. **Use `[common]` for truly shared values** (app name, log format)
2. **Use `extends` to reduce duplication** (base → dev → staging → prod)
3. **Use interpolation for secrets** (never commit passwords)
4. **Set `requires_confirmation = true`** for production environments
5. **Use descriptive colors** (green=dev, yellow=staging, red=prod)

---

## Troubleshooting

### "Environment not found"
Make sure you're in a directory with `.stand.toml` file.

### "Variable not found" (interpolation error)
The system environment variable doesn't exist. Set it first:
```bash
export VARIABLE_NAME=value
```

### "Command cannot be empty"
Provide a command after the environment name:
```bash
stand exec dev echo "test"
```

---

## Next Steps

- Read the [main documentation](../docs/)
- Check the [development guidelines](../docs/development-guideline.md)
- Explore [configuration format](../docs/design.md)

Happy coding with Stand! 🚀
