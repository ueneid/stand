# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

[0.2.0]: https://github.com/ueneid/stand/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/ueneid/stand/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/ueneid/stand/releases/tag/v0.1.0
