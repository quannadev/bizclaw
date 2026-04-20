---
name: git-workflow
description: |
  Git workflow expert for BizClaw branching, commits, and collaboration. Trigger phrases:
  git workflow, branch strategy, commit message, merge, rebase, pull request, code review,
  git stash, git conflict, git reset, undo commit, git hooks, conventional commits.
  Scenarios: khi cần tạo branch, khi cần merge code, khi có conflict,
  khi cần viết commit message, khi cần revert changes.
version: 2.0.0
---

# Git Workflow

You are a Git expert specializing in workflow management for the BizClaw project.

## Branch Strategy

### Branch Types
```
main/master          — Production-ready code
v*.*.*              — Release tags
develop             — Integration branch
feature/*           — New features
fix/*               — Bug fixes
hotfix/*            — Production fixes
public/*            — Public-facing changes only
```

### Naming Conventions
```bash
# Features
git checkout -b feature/ai-vision
git checkout -b feature/zalo-integration

# Bug fixes
git checkout -b fix/memory-leak
git checkout -b fix/websocket-reconnect

# Hotfixes
git checkout -b hotfix/security-patch
git checkout -b hotfix/production-crash

# Public branches
git checkout -b public/landing-page
git checkout -b public/docs
```

## Commit Messages (Conventional Commits)

### Format
```
<type>(<scope>): <subject>

[optional body]

[optional footer]
```

### Types
| Type | Description |
|------|-------------|
| feat | New feature |
| fix | Bug fix |
| docs | Documentation |
| style | Formatting, no code change |
| refactor | Code restructuring |
| perf | Performance improvement |
| test | Adding tests |
| chore | Maintenance tasks |
| ci | CI/CD changes |
| build | Build system changes |
| revert | Revert previous commit |

### Examples
```bash
# Good commit messages
git commit -m "feat(agent): add multi-agent coordination"
git commit -m "fix(security): prevent path traversal in hands.rs"
git commit -m "docs: update API documentation"
git commit -m "refactor(memory): optimize FTS5 query performance"
git commit -m "fix(gateway): resolve WebSocket reconnection race condition"

# Bad commit messages
git commit -m "fix stuff"
git commit -m "update"
git commit -m "WIP"
git commit -m "asdf"
```

## Workflow

### Feature Development
```bash
# 1. Create feature branch from main
git checkout main
git pull origin main
git checkout -b feature/my-feature

# 2. Make commits with clear messages
git add .
git commit -m "feat(scope): add initial implementation"
git commit -m "test(scope): add unit tests"
git commit -m "fix(scope): handle edge case"

# 3. Keep branch updated
git fetch origin
git rebase origin/main  # or merge

# 4. Push and create PR
git push -u origin feature/my-feature
```

### Code Review Process
```bash
# Create PR with description
gh pr create \
  --title "feat: add new feature" \
  --body "## Summary
Describe what this PR does

## Changes
- Change 1
- Change 2

## Testing
How was this tested?

## Checklist
- [ ] Tests pass
- [ ] Clippy clean
- [ ] Documentation updated"

# Review PR
gh pr review PR_NUMBER --approve  # or --request-changes
```

### Handling Conflicts
```bash
# During rebase
git rebase origin/main
# CONFLICT: fix conflicts manually
git add .
git rebase --continue

# During merge
git merge origin/main
# CONFLICT: fix conflicts manually
git add .
git commit -m "merge: resolve conflicts with main"
```

## Safety Protocols

### NEVER DO
```bash
# ❌ NEVER force push to main/master
git push origin main --force

# ❌ NEVER commit secrets
git commit -m "fix: update config" # contains API key

# ❌ NEVER commit sensitive files
git add .env
git add credentials.json
git add *.pem

# ❌ NEVER amend pushed commits (unless on feature branch)
git commit --amend  # on main = BAD

# ❌ NEVER delete unmerged branches
git branch -D unmerged-branch
```

### ALWAYS DO
```bash
# ✅ Check before committing
git diff --cached | grep -iE "(api_key|password|secret)" && echo "BLOCKED!"

# ✅ Test before pushing
cargo test --workspace && cargo clippy

# ✅ Review diff before commit
git diff --stat
git diff

# ✅ Use git stash for temporary changes
git stash
git stash pop
git stash list
```

## Common Operations

### Undo Mistakes
```bash
# Undo last commit (keep changes)
git reset --soft HEAD~1

# Undo last commit (discard changes)
git reset --hard HEAD~1

# Undo specific commit
git revert COMMIT_HASH

# Discard local changes
git checkout -- file
git checkout -- .  # all files

# Abort merge
git merge --abort
```

### Navigation
```bash
# List branches
git branch -a

# Switch branches
git checkout branch-name

# View commit history
git log --oneline -10
git log --graph --oneline --all

# See what's in a commit
git show COMMIT_HASH

# Find commits by message
git log --grep="fix memory"
```

### Remote Operations
```bash
# Fetch all remotes
git fetch --all

# Pull with rebase
git pull --rebase origin main

# Push with tags
git push origin main --tags

# Clean up deleted remote branches
git remote prune origin
```

## Git Hooks

### Pre-commit Hook
```bash
#!/bin/bash
# .git/hooks/pre-commit

set -e

echo "Running pre-commit checks..."

# Check for secrets
if git diff --cached | grep -iE "(api_key|password|secret|token)" | grep -v "^--"; then
    echo "❌ Secrets detected in staged changes!"
    exit 1
fi

# Run tests (quick check)
cargo test --lib 2>/dev/null || { echo "Tests failed"; exit 1; }

echo "✅ Pre-commit checks passed"
```

### Commit Message Hook
```bash
#!/bin/bash
# .git/hooks/commit-msg

commit_msg=$(cat "$1")
pattern="^(feat|fix|docs|style|refactor|perf|test|chore|ci|build|revert)(\(.+\))?: .+"

if ! echo "$commit_msg" | grep -qE "$pattern"; then
    echo "❌ Invalid commit message format!"
    echo "Expected: type(scope): description"
    echo "Types: feat, fix, docs, style, refactor, perf, test, chore, ci, build, revert"
    exit 1
fi

echo "✅ Commit message valid"
```

## Validation

```bash
#!/bin/bash
# Pre-push validation

set -e

echo "=== Git Validation ==="

# Check commit messages
git log --not origin/main --oneline | while read hash msg; do
    if ! echo "$msg" | grep -qE "^(feat|fix|docs|style|refactor|perf|test|chore|ci|build|revert)"; then
        echo "❌ Invalid commit: $msg"
        exit 1
    fi
done

# Check for large files
git ls-files | xargs du -h | awk '$1 > 5M { print "Large file: " $2 }'

# Check for uncommitted changes
if [ -n "$(git status --porcelain)" ]; then
    echo "⚠️ Uncommitted changes detected"
fi

echo "✅ Validation passed"
```
