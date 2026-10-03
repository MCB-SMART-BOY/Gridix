# Grid save context isolation

（原始修复记录未入库；本文档是唯一在库描述。）

## Root cause (fixed)

`handle_grid_save_done()` was clearing current `grid_state` before verifying workspace/tab match.
A stale save response for tab A could clear tab B's active edits.

## Two error classes (both fixed)

1. **Unrelated page cleared by stale response**: save response for tab A arrives while tab B is active → tab B's grid state cleared
2. **Old drafts revived on tab switch**: switching back to tab A restores workspace that was saved while you were on tab B

## Current invariants

1. Each save is bound to a complete `GridWorkspaceId` (document, connection name, **ConnectionId**, database, table); both the workspace store and the submitting snapshot use the original connection instance. A late completion from a replaced same-name connection cannot retire new drafts, change save locks or refresh the new workspace.
2. `GridSubmittedEdits` captures submitted cells, deleted rows and inserted rows. While that workspace's save is in flight, editing, row insertion/deletion, repeat submission and table refresh are blocked; other workspaces keep their drafts and navigation remains available.
3. On commit, only submitted edits are retired in the owning visible grid or saved workspace. The committed workspace remains read-only until a **post-commit** table refresh succeeds; an earlier response, failed refresh or inactive owner must not unlock stale row indices or primary keys. If the owner is inactive at commit, refresh it when returning before editing again.
4. Confirmed rollback keeps the draft and unlocks editing for retry. A PostgreSQL/MySQL COMMIT or ROLLBACK acknowledgement failure is **not** a confirmed rollback; a MySQL ROLLBACK warning after a non-transactional write is likewise unknown. Retain edits and lock the workspace. After verifying the database state, explicitly discard the uncertain draft and successfully refresh the table before editing again.
5. `OperationKey::GridSave { table_view }` is document-scoped, even when two SQL tabs show the same connection and table. Other tabs' already-loaded results are not invalidated by this save; they may need a manual refresh before editing the same table.
6. On connection/database change, persist and deactivate the visible grid **before** changing `manager.active` or `selected_database`. `switch_grid_workspace(None)` after changing identity would save the old draft (or save lock) under the new connection/database. A dropped database removes its old workspaces after deactivation.
7. Query results carry their submitting `ConnectionId` and database in `QueryTab.result_origin`; reopening an old SQL tab on another connection hides the result and table grid until the original identity is active again or the query is rerun. Saving a grid requires that result origin to match the current connection/database, not only a matching table name. Restoring a draft after a different connection's query must also restore the owning Tab's original result baseline, table and origin before saving or navigating; a foreign Tab result sharing the same `Arc` is never relabelled as the draft's origin. Any pending query reply for the replaced Tab result becomes UI-stale; the SQL task itself is **not** silently cancelled and may still change the database, so warn the user to verify its outcome. An empty tab workspace with no result may still restore its draft under its own workspace identity.
8. Async table/database deletion removes workspaces by the submitted connection name and database, not the currently active connection. Deactivate a matching visible workspace before removing it so an old draft cannot be saved back under a recreated table.
9. Once a table grid has pending edits, deletes or inserts, ordinary queries cannot replace its original row/primary-key baseline. A query that was already in flight when editing began may finish, but its result cannot overwrite the draft. Use another SQL tab to query, or explicitly save/discard the draft before requerying; a confirmed commit's refresh remains permitted.
10. A table/database deletion also detaches matching pending save snapshots. The database may already have processed such a save, but its late completion must not retire or unlock a new same-named table's draft after recreation. A successful table query from the sidebar binds the owning `QueryTab` to that table for navigation; deactivating the grid during a connection/database switch must not erase that Tab binding.

## Transactional batch save (audit B1/B2/B3, typed path)

Grid save no longer fires each UPDATE/DELETE/INSERT as an independent `execute()` call.
Staged edits are converted into a typed `MutationBatch` in `domain/mutation.rs` — the
module explicitly replaces the historical `generate_save_sql()` string output <!-- doc-symbols: ignore: names the removed pre-typed save path --> — and
dispatched through `execute_grid_save_typed()` (`app/runtime/database.rs`), which runs
the batch through `apply_mutations()` and registers the task in `TaskRegistry`:

- Transactional engines apply the batch atomically, but lost COMMIT/ROLLBACK acknowledgement leaves the **outcome unknown**; do not infer a rollback from a transport error. MySQL ROLLBACK warnings for non-transactional writes also quarantine the draft; non-transactional engines still cannot provide an atomic batch guarantee.
- Completion arrives as `RuntimeEvent`/`RuntimeOutcome`, handled on the UI thread; an older
  save can still commit even when a newer task superseded its registry key, so its
  outcome must retire only its own captured edits rather than being silently discarded.
- `classify_grid_save_outcome()` distinguishes committed, confirmed rollback, and unknown:
  - Committed → retire the submitted edits from the owning workspace, lock its current result baseline and refresh only when that workspace is visible; the refresh completion unlocks it.
  - Confirmed rollback → keep edits, unlock the workspace and show the error.
  - Unknown → retain the draft, quarantine that workspace and instruct the user to verify the database before explicitly discarding the draft and refreshing.
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

Manual: open two tabs on the same table, edit both, save tab A and switch to tab B → B's draft remains; return to A → A cannot edit its stale result until a successful table refresh. Slow-save and failed-refresh GUI flows still need direct acceptance evidence.
