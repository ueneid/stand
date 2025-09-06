Branch Review — feature/toml-single-file-config vs main

This review follows docs/pr-review-guidelines.md and focuses on the diff between main and feature/toml-single-file-config. Key changed files: Cargo.toml, docs/design.md, docs/requirements.md, src/config/{types.rs, loader.rs, validator.rs}, and new tests under tests/toml_*.rs.

Summary
- Moves configuration to a single TOML file (.stand.toml) with inline variables per environment.
- Adds TOML loader functions and variable inheritance (common → parent → child).
- Keeps legacy YAML loader for backward compatibility (segregated paths, some legacy utilities commented with TODOs).
- Expands tests to cover TOML parsing, interpolation, and inheritance chains.

Quick Triage
- Scope/size: Focused migration toward TOML single-file config. Reasonable PR size.
- Tests: New tests for TOML parsing and inheritance are present and meaningful.
- Local verification: Please confirm green on cargo fmt -- --check, cargo clippy -- -D warnings, cargo test.

Correctness & Tests
- Types (src/config/types.rs):
  - Configuration.common now Option<HashMap<String, String>> (shared variables).
  - Environment flattens all key=value items into variables: HashMap<String, String> via #[serde(flatten)].
  - color and requires_confirmation remain per environment; extends is used for inheritance.
- Loader (src/config/loader.rs):
  - load_config_toml reads .stand.toml, parses via toml, and interpolates ${VAR} in descriptions and variable values with strict erroring for unterminated/empty placeholders.
  - load_config_toml_with_inheritance applies two-stage inheritance: (1) merge common variables into each env (lowest priority), (2) merge through extends chain (parent first, child overrides). color and requires_confirmation inherit if unset in child.
  - Legacy YAML paths remain via load_config and friends; file-based validations/merges are commented with TODO noting TOML shift.
- Validator (src/config/validator.rs):
  - Required fields: version, non-empty environments, non-empty settings.default_environment, non-empty env.description.
  - Reference checks: default_environment exists; extends references exist; circular references detected.
  - Common validation: non-empty keys and values in common map.
- Tests:
  - tests/toml_parser_tests.rs covers parsing and basic structure.
  - tests/toml_inheritance_tests.rs covers common merge, parent→child overrides, circular detection, multi-hop inheritance.
  - tests/toml_config_tests.rs validates shape and example serialization; note it defines local structs mirroring library types (see “Nits”).

Security
- Interpolation errors reveal only variable names, not values; good.
- File-based validation is deprecated in the TOML path (variables are in config, not separate files). Legacy errors (FileNotFound/NotAFile) still exist in ConfigError; safe to keep for backward compatibility, but consider deprecating with clear docs.

Performance
- Inheritance merges clone HashMaps. For typical config sizes, overhead is negligible and not on a hot path.
- If deterministic ordering of variables is desired for UX/serialization, IndexMap is already a dependency and could be adopted (non-blocking).

Architecture & Design
- Clear separation: validator handles structural checks; loader orchestrates parsing, interpolation, inheritance.
- Coexistence of TOML and YAML loaders is acceptable during migration. Recommend documenting discovery order and deprecation timeline (see “Docs”).

Code Quality & Style
- Idiomatic Rust 2021; consistent error handling in library (ConfigError), with anyhow reserved for CLI boundaries.
- TODO markers on legacy utilities are clear. Avoid long-term commented code; track removals via issues.

Documentation
- docs/design.md and docs/requirements.md updated toward TOML single-file. Good improvement over previous mismatch.
- Please add a short “Configuration discovery and migration” section:
  - Discovery order: prefer .stand.toml; optionally fall back to legacy .stand/config.yaml (if present) with deprecation warning.
  - Migration guidance: how to convert multi-file YAML setups to TOML (examples of [common], [environments.<name>] blocks, extends semantics, interpolation rules).
  - Interpolation behavior: single-pass, ${VAR} only, error on unterminated/empty names; no nested expansion.

Maintainability
- Dependencies: adds toml crate; justified and minimal.
- Backward compatibility: legacy YAML loader retained. Provide a deprecation note and a future removal target version to keep surface area small.

Ready-to-Merge Criteria
- Code quality is solid and tests are thorough for TOML flow.
- Please ensure CI runs fmt/clippy/tests green.
- Not blocking, but recommended before merge: add an explicit “TOML validation” convenience (see below) and doc the discovery order.

Blocking vs Non-Blocking
- Blocking: None identified for functionality introduced by this branch.
- Non-Blocking / Nits:
  1) Add a load_config_toml_with_validation helper that reuses validator.rs checks (parity with YAML path).
  2) Consider a dedicated ConfigError variant for TOML parse (e.g., TomlError { source }) instead of mapping to ValidationError; or rename YamlError to ParseError to cover both.
  3) tests/toml_config_tests.rs defines local structs; prefer using stand::config::types::Configuration to avoid divergence.
  4) Document configuration discovery order and deprecation of YAML (.stand/config.yaml) with an explicit timeline/warning behavior.
  5) Optional: If stable output ordering matters, switch HashMap → IndexMap for variables/common to ensure deterministic serialization and UX.

File-specific Notes
- Cargo.toml: Adds toml = "0.8"; indexmap already present (consider using for stable ordering).
- src/config/types.rs: New flattened variables map is a good fit for TOML. If you need constrained key spaces later, document reserved keys or add a namespace to avoid collisions with color/extends.
- src/config/loader.rs: New TOML loaders are clear; interpolation is strict and safe. Provide load_config_toml_with_validation for symmetry; keep legacy YAML code behind a clear migration plan.
- src/config/validator.rs: Comprehensive and format-agnostic. Good separation and coverage (required fields, references, cycles, common).
- tests/toml_parser_tests.rs / tests/toml_inheritance_tests.rs: Meaningful coverage for parsing and inheritance. Nice chain test and circular detection.
- tests/toml_config_tests.rs: Helpful parsing/serialization checks; consider switching to library types to reduce duplication risk.

Recommendations (Actionable)
1) Add load_config_toml_with_validation that calls:
   - load_config_toml → validate_required_fields → validate_environment_references → validate_no_circular_references → validate_common_config.
2) Update docs to include discovery precedence and a short deprecation note for YAML with suggested migration steps.
3) Consider error variant unification: replace YamlError with ParseError (serde_yaml::Error | toml::de::Error) or add TomlError; keep ValidationError for logical checks.
4) Replace local test structs with library types in tests/toml_config_tests.rs (optional but recommended).
5) Optional: Adopt IndexMap for deterministic ordering of variables/common if you plan to display/serialize in a stable order.

Overall Assessment
The branch cleanly introduces TOML single-file configuration with robust interpolation and inheritance, plus strong validation and tests. With minor polish (TOML validation helper and doc clarifications), it is ready to merge.

