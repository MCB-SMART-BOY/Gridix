# Bug ledger

From the v4.1.0 → v6.1.0 recovery audit. Historical resolved entries remain below; this ledger does not claim that no bugs remain.

## Fixed — 2026-10-03

| ID | symptom | root cause | fix |
|---|---|---|---|
| SQLite-PRAGMA | Setting PRAGMA with zero result columns aborted the release process | The typed query path divided by `column_count == 0` even with no returned rows | Execute prepared zero-column statements as affected-row commands before result collection; typed integration and release API smoke verified |
| GRID-IDENTITY | Switching connection/database, an old SQL Tab or an asynchronous table/database delete could bind an old grid result or draft to a new identity | Navigation projected Tab results without their connection/database origin; deletion used the currently active identity; reconnect reset the selected database | Tag result provenance, reject mismatched grid save, deactivate matching workspaces before targeted deletion and retain network database selection on reconnect |
| GRID-BASELINE | Requery after unsaved table edits could retarget a different primary key or lose pending deletions; Tab navigation lost the sidebar-selected table | Queries replaced row-indexed grid baselines and cleared deleted rows, while successful sidebar table queries did not bind the owning Tab | Guard dirty grids before submission and after in-flight completion, retain the original baseline, and bind successful table queries to their Tab |
| GRID-DROP-REPLY | A late save for a deleted table/database could retire an identical draft in a recreated object; old database's table-list reply could populate the new database sidebar | Deletion removed workspaces without detaching pending save snapshots, and table reload replies lacked database provenance | Detach matching save snapshots on deletion and require source database identity on ActiveTables reload completion |
| IMPORT-LEXER | PostgreSQL backtick operators and non-ASCII dollar tags or MySQL control-character/Unicode-space/CR comments could hide transaction controls, panic, or rewrite destructive SQL | The lexer shared dialect-incompatible quote, whitespace and line-ending rules; dollar-tag byte offsets were used as character indices | Dialect-specific token boundaries match supported server syntax, reject hidden controls before writes and preserve SQL hidden in MySQL CR comments; cover with core and real database tests |
| GRID-DRAFT-ORIGIN | Returning to connection A's draft after querying connection B in the same SQL Tab could leave the Tab bound to B, making A's draft unsaveable; same-name connection recreation could restore the old draft | The grid workspace store was keyed by connection name, and restoring the grid did not restore the Tab's result provenance | Key workspaces by `ConnectionId`, rebind the original result and table only when the stored baseline is verified, reject foreign provenance and stale same-name replies |
| GRID-QUERY-RACE | A B-connection query completing after A's draft was restored could replace its row baseline while retaining A's origin, then delete the wrong primary key on A | Restoring a saved grid re-bound the SQL Tab but left B's `pending_request_id` valid | Invalidate the old Tab UI reply on verified draft rebind without cancelling already-issued SQL; notify that the database outcome may need checking, and cover the late B reply plus A Tab navigation |
| IMPORT-DELIMITER | A multiline `DELIMITER` inside SQL or a comment-retaining import could hide a transaction control and allow an earlier wrapped write to commit | The import parser treated every `DELIMITER` line as a MySQL client command even inside statements or other dialects, and skipped lexical comment tracking when comments were retained | Recognize MySQL client commands only at statement boundaries; track comments independently of stripping and reject hidden controls before writing |

## Historical fixes — 2026-06-21 (release-audit grid-save blockers)

| ID | symptom | root cause | fix |
|---|---|---|---|
| AUD-B1 | Grid edits stayed "modified" after a successful save | Historical `QueryDone` save path cleared only `rows_to_delete`, never `modified_cells`/`new_rows` <!-- doc-symbols: ignore: names the removed pre-typed save path --> | Replaced by typed grid-save completion handling that clears edits and refreshes only after a committed batch |
| AUD-B2 | Multi-statement save partially committed on error | Each statement ran as an independent asynchronous operation | Transactional typed mutation batch with all-or-nothing behavior |
| AUD-B3 | MySQL grid save emitted double-quoted identifiers in strict mode | Database type was not threaded to SQL generation | Typed MySQL mutation path now binds values and owns backend SQL generation |

## Current observations and acceptance boundaries (not bugs)

- **RA2 SQLite GUI evidence**: create/query/edit/save/reopen were observed on 2026-09-25; GUI-driven CSV/JSON/SQL export files and chooser screenshots were observed in a separate 2026-10-03 run. The export used an already populated fixture and local temporary artifacts, so a continuous same-database journey with retained release artifacts remains pending; see `docs/LIMITATIONS.md`.
- **MySQL cancellation coverage**: CI exercises direct `mysql:8.4` with observer and `KILL QUERY` permissions. TLS, SSH tunnel, execution-pool pressure, and reuse of the exact cancelled connection remain untested boundaries, not known failures.

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
| G41-B007 | Dialog horizontal overflow in narrow viewports | Fixed-width rows (`desired_width`, `ui.horizontal`) in `CreateDbDialog`/`CreateUserDialog`/`ExportDialog` | Shared responsive row helpers (`src/ui/dialogs/responsive.rs`) + `ui.horizontal_wrapped` option rows, with headless width measurements as proof |
