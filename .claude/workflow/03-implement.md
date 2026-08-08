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

### During implementation

Run the narrowest check that exercises the changed behavior. Keep backend differences explicit:

- Runtime query tasks use `execute_typed_cancellable`; they request cancellation cooperatively rather than aborting the query task.
- SQLite follows the synchronous, non-cancellable path; do not add an unsupported cancellation workaround.
- PostgreSQL and MySQL cancellation must await protocol-level server completion after `CancelToken` / `KILL QUERY`.

Before review, run the targeted test or smoke path appropriate to the change. Full workspace validation belongs to Stage 5.

## Exit Criteria
- [ ] All callers use the intended API without compatibility shims
- [ ] Targeted behavior is exercised
- [ ] No cross-layer imports introduced

## Artifacts
- Working code changes
- Tests for new functionality
