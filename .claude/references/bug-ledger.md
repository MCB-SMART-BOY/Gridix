# Bug ledger

From `docs/recovery/12-bug-ledger-4.1.0.md`. Current state: no active unblocked bugs.

## Fixed 2026-06-21 (release audit grid-save BLOCKERs)

| ID | symptom | root cause | fix |
|---|---|---|---|
| AUD-B1 | Grid edits stay "modified" after a successful save; re-save re-runs same writes | `QueryDone` save path cleared only `rows_to_delete`, never `modified_cells`/`new_rows` | Route save through `execute_grid_save`→`GridSaveDone`; `handle_grid_save_done` calls `clear_edits()` + refresh on a committed batch only |
| AUD-B2 | Multi-statement save partially commits on error (some rows saved, some not) | Each statement fired as an independent async `execute()` (fire-and-forget) | Single transactional batch via existing `execute_import_batch(.., use_transaction=true, stop_on_error=true)`; all-or-nothing |
| AUD-B3 | MySQL grid save fails: emits double-quoted identifiers in strict mode | `db_type` hardcoded `None` at `generate_save_sql` call (grid UI layer lacked it) | Thread `db_type` into `DataGrid::show_editable`→`generate_save_sql`; MySQL backticks, PG/SQLite double-quotes |

## Current observations (not bugs)

**G41-B007** (observation): dialog horizontal overflow from fixed-width row content in narrow viewports. Major live verification completed. Remaining low-frequency surfaces: CreateDbDialog, CreateUserDialog, ExportDialog at narrow widths.

## Resolved during recovery (v4.1.0 → v6.1.0)

| ID | symptom | root cause | fix |
|---|---|---|---|
| G41-B004 | Utility overlay + confirm contract inconsistent | Shell contracts not unified | Blocking modal + form dialog shell |
| G41-B005 | ER `l` key semantics wrong | `l` bound to relayout, should be geometry nav | `l`→geometry, `Shift+L`→relayout |
| G41-B006 | Toolbar menus raw popup | No dialog shell | Overlay dialog with scoped commands |
| G41-B008 | WelcomeSetup no keyboard contract | No scoped commands | Scoped commands + action index |
| G41-B009 | Tiny viewport crashes SQL editor | Unsafe clamp | Safe clamp with min height |
| G41-B010 | Sidebar delete entry points drift | Inconsistent delete targets | Unified SidebarDeleteTarget |
| G41-B011 | AboutDialog section stack | No brand design | Lighter brand page layout |
| G41-B012 | Help/KeyBindings header wasted height | No shared compact header | Shared compact header component |
| G41-B013 | DataGrid column headers invisible in dark theme | Hardcoded colors | Theme-aware text colors |
