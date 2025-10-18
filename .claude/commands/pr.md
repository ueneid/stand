---
allowed-tools: Bash(git:*)
description: Generate PR description and automatically create pull request on GitHub
---

## Context

- Current git status: !`git status`
- Changes in this PR: !`git diff main...HEAD`
- Commits in this PR: !`git log --oneline main..HEAD`
- PR template: @.github/pull_request_template.md

## Your task

Based on the provided option, perform one of the following actions with github-mcp:

### Options:

- **No option or default**: Generate PR description and create pull request
- **-p**: Push current branch and create pull request
- **-u**: Update existing pull request description only

### Default behavior (no option):

1. Create a PR description following the **exact format** of the PR template in English
2. **Add a Mermaid diagram** that visualizes the changes made in this PR
3. Create a PR with the generated title and description

### With -p option:

1. Push current branch to remote repository using `git push -u origin <current-branch>`
2. Create a PR description following the **exact format** of the PR template in English
3. **Add a Mermaid diagram** that visualizes the changes made in this PR
4. Create a PR with the generated title and description

### With -u option:

1. Create a PR description following the **exact format** of the PR template in English
2. **Add a Mermaid diagram** that visualizes the changes made in this PR
3. Update existing pull request description

### Requirements:

1. Follow the template structure exactly
2. Use English for all content
3. Include specific implementation details
4. List concrete testing steps
5. Always include a Mermaid diagram that shows:
   - Architecture changes (if any)
   - Data flow modifications
   - Component relationships
   - Process flows affected by the changes
6. Be comprehensive but concise

### Mermaid Diagram Guidelines:

- Use appropriate diagram types (flowchart, sequence, class, etc.)
- Show before/after states if applicable
- Highlight new or modified components
- Use consistent styling and colors
- Add the diagram in a dedicated section of the PR description

**Generate the PR description and create the pull request automatically.**

