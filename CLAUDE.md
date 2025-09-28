# CLAUDE.md

## Required Reading Before ANY Coding

**STOP! Read these files IN ORDER before writing any code:**

1. **AGENTS.md** - Primary source of truth
2. **docs/development-guideline.md** - TDD workflow (mandatory)
3. **docs/design.md** - Architecture
4. **docs/requirements.md** - Features
5. **docs/pr-review-guidelines.md** - PR checklist

## Pre-coding Checklist

Before starting ANY task, confirm:
- [ ] I have read AGENTS.md completely
- [ ] I have read development-guideline.md completely  
- [ ] I will follow TDD (RED→GREEN→REFACTOR)
- [ ] I will write tests BEFORE implementation
- [ ] I will commit after each TDD cycle

## Critical Rules Summary

1. **Language Usage**:
   - Chat responses to user: **Japanese**
   - Terminal/CLI/TUI output: **Japanese**
   - Git commit messages: **English**
   - PR titles and descriptions: **English**
   - Code comments and documentation: **English**
2. **TDD is mandatory** - No production code without failing test first
3. **Implementation plan required** - Present plan before coding
4. **Never use `git add .`** - Always specify files
5. **Commit after each TDD cycle** - Use proper prefixes (test:, feat:, refactor:)

## Quick Reference

- **Config**: `.stand.toml` (TOML format, project root)
- **Tests**: `cargo test`
- **Format**: `cargo fmt`
- **Lint**: `cargo clippy -- -D warnings`
- **Coverage**: 80% minimum

## If Conflict Arises

Priority order:
1. AGENTS.md
2. docs/development-guideline.md
3. This file

---

**Remember**: When in doubt, refer to the source documents. Don't guess - read the actual guidelines.
