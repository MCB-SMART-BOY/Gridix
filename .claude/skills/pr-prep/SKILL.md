---
name: pr-prep
description: Run the full pre-PR checklist. Use when asked to prepare a PR, check code before pushing, run pre-merge checks, or verify changes pass CI.
paths:
  - src/**/*.rs
  - tests/**/*.rs
  - Cargo.toml
  - docs/**/*.md
---

# Pre-PR checklist

Run each check in order. Stop on first failure.

## 1. Format

```bash
cargo fmt --check
```

Fail → `cargo fmt` then re-check.

## 2. Lint (strict)

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

Warnings are errors.

## 3. Test

```bash
cargo test
```

~620 tests, all must pass. MySQL integration tests are `#[ignore]`d — expected.

## 4. Doc links

```bash
cargo run --bin check-doc-links
```

## 5. Docs sync (if behavior changed)

- User-visible → `docs/CHANGELOG.md`
- Shortcuts → update `/keybindings` skill
- Config → update `CLAUDE.md` config section

## 6. Keybinding verification (if shortcuts changed)

```bash
source .claude/skills/run-gridix/driver.sh
launch
key Ctrl+N
ss check
quit
```

## One-liner

```bash
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test && cargo run --bin check-doc-links && echo "PASS"
```
