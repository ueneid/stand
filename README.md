# Stand

A CLI tool for explicit environment variable management that provides a clean, organized way to handle different environments (dev, staging, prod) with their specific configurations.

## Quick Start

### Installation

```bash
git clone https://github.com/ueneid/stand
cd stand
cargo build --release
```

### Basic Usage

1. **Initialize a new project:**
   ```bash
   stand init
   ```

2. **List available environments:**
   ```bash
   stand list
   ```

3. **Show variables for an environment:**
   ```bash
   stand show dev --values
   ```

4. **Execute command with environment:**
   ```bash
   stand exec dev -- npm start
   ```

### Configuration Example

Create a `.stand.toml` file in your project root:

```toml
version = "2.0"

[settings]
default_environment = "dev"

[common]
APP_NAME = "MyApp"
LOG_FORMAT = "json"

[environments.dev]
description = "Development environment"
color = "green"
DATABASE_URL = "postgres://localhost:5432/dev"
DEBUG = "true"

[environments.prod]
description = "Production environment"
color = "red"
extends = "dev"
requires_confirmation = true
DATABASE_URL = "postgres://prod.example.com/myapp"
DEBUG = "false"
```

## Features

- **Environment Management**: Define and switch between multiple environments
- **Variable Inheritance**: Use `extends` to inherit from other environments
- **Variable Interpolation**: Reference system environment variables with `${VAR}`
- **Shell Integration**: Start shell sessions with environment loaded
- **Command Execution**: Execute commands with specific environment variables
- **Configuration Validation**: Validate configuration file syntax and structure

## Examples

Learn by example! Check out the [examples/](examples/) directory for practical demonstrations:

- **[basic/](examples/basic/)** - Simple environment setup (dev, prod)
- **[inheritance/](examples/inheritance/)** - Using `extends` for environment inheritance
- **[web-app/](examples/web-app/)** - Realistic web application configuration
- **[interpolation/](examples/interpolation/)** - Variable interpolation with `${VAR}` syntax

Each example includes a complete `.stand.toml` configuration and usage instructions. See the [Examples README](examples/README.md) for detailed walkthroughs.

## Documentation

### User Documentation
- **[CLI Commands](docs/cli.md)** - Complete command reference
- **[Configuration Format](docs/configuration.md)** - Configuration file syntax and examples

### Developer Documentation
- **[Architecture Overview](docs/architecture.md)** - High-level system design
- **[Development Guidelines](docs/development-guideline.md)** - TDD workflow and build commands
- **[Design Documentation](docs/design.md)** - Detailed design decisions
- **[Requirements](docs/requirements.md)** - Feature specifications
- **[PR Review Guidelines](docs/pr-review-guidelines.md)** - Pull request checklist

### AI Assistant Documentation
- **[AGENTS.md](AGENTS.md)** - AI assistant instructions and workflow guidelines
- **[CLAUDE.md](CLAUDE.md)** - Quick reference for development

## Development

### Prerequisites
- Rust 2021 edition
- Cargo

### Setup
```bash
git clone https://github.com/ueneid/stand
cd stand
cargo build
```

### Running Tests
```bash
cargo test
```

### Code Quality
```bash
cargo fmt && cargo clippy -- -D warnings && cargo test
```

### Command Implementation Status

- ✅ `list` - List all available environments
- ✅ `validate` - Validate configuration file
- ✅ `current` - Show current environment status
- 🚧 `init` - Initialize new configuration (planned)
- ✅ `shell` - Start interactive shell with environment loaded
- ✅ `exec` - Execute command with environment variables
- ✅ `show` - Show environment variables with source attribution
- ✅ `env` - Show active environment variables in current subshell
- 🚧 `switch` - Switch default environment (planned)
- 🚧 `set`/`unset` - Modify environment variables (planned)

## Contributing

1. Read the [development guidelines](docs/development-guideline.md)
2. Follow the TDD workflow (RED → GREEN → REFACTOR)
3. Create feature branches from `main`
4. Ensure all tests pass before submitting PR
5. Follow the [PR review guidelines](docs/pr-review-guidelines.md)

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Support

- **Issues**: [GitHub Issues](https://github.com/ueneid/stand/issues)
- **Documentation**: See `docs/` directory
- **Development**: See [AGENTS.md](AGENTS.md) for AI assistant guidelines