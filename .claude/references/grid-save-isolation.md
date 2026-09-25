# Grid save context isolation

（原始修复记录未入库；本文档是唯一在库描述。）

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
4. Refresh after save must go to the same table (`OperationKey::GridSave { table_view }` pins the identity)

## Transactional batch save (audit B1/B2/B3, typed path)

Grid save no longer fires each UPDATE/DELETE/INSERT as an independent `execute()` call.
Staged edits are converted into a typed `MutationBatch` in `domain/mutation.rs` — the
module explicitly replaces the historical `generate_save_sql()` string output <!-- doc-symbols: ignore: names the removed pre-typed save path --> — and
dispatched through `execute_grid_save_typed()` (`app/runtime/database.rs`), which runs the
batch through `execute_import_batch(..., use_transaction=true, stop_on_error=true)` and
registers the task in `TaskRegistry`:

- All statements commit atomically or the whole batch rolls back (**B2**).
- Completion arrives as `RuntimeEvent`/`RuntimeOutcome`, handled on the UI thread; the
  task identity registered for the save is the stale guard, so a superseded batch is
  dropped before it can touch edit state.
- `classify_grid_save_outcome()` is the pure decision: `CommittedClearEdits` only when
  `report.failed == 0`, else `RolledBackKeepEdits`.
  - Committed → `clear_edits()` + `RefreshSelectedTable` (**B1**: edits no longer persist
    after a successful save; re-save no longer re-runs the same writes).
  - Rolled back → keep edits, show error (invariant 3 preserved; tx already rolled back).
- Identifier quoting follows the active backend (MySQL backticks, PG/SQLite double
  quotes) because the batch carries the connection's `db_type` (**B3**: MySQL grid save
  previously emitted broken double-quoted identifiers because `db_type` was hardcoded
  `None`).

## Validation ordering

- The client only blocks structurally impossible saves (for example a NULL primary key in
  an edited row); everything else is warn-but-allow.
- Column-level checks (NOT NULL, type compatibility) live in the driver entry points —
  `validate_mysql_mutation_batch` / `validate_mysql_input_values` and their SQLite and
  PostgreSQL counterparts — so no save path can bypass them.
- Column metadata for the grid comes from the `SchemaCatalog` projection
  (`grid_state`), which replaced the per-table `column_metadata` cache <!-- doc-symbols: ignore: names the replaced cache --> and the PK-only
  `fetch_primary_key` request <!-- doc-symbols: ignore: names the replaced request -->.

## Validation

```bash
cargo test -p gridix --lib grid        # DataGrid state tests
cargo test -p gridix --test grid_tests  # Grid integration tests
```

Manual: open two tabs on same table, edit both, save tab A, switch to tab B → edits must still be there.
