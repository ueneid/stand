# Architecture Overview

## High-Level Architecture

Stand is designed as a modular CLI tool with clear separation of concerns. The architecture follows domain-driven design principles to ensure maintainability and extensibility.

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   CLI Layer     │───▶│  Commands       │───▶│  Domain Logic   │
│                 │    │                 │    │                 │
│ • Argument      │    │ • list          │    │ • Config        │
│   parsing       │    │ • validate      │    │ • Environment   │
│ • Help text     │    │ • current       │    │ • Variables     │
│ • Error         │    │ • init          │    │ • Validation    │
│   formatting    │    │ • shell         │    │ • Interpolation │
└─────────────────┘    │ • exec          │    └─────────────────┘
                       │ • show          │              │
                       │ • set/unset     │              ▼
                       └─────────────────┘    ┌─────────────────┐
                                              │  Infrastructure │
                                              │                 │
                                              │ • File I/O      │
                                              │ • TOML parsing  │
                                              │ • Env vars      │
                                              │ • Shell exec    │
                                              └─────────────────┘
```

## Module Organization

### Source Structure
```
src/
├── main.rs              # Entry point and CLI wiring
├── lib.rs               # Library exports
├── cli/                 # CLI interface and argument parsing
│   ├── mod.rs
│   └── commands.rs      # Clap command definitions
├── commands/            # Command implementations
│   ├── mod.rs
│   ├── list.rs          # List environments
│   ├── validate.rs      # Validate configuration
│   ├── current.rs       # Show current status
│   ├── init.rs          # Initialize configuration
│   ├── shell.rs         # Shell integration
│   ├── exec.rs          # Execute with environment
│   ├── show.rs          # Display variables
│   └── variables.rs     # Set/unset variables
├── config/              # Configuration management
│   ├── mod.rs
│   ├── types.rs         # Configuration data structures
│   ├── loader.rs        # TOML loading and parsing
│   ├── validator.rs     # Configuration validation
│   └── interpolation.rs # Variable interpolation
├── environment/         # Environment management
│   ├── mod.rs
│   ├── manager.rs       # Environment operations
│   └── inheritance.rs   # Variable inheritance logic
├── shell/               # Shell integration
│   ├── mod.rs
│   ├── detector.rs      # Shell type detection
│   └── executor.rs      # Command execution
├── state/               # Application state
│   ├── mod.rs
│   └── current.rs       # Current environment tracking
└── utils/               # Shared utilities
    ├── mod.rs
    ├── errors.rs        # Error types and handling
    ├── colors.rs        # Terminal color management
    └── validation.rs    # Common validation functions
```

## Core Components

### 1. CLI Layer (`src/cli/`)
- **Responsibility**: Command-line interface definition and argument parsing
- **Key Files**: `commands.rs` (Clap command structure)
- **Dependencies**: `clap` for argument parsing
- **Interface**: Translates user input to command execution

### 2. Commands Layer (`src/commands/`)
- **Responsibility**: Implementation of individual CLI commands
- **Pattern**: Each command is a separate module with a main handler function
- **Error Handling**: Returns `anyhow::Result` for error propagation
- **Testing**: Each command has comprehensive unit and integration tests

### 3. Configuration Management (`src/config/`)
- **Responsibility**: TOML configuration loading, parsing, and validation
- **Key Components**:
  - `types.rs`: Data structures (Configuration, Environment, Settings)
  - `loader.rs`: File reading and TOML deserialization
  - `validator.rs`: Configuration validation logic
  - `interpolation.rs`: Variable interpolation with `${VAR}` syntax

### 4. Environment Management (`src/environment/`)
- **Responsibility**: Environment operations and variable inheritance
- **Key Features**:
  - Environment variable resolution
  - Inheritance chain processing (`extends` functionality)
  - Variable merging and override logic

### 5. Shell Integration (`src/shell/`)
- **Responsibility**: Shell detection and command execution
- **Capabilities**:
  - Detect current shell type (bash, zsh, fish, etc.)
  - Execute commands with modified environment
  - Shell prompt integration

### 6. State Management (`src/state/`)
- **Responsibility**: Track current environment and application state
- **Scope**: Session-local state (not persistent)

### 7. Utilities (`src/utils/`)
- **Responsibility**: Shared functionality across modules
- **Components**: Error handling, color management, validation helpers

## Data Flow

### Configuration Loading Flow
```
.stand.toml ──▶ loader.rs ──▶ validator.rs ──▶ Configuration
                   │               │              struct
                   ▼               ▼
              TOML parsing    Validation     ──▶ Ready for use
                              rules
```

### Variable Resolution Flow
```
Environment ──▶ inheritance.rs ──▶ interpolation.rs ──▶ Final variables
definition           │                    │
                     ▼                    ▼
                Apply extends      Apply ${VAR}
                chain             substitution
```

### Command Execution Flow
```
CLI args ──▶ commands.rs ──▶ command/*.rs ──▶ Domain logic ──▶ Output
             (parsing)       (handler)       (config/env)     (stdout/stderr)
```

## Design Principles

### 1. Separation of Concerns
- CLI parsing is separate from business logic
- Configuration management is isolated
- Each command is independently testable

### 2. Error Handling Strategy
- Use `anyhow::Result` for command-level error handling
- Define custom error types with `thiserror` for domain logic
- Provide user-friendly error messages with context

### 3. Testing Strategy
- Unit tests for individual functions
- Integration tests for command workflows
- Mocking for external dependencies (file system, environment variables)

### 4. Configuration Philosophy
- Single source of truth (`.stand.toml`)
- Explicit over implicit behavior
- Clear inheritance and override semantics

### 5. Security Considerations
- Never log sensitive variable values
- Validate file permissions for configuration files
- Use environment variable interpolation for secrets

## Extension Points

### Adding New Commands
1. Create new module in `src/commands/`
2. Add command definition to `src/cli/commands.rs`
3. Wire command in `src/main.rs`
4. Add comprehensive tests

### Adding Configuration Features
1. Extend data structures in `src/config/types.rs`
2. Update validation in `src/config/validator.rs`
3. Handle new features in relevant command handlers
4. Update configuration documentation

### Adding Shell Support
1. Extend shell detection in `src/shell/detector.rs`
2. Add shell-specific execution logic in `src/shell/executor.rs`
3. Test with target shell environment

## Dependencies

### Core Dependencies
- **clap**: CLI argument parsing and help generation
- **serde**: Serialization/deserialization for configuration
- **toml**: TOML format parsing
- **anyhow**: Error handling for applications
- **thiserror**: Error derivation for libraries

### Development Dependencies
- **tempfile**: Temporary directories for testing
- **assert_cmd**: CLI testing framework
- **predicates**: Assertion helpers for tests
- **serial_test**: Prevent test race conditions

### Optional Runtime Dependencies
- **colored**: Terminal color output
- **dirs**: System directory detection
- **which**: Executable location detection