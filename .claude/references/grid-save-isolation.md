# Grid save context isolation

From `docs/recovery/24-grid-save-context-isolation-fix.md`.

## Root cause (fixed)

`handle_grid_save_done()` was clearing current `grid_state` before verifying workspace/tab match.
A stale save response for tab A could clear tab B's active edits.

## Two error classes (both fixed)

1. **Unrelated page cleared by stale response**: save response for tab A arrives while tab B is active → tab B's grid state cleared
2. **Old drafts revived on tab switch**: switching back to tab A restores workspace that was saved while you were on tab B

## Current invariants

1. Save success must not clear unrelated workspace (verify `GridWorkspaceStore` key match)
2. Old drafts must not revive on tab switch (`persist_active_tab_state_for_navigation()` must overwrite)
3. Save failure must preserve edits (modified_cells not cleared on error)
4. Refresh after save must go to the same table (GridSaveContext preserves table identity)

## Validation

```bash
cargo test -p gridix --lib grid        # DataGrid state tests
cargo test -p gridix --test grid_tests  # Grid integration tests
```

Manual: open two tabs on same table, edit both, save tab A, switch to tab B → edits must still be there.
