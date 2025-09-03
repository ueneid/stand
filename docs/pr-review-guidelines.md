# PR Review Guidelines

## Purpose
Provide a consistent, high‑quality review process for this Rust CLI (stand). Use this as a checklist; mark blockers vs nits.

## Quick Triage
- Scope/size: Is the PR focused and reasonably small?
- Tests: Do new/changed behaviors include tests? Do tests fail without the change?
- Local verification: `cargo fmt -- --check && cargo clippy -- -D warnings && cargo test`.

## Correctness & Tests
- Behavior matches requirements/design docs; CLI flags/usage align with `clap` definitions.
- Tests cover happy paths, edge cases, and failures (integration tests in `tests/`, unit tests in modules).
- No flakiness; deterministic assertions (avoid timing/network).

## Security
- No secrets in logs, errors, or fixtures; avoid printing env values by default.
- File access: validate paths; handle not-found, permission, and directory cases (see loader tests).
- Command execution: avoid shell injection; prefer argument vectors over string concatenation.
- Permissions: remind users to keep `.stand/*` files private; avoid overly permissive defaults.
- Error messages: informative without leaking sensitive data.

## Performance
- Algorithmic complexity is appropriate; no unnecessary clones/allocations in hot paths.
- Minimize blocking I/O on critical paths; batch filesystem operations when possible.
- Process spawning is deliberate (shell/exec commands) and not in tight loops.
- Benchmarks optional; at least reason about impact for large env files (100+ vars).

## Architecture & Design
- Respect layering (CLI → commands → core modules: environment/config/shell/state/process).
- Separation of concerns: thin command handlers; logic in domain modules.
- Clear, composable APIs; small functions with single responsibility.
- Error handling: `anyhow` at boundaries, typed errors (`thiserror`) in libraries with helpful context.

## Code Quality & Style
- Rust 2021; idiomatic patterns (iterators, `Option/Result`, `?`).
- Naming: `snake_case` for fns/modules, `CamelCase` for types, `SCREAMING_SNAKE_CASE` for constants.
- Formatting/linting clean: `cargo fmt`, `cargo clippy -- -D warnings`.
- No TODOs left; comments explain why, not what.

## UI/UX (CLI)
- Help text (`--help`) is clear, consistent, and matches behavior.
- Output is actionable and concise; errors suggest next steps.
- Colors used consistently via `utils/colors` (avoid overuse; ensure readability without color).
- Do not echo sensitive values in `show` unless explicitly requested.

## Documentation
- PR uses `.github/pull_request_template.md` (description, linked issues, tests, diagrams if helpful).
- Update `docs/design.md` or `docs/requirements.md` if behavior/architecture changes.
- Update `CLAUDE.md`/`AGENTS.md` if workflows or commands change.

## Maintainability
- Dependency changes are justified; minimal surface area.
- Backwards compatibility considered for CLI flags/subcommands.
- Clear migration notes for breaking changes.

## Ready‑to‑Merge Criteria
- Green on: `cargo fmt -- --check`, `cargo clippy -- -D warnings`, `cargo test`.
- Review comments addressed; no unresolved conversations.
- Commit history clean, meaningful (TDD-friendly: test/feat/refactor/docs/fix).
