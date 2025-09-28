# AI Assistant Instructions

## AI Assistant Communication Language
- **Chat responses to user**: Japanese

## Development Artifact Language Standards
- **Git commit messages**: English
- **PR titles and descriptions**: English
- **Code comments and documentation**: English

## Development Workflow Guidelines

### Test-Driven Development (TDD)
- **Mandatory TDD Cycle**: RED → GREEN → REFACTOR
- No production code without failing test first
- Write tests BEFORE implementation
- Commit after each TDD cycle

### Coding Style & Conventions
- **Language**: Rust (edition 2021); 4-space indentation
- **Naming**: `snake_case` for functions/modules, `CamelCase` for types, `SCREAMING_SNAKE_CASE` for constants and env keys
- **Formatting/Linting**: Use `rustfmt` and `clippy` before pushing
- **Errors**: Prefer `anyhow::Result` at boundaries; define library errors with `thiserror`

### Testing Standards
- **Framework**: Built-in Rust tests; integration tests in `tests/` and unit tests behind `#[cfg(test)]` in modules
- **Naming**: Describe behavior (e.g., `test_cli_shows_help`, `test_parse_variable_expansion`)
- **Environment Variable Tests**: Use `#[serial]` attribute to prevent race conditions
- **Coverage**: 80% minimum coverage target

### Commit & Pull Request Guidelines
- **Commits**: Small, focused; follow TDD cycle. Prefix types: `test:`, `feat:`, `refactor:`, `docs:`, `fix:`
- **Git Rules**: Never use `git add .` - always specify files explicitly
- **PRs**: Use `.github/pull_request_template.md`. Include: clear description, linked issue (`Closes #123`), key changes, tests added, and local results (`cargo fmt`, `cargo clippy -- -D warnings`, `cargo test`)

### Security Guidelines
- Never print sensitive values; prefer masked logs
- Do not commit secrets to repository
- Ensure reasonable file permissions for configuration files (0600 for sensitive data)

## Required Reading Order
Before starting ANY coding task, read these files in order:
1. **README.md** - Project overview and setup instructions
2. **docs/architecture.md** - High-level system design
3. **docs/development-guideline.md** - Detailed TDD workflow
4. **docs/design.md** - Detailed design decisions
5. **docs/requirements.md** - Feature specifications
6. **docs/pr-review-guidelines.md** - PR review checklist

## Implementation Planning
- Always present implementation plan before coding
- Break down complex tasks into smaller steps
- Use TodoWrite tool to track progress throughout implementation

