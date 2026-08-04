# PR Description Template

## Summary

- What changed:
- Why:
- User-visible impact:

## Implementation Notes

- Affected layers:
- Important invariants:
- Risk areas:

## Verification

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo run --bin check-doc-links
```

## Follow-Up

- Remaining work:
- Known limitations:
