# AI Assistant Instructions

## Language Usage Guidelines
- **Chat responses to user**: Japanese
- **Terminal/CLI/TUI output**: Japanese
- **Git commit messages**: English
- **PR titles and descriptions**: English
- **Code comments and documentation**: English

# Repository Guidelines

## Project Structure & Module Organization
- `src/`: Rust sources organized by domain (`cli/`, `commands/`, `environment/`, `config/`, `shell/`, `state/`, `utils/`). Entry points: `src/main.rs`, `src/lib.rs`.
- `tests/`: Integration tests (e.g., `cli_tests.rs`, `env_*_tests.rs`).
- `docs/`: Design, requirements, and workflow docs.
- `.github/`: Pull request template and checks.
- `Cargo.toml`: Dependencies, metadata; Rust 2021 edition.

## Build, Test, and Development Commands
- `cargo build`: Debug build; compiles the CLI.
- `cargo run -- <subcommand>`: Run locally (e.g., `cargo run -- list`).
- `cargo test`: Run unit and integration tests.
- `cargo fmt` / `cargo fmt -- --check`: Format / verify formatting.
- `cargo clippy -- -D warnings`: Lint; treat warnings as errors.
- `cargo build --release`: Optimized binary.

## Coding Style & Naming Conventions
- **Language**: Rust (edition 2021); 4‑space indentation.
- **Naming**: `snake_case` for functions/modules, `CamelCase` for types, `SCREAMING_SNAKE_CASE` for constants and env keys.
- **Formatting/Linting**: Use `rustfmt` and `clippy` before pushing.
- **Errors**: Prefer `anyhow::Result` at boundaries; define library errors with `thiserror`.

## Testing Guidelines
- **Framework**: Built‑in Rust tests; integration tests in `tests/` and unit tests behind `#[cfg(test)]` in modules.
- **Naming**: Describe behavior (e.g., `test_cli_shows_help`, `test_parse_variable_expansion`).
- **Running**: `cargo test`, or a single test `cargo test <name>`.
- **Coverage**: Optional `cargo tarpaulin --out Html` (if installed).

## Commit & Pull Request Guidelines
- **Commits**: Small, focused; follow TDD cycle (RED→GREEN→REFACTOR). Prefix types: `test:`, `feat:`, `refactor:`, `docs:`, `fix:`.
- **PRs**: Use `.github/pull_request_template.md`. Include: clear description, linked issue (`Closes #123`), key changes, tests added, and local results (`cargo fmt`, `cargo clippy -- -D warnings`, `cargo test`). Add Mermaid diagram where useful.

## Security & Configuration Tips
- Single config file lives at project root: `.stand.toml` (TOML format) containing all configuration and environment variables. Do not commit secrets; add `.stand.toml` file to `.gitignore` if it contains sensitive data.
- Never print sensitive values; prefer masked logs.
- Ensure reasonable file permissions for `.stand.toml` file (0600 for sensitive data).

### Example .stand.toml file structure:
```toml
version = "2.0"

[settings]
default_environment = "dev"
show_env_in_prompt = true

# Common variables shared across all environments
[common]
APP_NAME = "MyApp"
LOG_FORMAT = "json"

# Environment-specific variables
[environments.dev]
description = "Development environment"
color = "green"
DATABASE_URL = "postgres://localhost:5432/dev"
DEBUG = "true"

[environments.prod]
description = "Production environment"
color = "red"
extends = "dev"  # Inherits from dev environment
requires_confirmation = true
DATABASE_URL = "postgres://prod.example.com/myapp"
DEBUG = "false"
```

## Workflow Reference
- This repo enforces TDD and GitHub Flow. See `docs/development-guideline.md` for required steps before implementation.

