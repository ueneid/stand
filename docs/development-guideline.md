# Development Workflow Guidelines

## 1. Overview

This document defines the mandatory development workflow for implementing the Stand project. All development must strictly follow these guidelines to ensure code quality, maintainability, and proper version control.

## 2. Development Process

### 2.1 Pre-Implementation Phase

**REQUIREMENT: Before writing any implementation code, create and present a detailed implementation plan.**

The implementation plan must include:
- Feature breakdown into small, testable units
- Test scenarios for each unit
- Expected interfaces and data structures
- Implementation order and dependencies
- Estimated completion checkpoints

Present this plan for review before proceeding with implementation.

### 2.2 Test-Driven Development (TDD)

**REQUIREMENT: Follow T-Wada style TDD strictly.**

The TDD cycle must be:

1. **RED Phase**
   - Write a failing test first
   - Run the test to confirm it fails
   - The test must fail for the right reason

2. **GREEN Phase**
   - Write the minimum code to make the test pass
   - Do not add unnecessary features
   - Focus only on making the current test pass

3. **REFACTOR Phase**
   - Improve code structure while keeping tests green
   - Remove duplication
   - Improve naming and clarity
   - Ensure all tests still pass after refactoring

**Rules:**
- Never write production code without a failing test
- Write one test at a time
- Tests should be small and focused on one behavior
- Commit after each complete RED-GREEN-REFACTOR cycle

### 2.3 Version Control Workflow

**REQUIREMENT: Follow GitHub Flow for all development.**

#### Initial Setup
```bash
# 1. Create a feature branch from main
git checkout main
git pull origin main
git checkout -b feature/[feature-name]

# Example branch names:
# feature/subshell-command
# feature/env-resolver
# feature/config-parser
```

#### During Development
```bash
# After each TDD cycle or logical unit of work
git add [files]
git commit -m "[type]: [description]"

# Commit message types:
# test: Add failing test for [feature]
# feat: Implement [feature] to pass test
# refactor: Improve [component] structure
# docs: Update [documentation]
# fix: Correct [issue]
```

#### Commit Guidelines

**REQUIREMENT: Make frequent, meaningful commits.**

- Commit after each complete TDD cycle
- Each commit should represent a working state (all tests pass)
- Commit messages must be clear and descriptive
- Group related changes in a single commit
- Never commit broken code to the branch

Example commit sequence:
```
test: Add test for loading dev environment
feat: Implement basic environment loading
refactor: Extract environment parser to separate module
test: Add test for variable override behavior
feat: Implement variable override in resolver
docs: Update design.md with resolver details
```

### 2.4 Implementation Completion Criteria

**REQUIREMENT: Implementation is complete only when ALL initially created tests are GREEN.**

Completion checklist:
- [ ] All tests from implementation plan are written
- [ ] All tests pass without modification
- [ ] Code coverage meets requirements (>80%)
- [ ] No TODO comments remain in code
- [ ] All functions have appropriate error handling
- [ ] Code passes linting and formatting checks

### 2.5 Documentation Updates

**REQUIREMENT: After implementation completion, update all relevant documentation.**

Documentation that must be reviewed and updated:
1. **CLAUDE.md** - Update if implementation details affect the project overview
2. **docs/requirements.md** - Mark completed requirements
3. **docs/design.md** - Update if implementation revealed design changes
4. **README.md** - Update usage examples if needed
5. **API documentation** - Generate/update from code comments

## 3. Workflow Example

Here's a complete example for implementing the `stand shell` command:

### Step 1: Implementation Plan
```markdown
## Implementation Plan: stand shell command

### Test Scenarios:
1. Test shell command creates subshell with environment
2. Test nested shell detection prevents double nesting
3. Test environment variables are correctly injected
4. Test prompt shows environment name
5. Test exit returns to original shell state

### Implementation Order:
1. Shell detection module
2. Environment resolver
3. Subshell spawner
4. Prompt formatter
5. Integration with CLI
```

### Step 2: TDD Implementation
```bash
# First cycle
git checkout -b feature/shell-command

# Write failing test
# ... code test file ...
git add tests/shell_test.rs
git commit -m "test: Add test for shell detection"

# Make it pass
# ... implement shell detection ...
git add src/shell/mod.rs
git commit -m "feat: Implement shell type detection"

# Refactor if needed
# ... improve code ...
git add src/shell/mod.rs
git commit -m "refactor: Extract shell detection to separate function"

# Continue for each test scenario...
```

### Step 3: Completion Verification
```bash
# Run all tests
cargo test

# Check coverage
cargo tarpaulin --out Html

# Verify all planned tests exist and pass
```

### Step 4: Documentation Update
```bash
# Update documentation
# ... edit CLAUDE.md, docs/*, README.md ...
git add CLAUDE.md docs/ README.md
git commit -m "docs: Update documentation for shell command"

# Push branch
git push origin feature/shell-command
```

## 4. Quality Gates

Before considering any feature complete:

1. **Test Coverage**: Minimum 80% code coverage
2. **Test Quality**: All tests are meaningful and test behavior, not implementation
3. **Code Quality**: No compiler warnings, passes clippy lints
4. **Documentation**: All public APIs are documented
5. **Commits**: Clean, logical commit history

## 5. Prohibited Practices

The following practices are strictly forbidden:

- ❌ Writing code before tests
- ❌ Committing failing tests to main
- ❌ Skipping the refactor phase
- ❌ Large, monolithic commits
- ❌ Commits with message like "WIP" or "fix"
- ❌ Merging without all tests passing
- ❌ Leaving console.log or debug prints in code
- ❌ Ignoring test failures with `#[ignore]`

## 6. Tools and Commands

### Essential Commands
```bash
# Run tests
cargo test

# Run specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture

# Check code coverage
cargo tarpaulin --out Html

# Format code
cargo fmt

# Lint code
cargo clippy -- -D warnings

# Build and test
cargo build && cargo test
```

## 7. Continuous Improvement

After each feature implementation:
1. Review what went well
2. Identify what could be improved
3. Update this workflow document if needed
4. Share learnings in commit messages or PR description

---

**Remember**: The goal is not just working code, but maintainable, well-tested, and well-documented code that follows best practices throughout the entire development process.

