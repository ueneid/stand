Branch Re‑Review — feature/toml-single-file-config vs main

Scope: Re‑review of the branch after addressing prior comments, following docs/pr-review-guidelines.md. Focused files: docs/design.md, docs/requirements.md, src/config/{types.rs, loader.rs, validator.rs}, tests/toml_*.rs.

Summary
- Documentation now explicitly defines configuration discovery precedence and legacy deprecation, and documents interpolation rules. This resolves the main doc gap from the previous review.
- Implementation for TOML single‑file configuration, strict interpolation, and variable inheritance remains solid and unchanged in behavior.
- Legacy YAML loader is still present for compatibility; TOML path is the default per docs.

What Changed Since Last Review (Addressed)
- Discovery precedence documented: .stand.toml preferred, .stand/config.yaml as legacy fallback, with a deprecation timeline (docs/design.md 5.3; docs/requirements FR‑4.6–4.8). Good clarity.
- Interpolation behavior documented: single‑pass, ${VAR} only, explicit errors for unterminated/empty placeholders. Matches implementation.

Outstanding Recommendations (Non‑Blocking)
1) Validation helper for TOML path
   - Add load_config_toml_with_validation mirroring YAML parity (validate_required_fields → validate_environment_references → validate_no_circular_references → validate_common_config). This keeps call sites symmetric and reduces misuse.

2) Parse error typing
   - Consider adding TomlError { source: toml::de::Error } or a unified ParseError (enum or generic) in ConfigError, instead of mapping TOML parse failures to ValidationError. Improves diagnosability while preserving typed errors.

3) Test type duplication
   - tests/toml_config_tests.rs declares local Configuration/Environment/Settings. Prefer using stand::config::types::Configuration to avoid drift from library types.

4) Deterministic ordering (optional)
   - If stable iteration/serialization order of variables is important, consider IndexMap for Configuration.common and Environment.variables (indexmap is already a dependency elsewhere). Not a blocker.

Correctness & Tests
- Types: Flattened environment variables via #[serde(flatten)] continue to parse as expected; common variables are merged at lowest priority; color/requires_confirmation inherit if unset in child.
- Loader: load_config_toml + load_config_toml_with_inheritance are straightforward; interpolation remains strict and safe (errors on unterminated/empty names; exposes variable name only).
- Validator: Required fields, reference checks, and cycle detection remain format‑agnostic and appropriate. Common keys/values non‑empty check is reasonable.
- Tests: TOML parsing and inheritance chains covered; circular detection is validated; YAML legacy tests remain untouched and are fine alongside the migration.

Security
- Interpolation errors do not leak values; only variable names. No new risks identified.

Performance
- HashMap cloning in merges is acceptable for expected config sizes and non‑hot code paths. No performance concerns.

Documentation
- With discovery precedence and deprecation timeline added, the docs now align with the implementation direction. Consider a brief “migration tips” snippet in README/CLI help in a separate PR for user visibility (optional).

Ready‑to‑Merge
- Functionality and documentation are in a good state. The remaining items are nits/quality improvements and not blocking.
- Please ensure green on: cargo fmt -- --check, cargo clippy -- -D warnings, cargo test.

Overall Assessment
The branch is ready to merge. The documentation additions address the critical clarity gap. Follow‑ups (TOML validation helper, parse error variant, test type reuse) can be tackled incrementally without blocking this merge.

