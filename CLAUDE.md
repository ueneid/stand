# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Stand is a Rust-based CLI tool for managing environment variables across multiple configurations within a single project directory. It provides explicit control over environment switching to prevent accidental misconfigurations.

For detailed requirements and design, see:
- [Requirements Document](docs/requirements.md) - User stories, functional/non-functional requirements
- [Design Document](docs/design.md) - Architecture, technology stack, data models

## Development Workflow

**IMPORTANT**: This project follows strict TDD (Test-Driven Development) and GitHub Flow. See [Development Guidelines](docs/development-guideline.md) for mandatory workflow requirements.

### Key Requirements:
1. **Pre-Implementation**: Create and present detailed implementation plan before coding
2. **TDD Cycle**: Follow RED-GREEN-REFACTOR strictly (T-Wada style)
3. **Version Control**: Use GitHub Flow with feature branches
4. **Commits**: Make frequent, meaningful commits after each TDD cycle
5. **Completion**: Implementation is complete only when ALL tests are GREEN

## Development Commands

### Build
```bash
# Development build
cargo build

# Release build with optimizations
cargo build --release

# Build all targets
cargo build --all-targets
```

### Test
```bash
# Run all tests
cargo test

# Run a specific test
cargo test test_name

# Run with verbose output
cargo test -- --nocapture

# Run integration tests only
cargo test --test '*'

# Check code coverage
cargo tarpaulin --out Html
```

### Lint and Format
```bash
# Format code
cargo fmt

# Check formatting (CI)
cargo fmt -- --check

# Run clippy linter
cargo clippy

# Strict linting (treat warnings as errors)
cargo clippy -- -D warnings

# Build and test together
cargo build && cargo test
```

### Debug
```bash
# Run with debug output
STAND_DEBUG=1 cargo run -- [command]

# Run with backtrace
RUST_BACKTRACE=1 cargo run -- [command]

# Run debug build
cargo run -- [command]
```

## Code Structure

The codebase follows a modular architecture with clear separation of concerns:

- `src/main.rs` - Entry point and CLI setup
- `src/cli/` - Command-line interface and argument parsing
- `src/commands/` - Command handlers for each subcommand
- `src/environment/` - Environment variable resolution and management
- `src/config/` - Configuration file parsing and validation
- `src/shell/` - Shell-specific integrations and prompt handling
- `src/state/` - Runtime state persistence

## Implementation Standards

### TDD Requirements
- **Never write code before tests** - All production code must be driven by failing tests
- Follow RED-GREEN-REFACTOR cycle strictly
- Write one test at a time, focused on single behavior
- Commit after each complete TDD cycle
- Minimum 80% test coverage required

### Git Workflow
- Use feature branches from main: `feature/[feature-name]`
- Commit messages must follow format: `[type]: [description]`
- Types: test, feat, refactor, docs, fix
- Make frequent, meaningful commits (never broken code)
- Each commit should represent working state with passing tests

### Code Quality Gates
Before any feature is complete:
- [ ] All planned tests written and GREEN
- [ ] Code coverage >80%
- [ ] No compiler warnings
- [ ] Passes `cargo clippy -- -D warnings`
- [ ] All public APIs documented
- [ ] No TODO comments in code

### Error Handling
- Use `anyhow::Result` for error propagation with context
- Provide actionable error messages with suggested fixes
- Validate configuration early and fail fast

### Testing Approach
- Unit tests for individual functions in module files
- Integration tests in `tests/` directory
- Test coverage for configuration parsing, variable resolution, and shell interactions
- Tests must test behavior, not implementation details

### Performance Considerations
- Lazy loading of configuration files
- Minimal allocations in hot paths
- Single binary distribution for fast startup

### Security
- Never log sensitive environment variable values
- Automatically update `.gitignore` for sensitive files
- Validate file permissions on configuration files