# Claude Code Instructions

## Primary Reference

**🔗 All coding instructions are in [AGENTS.md](AGENTS.md)**

This file serves as a pointer to the main AI assistant instructions. For complete guidelines, workflow requirements, and development standards, refer to AGENTS.md.

## Quick Summary from AGENTS.md

### Language Usage
- Chat responses to user: **Japanese**
- Git commits/PRs/code comments: **English**

### Required Reading Order (from AGENTS.md)
1. **README.md** - Project overview
2. **docs/architecture.md** - System design
3. **docs/development-guideline.md** - TDD workflow
4. **docs/design.md** - Design decisions
5. **docs/requirements.md** - Feature specs
6. **docs/pr-review-guidelines.md** - PR checklist

### Critical Rules (from AGENTS.md)
- **TDD Mandatory**: RED → GREEN → REFACTOR
- **Never use `git add .`** - always specify files
- **Present implementation plan** before coding
- **Commit after each TDD cycle**

### Quick Commands (from docs/development-guideline.md)
```bash
cargo test                              # Run tests
cargo fmt && cargo clippy -- -D warnings && cargo test  # Full validation
```

---

**⚠️ Important**: This is only a summary. For complete instructions, workflow details, and all requirements, **read [AGENTS.md](AGENTS.md) first**.
