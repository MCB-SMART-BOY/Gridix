# Stage 5: Test

## Entry Criteria
- [ ] Stage 4 review passed
- [ ] All build checks pass

## Test Strategy

Use the smallest test that catches the likely failure:

| Change | Preferred tests |
|---|---|
| Pure logic | unit tests |
| Boundary/API | integration or contract tests |
| Bug fix | regression test |
| Refactor | characterization tests |
| Config/data migration | old-format and new-format load/save tests |
| Performance | benchmark or repeatable measurement |
| UI | state/reducer tests plus minimal render tests |

## Rust Test Patterns

Pure test:

```rust
#[test]
fn parses_valid_input() {
    let parsed = parse("input").unwrap();
    assert_eq!(parsed.value, 42);
}
```

Result-returning test:

```rust
#[test]
fn writes_config() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    write_config(dir.path())?;
    Ok(())
}
```

Async test:

```rust
#[tokio::test]
async fn executes_async_flow() {
    let result = run_flow().await;
    assert!(result.is_ok());
}
```

egui component test:

```rust
#[test]
fn test_new_widget() {
    let ctx = egui::Context::default();
    ctx.begin_pass(RawInput::default());
    // Render and assert
}
```

## Running

```bash
cargo test
cargo test -p <package> --lib
cargo test --test <integration_test>
cargo test <module_or_test_name>
```

## Exit Criteria
- [ ] Relevant tests pass
- [ ] Full test suite passes before merge or skipped checks are justified
- [ ] New behavior has coverage or documented manual verification
- [ ] Flaky tests are not introduced
