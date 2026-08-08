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
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## 3. Test and documentation build

```bash
cargo test --workspace --all-features
cargo doc --workspace --no-deps
cargo run --bin check-doc-links
```

## 4. Security audit

```bash
cargo audit
```

## 5. Docs sync (if behavior changed)

- User-visible → `docs/CHANGELOG.md`
- Shortcuts → update `/keybindings` skill
- Config → update `CLAUDE.md` config section

## 6. Keybinding verification (if shortcuts changed)

```bash
cargo run --bin gridix-driver -- launch
cargo run --bin gridix-driver -- key Ctrl+N
cargo run --bin gridix-driver -- ss check
cargo run --bin gridix-driver -- quit
```

## One-liner

```bash
cargo fmt --check && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test --workspace --all-features && cargo doc --workspace --no-deps && cargo run --bin check-doc-links && cargo audit && echo "PASS"
```
