# Rust Modern Engineering Playbook

## Purpose

Use this playbook for Rust projects. It complements `~/.codex/references/modern-software-engineering-workflow.md`.

The workflow is intentionally modern: strong type boundaries, explicit ownership, small crates/modules, fast feedback loops, property/fuzz tests where useful, and CI gates that match release risk.

## Default Rust Quality Gates

Fast local loop:

```bash
cargo check
cargo test -p <package> <test_name>
```

Pre-commit:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

Pre-merge for workspaces:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo doc --workspace --no-deps
```

Useful optional checks:

```bash
cargo test --workspace --no-default-features
cargo test --workspace --all-features
cargo tree -d
cargo audit
cargo deny check
cargo nextest run
cargo llvm-cov nextest --workspace --all-features
cargo bench
```

Use optional tools only if installed and configured. Do not block a task on missing optional tooling unless the project requires it.

## Toolchain

Recommended:
- Pin toolchain in `rust-toolchain.toml`.
- Set MSRV explicitly if publishing a library.
- Keep `Cargo.lock` for applications and binaries.
- Use workspace-level lints where appropriate.

Example:

```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy"]
```

## Cargo Structure

Good defaults:
- `src/lib.rs` contains reusable logic.
- `src/main.rs` is thin.
- `src/bin/*.rs` for extra binaries.
- `tests/*.rs` for integration tests.
- `benches/*.rs` for benchmarks.
- `examples/*.rs` for public examples.

Module rules:
- Keep side effects near boundaries.
- Keep core logic in pure functions where possible.
- Do not make one `mod.rs` own multiple unrelated concepts.
- Split files before they become hard to review.
- Prefer explicit module names over generic `utils`.

## Error Handling

Applications:
- Use `thiserror` for domain errors.
- Use `anyhow` at top-level boundaries if helpful.
- Preserve source errors.
- Include actionable context.

Libraries:
- Avoid `anyhow` in public APIs.
- Expose typed errors.
- Avoid panics except for impossible internal invariants.

Rules:
- No `unwrap()`/`expect()` in production paths unless justified by invariant.
- `expect()` message should explain why the invariant holds.
- Convert IO/config/input failures into errors.

## Ownership And API Design

Prefer:
- Borrowing over cloning for read-only access.
- Small owned types at boundaries.
- Newtypes for IDs and domain concepts.
- `Arc` only when shared ownership is real.
- `Cow` only when it simplifies real copy/borrow cases.

Avoid:
- Leaking internal collections mutably.
- Large parameter lists without a context struct.
- Trait objects before a real dynamic boundary exists.
- Generic abstractions with only one implementation.

## Async Rust

Rules:
- Do not block inside async tasks.
- Use `spawn_blocking` for blocking CPU/IO when needed.
- Pass cancellation tokens or request IDs for stale work.
- Avoid holding locks across `.await`.
- Prefer bounded channels for backpressure.
- Use `tracing` spans around async task boundaries.

Testing:

```rust
#[tokio::test]
async fn does_the_async_thing() {
    // arrange, act, assert
}
```

If time is involved:
- Prefer fake clocks where practical.
- Use timeouts sparingly.
- Avoid sleeps in tests unless testing timing behavior directly.

## State Machines

Use explicit enums for states:

```rust
enum LoadState<T, E> {
    Idle,
    Loading { request_id: u64 },
    Loaded(T),
    Failed(E),
}
```

Rules:
- Make invalid states unrepresentable when practical.
- Put transitions in methods or reducers.
- Test transitions independently from UI/IO.

## Configuration

Rules:
- Deserialize into typed structs.
- Add `#[serde(default)]` for additive migrations.
- Validate and normalize after load.
- Test old config and new config.
- Save atomically when writing user config.

Pattern:

```rust
impl AppConfig {
    pub fn load() -> Self {
        let mut config = read_or_default();
        config.normalize();
        config
    }
}
```

## Testing In Rust

Unit tests:

```rust
#[test]
fn parses_valid_input() {
    let parsed = parse("input").unwrap();
    assert_eq!(parsed.value, 42);
}
```

Result-returning tests:

```rust
#[test]
fn writes_config() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    write_config(dir.path())?;
    Ok(())
}
```

Property tests:
- Use for parsers, formatters, serializers, ID normalization, filters, reducers.
- Tools: `proptest`, `quickcheck`.

Fuzz tests:
- Use for untrusted parsers and binary/text import.
- Tool: `cargo fuzz`.

Snapshot tests:
- Use for stable CLI output, generated SQL, serialized config.
- Avoid for noisy UI unless normalized.

Benchmarks:
- Use Criterion for algorithm or parser performance.
- Always compare against a baseline.

## Refactoring Rust Safely

Recommended order:

1. Add tests around current behavior.
2. Extract pure functions.
3. Introduce new structs/enums with `From`/adapter impls.
4. Move code into modules with `pub(crate)` APIs.
5. Update imports through facade modules.
6. Delete old code after tests pass.

Mechanical move pattern:

```text
1. Create new module.
2. Move type/function.
3. Re-export from old module.
4. Run cargo check.
5. Update call sites in small groups.
6. Remove re-export when no longer needed.
```

Avoid broad search/replace when field names are common.

## Performance In Rust

Measure first:

```bash
cargo bench
hyperfine 'cargo test -p mycrate'
```

Profiling options:
- `cargo flamegraph`
- `perf`
- `samply`
- `heaptrack`
- `valgrind massif`

Common wins:
- Avoid repeated allocation in hot loops.
- Reuse buffers.
- Use iterators for clarity, but inspect generated performance only on hot paths.
- Prefer `&str`/slices for read-only APIs.
- Avoid unnecessary `String` cloning.
- Use `SmallVec`/`IndexMap` only when measurements justify them.
- Batch IO and database calls.

Do not:
- Replace clear code with unsafe code without benchmark and safety proof.
- Add caching without invalidation strategy.

## Unsafe Policy

Default: no `unsafe`.

If required:
- Isolate in a tiny module.
- Document safety invariants.
- Add tests around boundary behavior.
- Consider `miri` if applicable.

Command:

```bash
cargo +nightly miri test
```

Only use if the project supports nightly/Miri.

## CI Recommendations

Minimum:

```text
fmt
clippy
test
doc build
```

Recommended additions:

```text
all-features test
no-default-features test
audit/deny
coverage on main branch
benchmarks on demand
release artifact smoke test
```

For GUI apps:
- Add headless smoke test where possible.
- Keep UI logic testable without GPU.
- Separate state/reducer tests from rendering tests.

## Rust Definition Of Done

A Rust change is done when:

- `cargo fmt --check` passes.
- `cargo clippy --all-targets --all-features -- -D warnings` passes or skipped with reason.
- Relevant `cargo test` commands pass.
- Public API changes are documented.
- Config/data migrations are tested.
- No production `unwrap()`/`expect()` was added without invariant justification.
- Async code has cancellation/stale-response behavior where applicable.
