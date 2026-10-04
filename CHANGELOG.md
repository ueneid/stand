# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Breaking

- Decryption now requires an interactive terminal (`/dev/tty`).
- `STAND_PRIVATE_KEY` is only accepted with `CI=true`/`1` or `STAND_KEY_SOURCE=env`. Otherwise Stand refuses with an error.
- `.stand.keys` is deprecated. It still works but prints a warning. Run `stand encrypt migrate`.
- `stand env` no longer decrypts. It reads values from the current shell's environment.
- `stand set` refuses reserved keys (`description`, `extends`, `color`, `requires_confirmation`) and blocked variable names.
- `stand shell` and `stand exec` refuse to inject blocked variable names (checked before confirmation and decryption), and warn about risky ones.

### Added

- `stand unset <environment> <key>` removes a variable from an environment.
- Shell reflect: inside `stand shell` (bash, zsh, fish), `stand set` and `stand unset` update the current shell immediately.
- `stand encrypt enable [--touch-id]`, `stand encrypt migrate`, `stand encrypt add-touch-id`, and an updated `stand encrypt disable`.
- Touch ID unlock through a Secure Enclave key (`age-plugin-se`, macOS 14+).
- Variable name guards: names must match `^[A-Za-z_][A-Za-z0-9_]*$`. Hook-planting names are refused (ASCII case-insensitive). Risky names (`PATH`, `NODE_OPTIONS`, `NODE_PATH`, `JAVA_TOOL_OPTIONS`, ...) produce a warning.
- `stand set --encrypt` warns if the project's public key matches no local identity.
- Wrapped keys are bound to their project (`# project: <path>`). Read-only decryption asks for confirmation on a mismatch; key-writing operations require an exact match.
- `stand encrypt disable` deletes identity files only after a binding-verified unlock; otherwise it keeps them and prints their paths. `stand encrypt migrate` refuses if `<public_key>.age` exists or if `.stand.keys` is a symlink or has extra hard links.
- Prompt text is sanitized (control, bidi and zero-width characters are escaped) and the banner shows the resolved `age-plugin-se` path before Touch ID.
- `docs/security.md`: threat model, setup, migration, CI, sandbox advice, residual risks.

### Changed

- The private key is stored under `$XDG_CONFIG_HOME/stand/identities/` (default `~/.config/stand/identities/`), wrapped by a passphrase and optionally Touch ID. The format of encrypted values and `[encryption].public_key` is unchanged.
- Unlock order: `STAND_PRIVATE_KEY` (CI only), legacy `.stand.keys`, Touch ID, passphrase.
- `stand init --encrypt` no longer writes `.stand.keys`.
- Temp directories for bash/zsh init are random and `0700`.
- `docs/security.md` is included in the published crate.

### Fixed

- Documentation named a `show` command; the command is `inspect`. Removed documented environment variables (`STAND_CONFIG`, `STAND_NO_COLOR`, `STAND_QUIET`) and exit codes that the code never implemented.

### Security

- Each decryption is a deliberate human act and shows which command decrypts how many secrets for which environment.
- Config values can no longer set variables such as `BASH_ENV`, `PROMPT_COMMAND`, `LD_*` or `DYLD_*` in the interactive shell.
- Secrets in use remain readable by any process of the same user. See `docs/security.md`.

## [0.2.2] - 2026-05-17

### Changed

- Updated dependencies to current major versions:
  - `thiserror` 1 → 2
  - `toml` 0.8 → 1.1
  - `toml_edit` 0.22 → 0.25
  - `colored` 2 → 3
  - `age` 0.10 → 0.11

### Internal

- Added a hardened CI workflow (lint, test matrix, `cargo-deny`,
  `cargo-machete`).
- Added Dependabot configuration with supply-chain hardening.

## [0.2.1] - 2026-05-12

### Changed

- **MSRV is now Rust 1.88** (previously undeclared). Declared via
  `rust-version` in `Cargo.toml` so `cargo install stand` fails fast with
  a clear error on older toolchains instead of failing mid-build when a
  transitive dependency (e.g. `home@0.5.12`) demands a newer rustc.

## [0.2.0] - 2026-01-16

### Added

- **Encryption support**: Encrypt sensitive values in configuration files using age encryption (X25519 + ChaCha20-Poly1305)
  - `stand encrypt enable` - Enable encryption and generate key pair
  - `stand encrypt disable` - Disable encryption and decrypt all values
  - `stand set <env> <key> <value> --encrypt` - Set an encrypted variable
  - `stand get <env> <key>` - Get a variable (automatically decrypts if encrypted)
  - `stand init --encrypt` - Initialize with encryption enabled
- Automatic decryption of encrypted values when using `stand shell` and `stand exec`
- `[ENCRYPTED]` marker displayed for encrypted values in `stand inspect`
- Automatic decryption in `stand env` output

### Changed

- **BREAKING**: `stand show <env>` command renamed to `stand inspect <env>`
  - Migration: Replace `stand show` with `stand inspect` in your scripts and workflows

## [0.1.1] - 2026-01-12

### Changed

- Updated documentation and crate-level docs for crates.io
- Added auto-exit subshell when leaving project directory

## [0.1.0] - 2025-08-25

### Added

- Initial release
- `stand init` - Initialize Stand in the current directory
- `stand shell <env>` - Start a subshell with environment variables
- `stand exec <env> <command>` - Execute a command with environment variables
- `stand list` - List available environments
- `stand show <env>` - Show environment variables for an environment
- `stand validate` - Validate configuration
- `stand current` - Show current active environment
- `stand env` - Show environment variables in the current Stand subshell
- Environment inheritance with `extends`
- Common variables support
- Color-coded prompts
- Nested shell prevention
- Confirmation prompts for protected environments

[0.2.2]: https://github.com/ueneid/stand/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/ueneid/stand/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/ueneid/stand/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/ueneid/stand/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/ueneid/stand/releases/tag/v0.1.0
