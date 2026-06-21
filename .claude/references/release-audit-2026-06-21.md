# Release-readiness audit — 2026-06-21

Scope: completeness of logic chains, operation linkage (联动), and visual/interaction
design across the whole app. Method: 7 read-only static sweeps mapped onto the 9 core
flows in `core-flows.md`, cross-checked against `bug-ledger.md` / `tech-debt.md`.
Three highest-impact findings (G1, G3, CONN-F4) were re-verified against source by hand.

Severity legend:
- **BLOCKER** — data loss, crash, zombie state, or a core feature broken on a supported path. Should gate a confident release.
- **GAP** — stale view, missing feedback, or keyboard-first/contrast contract broken under some condition.
- **POLISH** — inconsistency, missing design-system token, magic number.

---

## Headline root cause: no centralized derived-state invalidation

The single largest pattern across findings is **"the operation succeeds but its dependent
state is not invalidated."** Each handler manually clears *some* of the state that depends
on a connection/schema/result, and forgets the rest. This one root cause produces at least
9 separate findings (G1, ER-4/5/6, SM-8/9, CONN-F1/2/6, CONN-F3/F8). It is exactly the
"操作联动不完善" the user described.

The durable fix is **one invalidation cascade** that every state-changing path calls
(connect / disconnect / switch-connection / switch-database / successful DDL / successful
grid save), rather than 9 independent one-off patches. Design this before patching the
individual symptoms — the symptoms are the test cases for the cascade.

---

## BLOCKERS

> **Status 2026-06-21:** All 9 audit BLOCKERs addressed across separate commits.
> B1/B2/B3-grid (transactional grid save), B4 (query cancel UI), B5/B6 (zombie active +
> stale db-switch metadata), B6-ER (cross-connection ER guard), B7 (hot-path panics),
> B8/B9 (WelcomeSetup modal + keyboard-first bindings). B3-tabclose (unsaved-edit on
> close) MITIGATED to a warning (full pre-close confirm dialog is a tracked follow-up).
> Remaining work is the GAP/POLISH backlog below and the invalidation-cascade refactor.

| ID | Title | Evidence | Symptom |
|---|---|---|---|
| ~~B1 (G1)~~ ✅ | Grid edits not cleared after successful save | `app/runtime/handler.rs` `handle_grid_save_done` now clears edits on committed batch | FIXED |
| ~~B2 (G3)~~ ✅ | MySQL grid save uses wrong identifier quoting | `db_type` threaded into `generate_save_sql` | FIXED |
| ~~B2b (G2)~~ ✅ | Multi-statement save fire-and-forget | now single transactional `execute_import_batch` | FIXED |
| B3 (G4) | Unsaved grid edits silently dropped on tab close / table switch | ~~`request_lifecycle.rs`/`input_router.rs`~~ PARTIAL: table-switch already persists edits to the store (isolation fix, recoverable). Tab close now warns via `warn_if_tab_has_unsaved_grid_edits` before discarding (no longer silent). Full pre-close confirm dialog remains a follow-up. | MITIGATED 2026-06-21 |
| B4 (Q1) | No user-visible way to cancel a running query | ~~`app/runtime/request_lifecycle.rs:9`~~ FIXED: `cancel_active_query()` + `AppAction::CancelQuery` + palette command "取消查询" + ⏹ stop button beside the SQL-editor spinner | FIXED 2026-06-21 |
| B5 (CONN-F4) | Zombie `manager.active` on connect failure | ~~`app/runtime/database.rs:107`~~ FIXED: `handle_connection_error` now resets `manager.active=None` when the failed connection is active | FIXED 2026-06-21 |
| B6 (CONN-F3 + F8) | In-flight ER fetches are untracked + carry no connection identity | ~~`session/message.rs:49-52`~~ FIXED: ER messages carry a monotonic `load_generation`; `ERDiagramState.clear()`/`begin_loading()` bump it; handlers drop mismatched generations; disconnect clears ER (also fixes CONN-F1) | FIXED 2026-06-21 |
| B7 (crash) | Hot-path panic on missing scoped-command registry entry | ~~`ui/shortcut_tooltip.rs:527, 685`~~ FIXED: graceful empty/sentinel fallback + debug_assert + tracing; `MISSING_SCOPED_COMMAND` sentinel in `core/commands.rs` | FIXED 2026-06-21 |
| B8 (DLG) | WelcomeSetup bypasses modal input blocking | ~~`app/surfaces/render.rs:407-408`~~ FIXED: `has_modal_dialog_open()` now treats `show_welcome_setup_dialog` as modal directly | FIXED 2026-06-21 |
| B9 (DLG) | Keybindings dialog is keyboard-unreachable | ~~`Action::OpenKeybindingsDialog` no default~~ FIXED: bound `Ctrl+,`; `FocusSidebar*` bound `Ctrl+1..6` | FIXED 2026-06-21 |

---

## GAPS

### Linkage / staleness (the invalidation-cascade family)

| ID | Title | Evidence | Symptom |
|---|---|---|---|
| ER-4 | ER stale after CREATE/DROP TABLE in SQL editor | ~~`handler.rs:572-576`~~ FIXED: schema-invalidation cascade reloads tables + ER (if open) after table DDL | FIXED 2026-06-21 |
| ER-5 | ER stale after sidebar DROP TABLE | ~~`handler.rs:498-537`~~ FIXED: `handle_table_dropped` reloads ER when open | FIXED 2026-06-21 |
| ER-6 | ER not reloaded on connection/db switch | ~~`handler.rs:286-317, 405-437`~~ FIXED: editor DDL via cascade; db-switch reloads ER when open. (Switch-active-*connection* while ER open = CONN-F2, still open.) | FIXED 2026-06-21 |
| SM-8 | Sidebar triggers/routines not refreshed after DDL | ~~`handler.rs:572-800`~~ FIXED: cascade calls `load_triggers/routines` after trigger/routine DDL | FIXED 2026-06-21 |
| SM-9 | Re-expanding Triggers/Routines does not re-fetch | `sidebar/mod.rs:1051-1065`, `render.rs:1020-1021` | Collapse+expand shows cached pre-DDL data |
| CONN-F1 | ER state not cleared on disconnect | ~~`database.rs:218-273`~~ FIXED: disconnect active-branch now clears `er_diagram_state` (also bumps load generation) | FIXED 2026-06-21 |
| CONN-F2 | ER state not cleared on switch connection | ~~`render.rs:752-763`~~ FIXED: switch-connection clears `er_diagram_state` (bumps generation); `handle_connected_with_tables` reloads ER when open | FIXED 2026-06-21 |
| CONN-F6 | Autocomplete not cleared on switch-db failure | ~~`handler.rs:428-435`~~ FIXED: error arm now clears autocomplete + triggers/routines | FIXED 2026-06-21 |

### Query / grid feedback

| ID | Title | Evidence | Symptom |
|---|---|---|---|
| G2 | Multi-statement grid save is fire-and-forget | `app/surfaces/render.rs:519-521` loops independent `execute()` calls | Partial failure: some rows commit, others fail, no per-row outcome |
| G5 | Empty cell silently coerced to NULL | `ui/components/grid/actions.rs:116-124` | NOT NULL columns fail at DB with no pre-save hint |
| G6 | No client-side type validation | `ui/components/grid/actions.rs` (absent) | Type errors only surface at save time |
| Q2 | Tab-bar close skips `sync_from_active_tab` | ~~`app/surfaces/render.rs:1211-1232`~~ FIXED: `handle_tab_actions` calls `sync_from_active_tab` after any close (close_tab/others/right) | FIXED 2026-06-21 |
| EL-02 | Results panel shows no executing state | ~~`app/surfaces/workbench.rs`~~ FIXED: `render_bottom_panel_results` shows a spinner/loading card while the active tab is executing | FIXED 2026-06-21 |

### Metadata / error feedback

| ID | Title | Evidence | Symptom |
|---|---|---|---|
| ER-3 | ER load error never rendered in canvas | ~~`er_diagram/state.rs`~~ STATE FIXED: `error` field + `set_error`/`clear` wiring + handler sets it on FK-fetch failure. Canvas error-card render branch is staged in working-tree `render.rs` (deferred to land with the in-flight ER visual redesign already modifying that file). | STATE FIXED 2026-06-21 (render pending) |
| SM-3 | No table-list loading indicator | ~~`sidebar/state.rs:293-377`~~ FIXED: `loading_tables` (from `session.connecting`) → spinner + "正在加载表…" in the empty table branch (SQLite + multi-db) | FIXED 2026-06-21 |
| SM-6 | Trigger load error indistinguishable from empty | ~~`handler.rs:908-918`~~ FIXED: `error_triggers` field + panel error branch | FIXED 2026-06-21 |
| SM-7 | Routine load error indistinguishable from empty | ~~`handler.rs:957-970`~~ FIXED: `error_routines` field + panel error branch; SQLite "不支持" treated as empty not error | FIXED 2026-06-21 |
| CONN-F5 | Welcome setup dialog opens on every connect failure | ~~`database.rs:551-556`~~ FIXED: `connection_error_warrants_onboarding` gates the dialog to setup/init errors only (not timeout/refused/auth) | FIXED 2026-06-21 |
| CONN-F7 | PostgreSQL pool task leaks on remove | `data/pool.rs:366-369` drops `Arc<Client>` without signalling bg task | Lingering PG server connections after disconnect |

### Keyboard / dialog contract

| ID | Title | Evidence | Symptom |
|---|---|---|---|
| DLG-A2-2 | 6 form dialogs have no keyboard close | ~~`input_router.rs:1349`~~ FIXED: `resolve_dialog_shortcut_fallback_with` + `CloseDialog` handler now route Esc→close for Connection/Export/Import/Ddl/CreateDatabase/CreateUser (blurs field first when typing, same as Help/History) | FIXED 2026-06-21 |
| DLG-A3-1 | `open_dialog()` doesn't close other major dialogs | ~~`dialogs/host.rs:160-183`~~ FIXED: `close_other_modal_dialogs` closes all other standard dialogs (except WelcomeSetup overlay) before opening | FIXED 2026-06-21 |
| DLG-B1-1 | BottomPanel/RightInspector not in Tab focus cycle | `app/input/keyboard.rs:16-31` | No keyboard route to focus Results/Messages/Inspector |
| DLG-B2-1b | All 6 `FocusSidebar*` actions have no default binding | ~~`core/keybindings.rs:1038-1099`~~ FIXED: bound `Ctrl+1..6` | FIXED 2026-06-21 |
| DLG-B2-3 | `editor.insert.history_browse` shadows history_prev/next | ~~`core/commands.rs:856-879`~~ FIXED: `history_browse` (never functionally consumed) now has no default bindings; help display repointed to `SqlHistoryPrev` | FIXED 2026-06-21 |
| A4-1 | No explicit focus-return after dialog dismiss | `dialogs/host.rs:186-210` | Focus left implicit after cancel via X |

### Visual — broken under specific themes (G41-B013 family)

| ID | Title | Evidence | Symptom |
|---|---|---|---|
| TC-03 / HC-05 | Notification toast text gray-220 | ~~`ui/components/notifications.rs:89`~~ FIXED: uses `visuals().text_color()` + fade alpha | FIXED 2026-06-21 |
| TC-04 / HC-06 | Help info-card near-white text | ~~`help_dialog/topic_content.rs`~~ FIXED: uses `body_text_color`/`muted_text_color` helpers | FIXED 2026-06-21 |
| HC-07 | Help action-button fixed dark-blue fill | ~~`topic_content.rs:1018`~~ FIXED: `theme_accent` fill + `contrasting_text` label + `theme_selection_fill` secondary | FIXED 2026-06-21 |
| TC-01 / HC-08 | Filter divider gray-60 + dark-only selection bg | ~~`sidebar/filter_panel.rs:139,181-188`~~ FIXED: `theme_subtle_stroke` divider + `theme_selection_fill`/`theme_accent` selection | FIXED 2026-06-21 |
| HC-03/04 | Grid delete/filter colors hardcoded | ~~`grid/render.rs:39,66,82,90,146,186`~~ FIXED: `theme_error`/`theme_success`/`contrasting_text`. (HC-01/02 grid cell-selection + mode-indicator constants still hardcoded — need visuals threaded through `GridMode::color`, tracked.) | PARTIAL |

### Visual — layout / design-system

| ID | Title | Evidence | Severity | Symptom |
|---|---|---|---|---|
| NV-01/02 (SP-01) | CreateUserDialog has no responsive row system | `create_user_dialog.rs:446-534` | GAP | Overflow/clip at narrow width (G41-B007 fixed only in CreateDbDialog) |
| FO-01 | No focus ring/stripe on active surface | `ui/workbench/surface.rs` (absent) | GAP | User can't tell which panel owns keyboard input |
| MX-01 | Emoji 😊 avatar in text-glyph chrome | `toolbar/mod.rs:147` | GAP | Mixed icon language; OS-dependent rasterization |

---

## POLISH (tracked, not release-gating)

- HC-09..17: remaining hardcoded colors in sidebar chrome, connection-dialog strokes, ACCENT_BLUE spinner/avatar, primary-button white text, create dialogs' gray labels, export selection indicators — migrate to `ThemeColors`/`theme_*` helpers.
- SP-02/03/04: magic-number `add_space`/heights in About/Help/SurfaceHeader/filter panel — adopt `SPACING_*`/`SURFACE_HEADER_HEIGHT` constants.
- MX-02..05: emoji mode toggle, text-tab strips (compat), query-tab bg using SQL keyword color, toolbar focus asymmetry.
- EL-01/03/04: unstyled empty states (grid/bottom/inspector/sidebar) — adopt one shared empty-state template.
- A2-1/A2-4: WelcomeSetup & CommandPalette Esc handled widget-side, not via central router.
- ER-7: no large-schema (100+ table) layout guard — synchronous O(n²) layout on UI thread.
- SM-5: ~~routine empty-state copy hardcoded "SQLite 不支持存储过程"~~ FIXED 2026-06-21: generic "选择数据库后自动加载"; SQLite no-routine treated as empty.
- CONN-F9: SSH tunnel stop fire-and-forget; port may take ~1s to free.
- D5/D6/D7: drop-db cleanup skipped for non-active conn; delete-connection dialog text understates discard; no DROP USER confirm path (feature gap).
- Q3/Q4: `was_user_cancelled` path unreachable (depends on B4); close-others/close-right skip sync.

---

## Verified-OK (checked, working)

Tab isolation (per `(tab,conn,db,table)` key); save-failure preserves edits; save-confirm
blocks grid keyboard; all sidebar destructive actions route through one confirm dialog;
row-delete requires mark+confirm; drop-table/drop-database clean up workspaces+sidebar;
stale-response request_id guards on query/connect/db messages; empty-SQL execute guarded at
3 layers; DDL/0-row success feedback; error-state reset + Messages reveal; ER `RenderColors`
fully theme-derived; dialog shells (window/card/nav/shortcut-hint) theme-derived; all 16
DialogId variants present in every host.rs match arm; ER + Help + History keyboard contracts
complete; last-tab-close guarded 4 ways; in-flight query (not ER) tasks cancelled on
disconnect/tab-close.

---

## Recommended sequencing

1. **Design the invalidation cascade first** (root cause of B1, B6, ER-4/5/6, SM-8/9, CONN-F1/2/6). One function, called from every state-changing path; the listed symptoms become its test matrix.
2. **B2/B3 grid-save correctness** (data integrity + MySQL broken) — thread `db_type` through, clear edits on success, warn on unsaved-edit navigation.
3. **B5/B6 connection-state correctness** (zombie active + cross-connection ER) — reset `active` on failure, add connection-id/request-id guard to ER messages, register ER tasks for cancellation.
4. **B4 query cancel** — wire `AppAction::CancelQuery` + toolbar/shortcut to the existing user-visible cancel path.
5. **B7/B8/B9 crash + keyboard-first contract** — replace hot-path panics with graceful fallback; fix Welcome modal blocking; bind OpenKeybindingsDialog + FocusSidebar*.
6. **Theme-contrast GAPs** (TC/HC family) — these are visible to every user on half the themes; high value-to-effort.
7. POLISH backlog as capacity allows.
