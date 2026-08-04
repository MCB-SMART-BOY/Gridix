---
name: modern-engineering-workflow
description: Use this general software engineering workflow for planning, implementing, refactoring, optimizing, testing, reviewing, and delivering software changes in any codebase. Especially useful when the user asks for a modern development process, safe refactor plan, testing strategy, Rust workflow, CI quality gates, or an execution method that generalizes beyond one project.
paths:
  - Cargo.toml
  - src/**/*.rs
  - tests/**/*.rs
---

# Modern Engineering Workflow

Use this skill for cross-project engineering process decisions and for guiding implementation work when no more specific project skill applies.

## Core References

- General workflow: `.claude/references/modern-software-engineering-workflow.md`
- Rust playbook: `.claude/references/rust-modern-engineering-playbook.md`

Read the general workflow first. Read the Rust playbook when the repository is Rust or the task asks about Rust-specific quality, testing, optimization, async, Cargo, or refactoring.

## Default Process

1. Classify the work: feature, bug fix, refactor, optimization, test hardening, migration, or release.
2. Inspect the repo before proposing changes.
3. Define success criteria and verification commands.
4. Add or identify the safety net before high-risk changes.
5. Implement in small slices.
6. Run targeted checks during the loop.
7. Run full quality gates before delivery when practical.
8. Report exact checks run and any skipped checks.

## Rust Defaults

For Rust projects, prefer:

```bash
cargo check
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

For workspaces, use:

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Use optional tools such as `cargo nextest`, `cargo llvm-cov`, `cargo audit`, `cargo deny`, `cargo fuzz`, and Criterion only when they are installed/configured or the project requires them.

## Refactor Rule

For refactoring, preserve behavior first:

```text
characterize -> extract -> adapt -> move -> switch callers -> delete old path -> verify
```

Do not combine broad structural moves with behavior changes unless the behavior change is explicitly scoped and tested.

## Optimization Rule

For optimization:

```text
define metric -> measure baseline -> locate hot path -> change one thing -> re-measure -> keep or revert
```

Never claim a performance improvement without a repeatable measurement or a clearly reasoned complexity reduction.
