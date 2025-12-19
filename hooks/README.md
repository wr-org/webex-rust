# Git Hooks for webex-rust

This directory contains git hooks to maintain code quality and consistency.

## Available Hooks

### pre-commit

Automatically runs `cargo fmt` before each commit to ensure all code is properly formatted.

**What it does:**
- Checks if code formatting is required
- Runs `cargo fmt --all` if needed
- Automatically adds formatted files to the commit
- Prevents commits with formatting issues

## Installation

To install the hooks, run:

```bash
./hooks/install.sh
```

Or manually:

```bash
cp hooks/pre-commit .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit
```

## Uninstalling

To remove the hooks:

```bash
rm .git/hooks/pre-commit
```

## Bypassing Hooks

If you need to bypass the hooks temporarily (not recommended):

```bash
git commit --no-verify
```

## CI/CD

The CI pipeline in `.github/workflows/` runs the same checks, so even if hooks are bypassed locally, the CI will catch formatting issues.
