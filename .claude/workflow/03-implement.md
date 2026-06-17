# Stage 3: Implement

## Entry Criteria
- [ ] Approved plan from Stage 1
- [ ] Design decisions documented (for architectural changes)

## Activities

### Layer Awareness

When editing files, know which layer you're in:

| Layer | Directory | Can import from | Cannot import from |
|-------|-----------|-----------------|---------------------|
| -1 | `src/types.rs` | nothing | anything |
| 0 | `src/core/` | types | data, session, state, ui, egui |
| 1 | `src/data/` | types, core | session, state, ui, egui |
| 2 | `src/session/` | types, core, data | state, ui, egui |
| 3 | `src/state/` | types, core, data, session | ui, egui, app |
| 4 | `src/app/`, `src/ui/` | all below | nothing (top) |

### Field Migration Pattern

IF moving a field from DbManagerApp to Session or UiState:
1. Add field to target struct FIRST
2. Update target struct's constructor/Default
3. Replace ONE reference, verify with `cargo check`
4. Repeat for ALL references
5. Remove from DbManagerApp
6. Run `cargo test`

**NEVER use sed batch replacement** — it corrupts other structs with same-named fields.

### Session Access Pattern

- Session fields: `self.session.xxx`
- State fields: `self.state.xxx`
- DbManagerApp fields: `self.xxx` (only for remaining ~11 fields)

### Code Conventions

- `use crate::prelude::*;` for common types
- `thiserror` for error types
- `// =====...=====` section separators
- Chinese `//!` module docs, English identifiers
- `#[allow(dead_code)]` only with Chinese justification comment

### After Each Change

```bash
cargo check    # Quick verification
cargo test     # Full verification before commit
```

## Exit Criteria
- [ ] `cargo test` passes (0 failures)
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] No `self.field` references to migrated fields
- [ ] No cross-layer imports introduced

## Artifacts
- Working code changes
- Tests for new functionality
