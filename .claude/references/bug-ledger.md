# Bug ledger

From the v4.1.0 → v6.1.0 recovery audit. Historical resolved entries remain below; this ledger does not claim that no bugs remain.

## Historical fixes — 2026-06-21 (release-audit grid-save blockers)

| ID | symptom | root cause | fix |
|---|---|---|---|
| AUD-B1 | Grid edits stayed "modified" after a successful save | Historical `QueryDone` save path cleared only `rows_to_delete`, never `modified_cells`/`new_rows` | Replaced by typed grid-save completion handling that clears edits and refreshes only after a committed batch |
| AUD-B2 | Multi-statement save partially committed on error | Each statement ran as an independent asynchronous operation | Transactional typed mutation batch with all-or-nothing behavior |
| AUD-B3 | MySQL grid save emitted double-quoted identifiers in strict mode | Database type was not threaded to SQL generation | Typed MySQL mutation path now binds values and owns backend SQL generation |
## Current observations and acceptance boundaries (not bugs)

- **RA2 SQLite GUI evidence**: the manual create/query/edit/save/reopen/export journey still lacks captured artifacts. This is an incomplete release-acceptance evidence item, not evidence of a product defect.
- **MySQL cancellation coverage**: CI exercises direct `mysql:8.4` with observer and `KILL QUERY` permissions. TLS, SSH tunnel, execution-pool pressure, and reuse of the exact cancelled connection remain untested boundaries, not known failures.
- **G41-B007**: dialog horizontal overflow can occur from fixed-width row content in narrow viewports. The remaining low-frequency surfaces are `CreateDbDialog`, `CreateUserDialog`, and `ExportDialog`.
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
