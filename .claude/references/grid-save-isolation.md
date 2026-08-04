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

## Transactional batch save (2026-06-21 — fixes audit B1/B2/B3)

Grid save no longer fires each UPDATE/DELETE/INSERT as an independent `execute()` call.
It now routes through the existing transactional batch seam:

- `DbManagerApp::execute_grid_save(table, statements)` (`app/runtime/database.rs`) →
  `execute_import_batch(&config, statements, use_transaction=true, stop_on_error=true)`.
  All statements commit atomically or the whole batch rolls back (**B2**).
- Result returns via `Message::GridSaveDone { result, table, request_id, elapsed_ms }`
  (`session/message.rs`), handled by `handle_grid_save_done` (`app/runtime/handler.rs`).
- `classify_grid_save_outcome()` is the pure decision: `CommittedClearEdits` only when
  `report.failed == 0`, else `RolledBackKeepEdits`.
  - Committed → `grid_state.clear_edits()` + `RefreshSelectedTable` (**B1**: edits no
    longer persist after a successful save; re-save no longer re-runs the same writes).
  - Rolled back → keep edits, show error (invariant 3 preserved; tx already rolled back).
- `db_type` is threaded into `DataGrid::show_editable(..., db_type)` →
  `generate_save_sql(..., db_type)` so identifier quoting is correct: MySQL backticks,
  PG/SQLite double-quotes (**B3**: MySQL grid save previously emitted broken double-quoted
  identifiers because `db_type` was hardcoded `None`).
- Stale-guard: `Session.pending_grid_save_request` tracks the latest save's `request_id`;
  late/superseded batch responses are dropped before touching edit state.

## Pre-save client validation (G5 + G6)

Before the SQL is dispatched, `generate_save_sql` surfaces likely problems in the
user-facing message (warn-but-allow — it never blocks the save):

- **G5**: counts empty cells coerced to NULL → "（其中 N 个空单元格将保存为 NULL）".
- **G6**: `validate_cell(value, &ColumnInfo)` checks each edited/inserted cell against the
  cached column metadata: empty in a NOT NULL column → `NotNull`; value unparsable for an
  int/float/bool column → `TypeMismatch`. Issues are deduped + capped and appended as
  "。请检查:…". Only fires when `grid_state.column_metadata` is populated.
- Column metadata is fetched by `fetch_column_metadata(table)` (`app/runtime/database.rs`,
  replaces the old PK-only `fetch_primary_key`) → `Message::ColumnMetadataFetched` →
  `handle_column_metadata_fetched` caches it and derives `primary_key_column` from
  `is_primary_key`. Stale-guarded on `selected_table`.

## Validation

```bash
cargo test -p gridix --lib grid        # DataGrid state tests
cargo test -p gridix --test grid_tests  # Grid integration tests
```

Manual: open two tabs on same table, edit both, save tab A, switch to tab B → edits must still be there.
