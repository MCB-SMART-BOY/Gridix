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

> **Status 2026-06-21:** B1, B2, B3 (grid-save trilogy) **FIXED** — routed grid save
> through the transactional batch seam (`execute_grid_save`→`GridSaveDone`→
> `handle_grid_save_done`), threaded `db_type` for correct quoting, clear-edits-on-commit.
> See `grid-save-isolation.md` and `bug-ledger.md` (AUD-B1/B2/B3). Remaining: B4–B9.

| ID | Title | Evidence | Symptom |
|---|---|---|---|
| ~~B1 (G1)~~ ✅ | Grid edits not cleared after successful save | `app/runtime/handler.rs` `handle_grid_save_done` now clears edits on committed batch | FIXED |
| ~~B2 (G3)~~ ✅ | MySQL grid save uses wrong identifier quoting | `db_type` threaded into `generate_save_sql` | FIXED |
| ~~B2b (G2)~~ ✅ | Multi-statement save fire-and-forget | now single transactional `execute_import_batch` | FIXED |
| B3 (G4) | Unsaved grid edits silently dropped on tab close / table switch | `app/runtime/request_lifecycle.rs:51-55`, `input_router.rs:1199-1230`; close path calls `remove_grid_workspaces_for_tab` with no warning | Closing a tab or switching tables silently destroys uncommitted cell edits, new rows, and delete marks |
| B4 (Q1) | No user-visible way to cancel a running query | `app/runtime/request_lifecycle.rs:9` always passes `user_visible=false`; no `AppAction::CancelQuery`, no button/shortcut. User-cancel machinery exists but has no entry point | A slow/runaway query can only be escaped by waiting for the timeout or killing the app |
| B5 (CONN-F4) | Zombie `manager.active` on connect failure | `app/runtime/database.rs:107` sets `manager.active` synchronously before async; error arms (`handler.rs:309-315, 365-370`) never reset it. `ConnectionManager::handle_connect_result` resets it but is `#[allow(dead_code)]` | After a failed/timed-out connect the UI still shows the connection as active; later SQL/db-select operate against a broken connection |
| B6 (CONN-F3 + F8) | In-flight ER fetches are untracked + carry no connection identity | `session/message.rs:49-52` (`ForeignKeysFetched`/`ERTableColumnsFetched` carry no conn id/request id); `app/runtime/er_diagram.rs:91-128` tasks not registered in any cancellable set; `handler.rs:975-1051` applies results unconditionally | Disconnect from A while ER is loading, connect to B → B's ER view receives A's columns/FKs (cross-connection schema corruption) |
| B7 (crash) | Hot-path panic on missing scoped-command registry entry | `ui/shortcut_tooltip.rs:527, 685` `panic!`/`unwrap_or_else(panic!)` in per-keypress binding lookup | Any `LocalShortcut`/scoped command referenced but not registered crashes the running app on the first keypress that touches it |
| B8 (DLG) | WelcomeSetup bypasses modal input blocking | `app/surfaces/render.rs:407-408` renders Welcome when `active_dialog_id()==None`; `request_lifecycle.rs:14-15` `has_modal_dialog_open` returns false in that state | Workspace shortcuts (Ctrl+T/N…) fire through the WelcomeSetup overlay while it is visually foreground |
| B9 (DLG) | Keybindings dialog is keyboard-unreachable | `Action::OpenKeybindingsDialog` has no entry in `KeyBindings::default()` (`core/keybindings.rs:1038-1099`) | In a keyboard-first app, the only shortcut-customization surface can only be opened via command palette or mouse |

---

## GAPS

### Linkage / staleness (the invalidation-cascade family)

| ID | Title | Evidence | Symptom |
|---|---|---|---|
| ER-4 | ER stale after CREATE/DROP TABLE in SQL editor | `handler.rs:572-576` no `is_create_table`; `data/query/mod.rs:327-331` | ER diagram shows pre-DDL schema until manual refresh; CREATE TABLE undetected |
| ER-5 | ER stale after sidebar DROP TABLE | `handler.rs:498-537` does not call `load_er_diagram_data()` | Dropped table card stays in an open ER diagram |
| ER-6 | ER not reloaded on connection/db switch | `handler.rs:286-317, 405-437` no ER reload | Switching DB while ER open shows old schema |
| SM-8 | Sidebar triggers/routines not refreshed after DDL | `handler.rs:572-800` never calls `load_triggers/routines` | CREATE/DROP TRIGGER leaves sidebar panel stale |
| SM-9 | Re-expanding Triggers/Routines does not re-fetch | `sidebar/mod.rs:1051-1065`, `render.rs:1020-1021` | Collapse+expand shows cached pre-DDL data |
| CONN-F1 | ER state not cleared on disconnect | `database.rs:218-273` never clears `er_diagram_state` | Stale schema diagram after disconnect |
| CONN-F2 | ER state not cleared on switch connection | `render.rs:752-763`, `action_system.rs:1545` | Connection A's schema shown after switching to B |
| CONN-F6 | Autocomplete not cleared on switch-db failure | `handler.rs:428-435` error arm omits `autocomplete.clear()` | After a failed DB switch, completion shows previous DB's tables |

### Query / grid feedback

| ID | Title | Evidence | Symptom |
|---|---|---|---|
| G2 | Multi-statement grid save is fire-and-forget | `app/surfaces/render.rs:519-521` loops independent `execute()` calls | Partial failure: some rows commit, others fail, no per-row outcome |
| G5 | Empty cell silently coerced to NULL | `ui/components/grid/actions.rs:116-124` | NOT NULL columns fail at DB with no pre-save hint |
| G6 | No client-side type validation | `ui/components/grid/actions.rs` (absent) | Type errors only surface at save time |
| Q2 | Tab-bar close skips `sync_from_active_tab` | `app/surfaces/render.rs:1211-1232` | Bottom panel/result/search stale one frame after × close |
| EL-02 | Results panel shows no executing state | `app/surfaces/workbench.rs` (absent) | Long query: results area shows old/empty content, only toolbar spins |

### Metadata / error feedback

| ID | Title | Evidence | Symptom |
|---|---|---|---|
| ER-3 | ER load error never rendered in canvas | `er_diagram/state.rs` (no error field), `render.rs:602-614` (no error branch) | Failed ER load silently shows incomplete/empty diagram |
| SM-3 | No table-list loading indicator | `sidebar/state.rs:293-377` (no `loading_tables`) | Blank table list during async connect looks like empty schema |
| SM-6 | Trigger load error indistinguishable from empty | `handler.rs:908-918`, `trigger_panel.rs` no error branch | Network error looks identical to "no triggers" |
| SM-7 | Routine load error indistinguishable from empty | `handler.rs:957-970`, `routine_panel.rs` no error branch | Same as SM-6 for routines |
| CONN-F5 | Welcome setup dialog opens on every connect failure | `database.rs:551-556` | Momentary timeout pops onboarding repeatedly for experienced users |
| CONN-F7 | PostgreSQL pool task leaks on remove | `data/pool.rs:366-369` drops `Arc<Client>` without signalling bg task | Lingering PG server connections after disconnect |

### Keyboard / dialog contract

| ID | Title | Evidence | Symptom |
|---|---|---|---|
| DLG-A2-2 | 6 form dialogs have no keyboard close | `input_router.rs:1349` falls through; Connection/Export/Import/Ddl/CreateDatabase/CreateUser | Must mouse-click the X; no Esc/Q route |
| DLG-A3-1 | `open_dialog()` doesn't close other major dialogs | `dialogs/host.rs:160-183` | Two `show_*` bools can be true at once; transient owner inconsistency |
| DLG-B1-1 | BottomPanel/RightInspector not in Tab focus cycle | `app/input/keyboard.rs:16-31` | No keyboard route to focus Results/Messages/Inspector |
| DLG-B2-1b | All 6 `FocusSidebar*` actions have no default binding | `core/keybindings.rs:1038-1099` | Sidebar section jumps unreachable without customization |
| DLG-B2-3 | `editor.insert.history_browse` shadows history_prev/next | `core/commands.rs:856-879` | Overlapping default keys; history_browse effectively unreachable |
| A4-1 | No explicit focus-return after dialog dismiss | `dialogs/host.rs:186-210` | Focus left implicit after cancel via X |

### Visual — broken under specific themes (G41-B013 family)

| ID | Title | Evidence | Symptom |
|---|---|---|---|
| TC-03 / HC-05 | Notification toast text gray-220 | `ui/components/notifications.rs:89` | Near-invisible on all 6 light themes |
| TC-04 / HC-06 | Help info-card near-white text | `help_dialog/topic_content.rs:892,899,930,941` | White-on-white on light themes |
| HC-07 | Help action-button fixed dark-blue fill | `topic_content.rs:1018` | Clashes on warm/light themes |
| TC-01 / HC-08 | Filter divider gray-60 + dark-only selection bg | `sidebar/filter_panel.rs:139,181-188` | Divider invisible on dark; wrong-direction tint on light |
| HC-01..04 | Grid cell/mode/delete/filter colors hardcoded | `grid/mod.rs:59-62`, `grid/render.rs:39,66,82,90`, `grid/mode.rs:25-27`, `sql_editor.rs:44-45` | Selection/mode/delete states clash or vanish on non-blue themes |

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
- SM-5: routine empty-state copy hardcoded "SQLite 不支持存储过程" shown on MySQL/PG.
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
