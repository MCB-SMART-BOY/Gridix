# Architecture Decision Records (ADR)

Key architectural decisions for Gridix, with context and rationale.

## ADR-001: 6-Layer Unidirectional Dependency

**Date:** 2026-06  
**Status:** Implemented

**Context:** DbManagerApp had ~100 flat fields. No separation between DB logic, async infrastructure, UI state, and rendering.

**Decision:** 6-layer architecture: `types (-1) ← core (0) ← data (1) ← session (2) ← state (3) ← ui/app (4)`. Each layer depends only on layers below it.

**Rationale:** 
- `types`: shared types without any dependencies (Layer -1)
- `core`: pure functions, no side effects, no egui (Layer 0)
- `data`: database operations via `match db_type` dispatch (Layer 1)
- `session`: async infrastructure, request tracking, tab management (Layer 2)
- `state`: UI rendering state, no DB logic (Layer 3)
- `ui/app`: eframe App impl, rendering, input routing (Layer 4)

**Result:** DbManagerApp reduced from ~100 to ~11 fields. ~89 fields migrated to Session (~30) and UiState (~60).

---

## ADR-002: No Database Driver Trait

**Date:** 2026-06  
**Status:** Implemented

**Context:** A `DatabaseDriver` trait with ~11 operations was defined in `database/driver.rs` but never implemented.

**Decision:** Use `match db_type` dispatch in `data/query/mod.rs` instead of a trait-based abstraction.

**Rationale:** SQLite (sync, spawn_blocking), PostgreSQL (async), and MySQL (async, pooled) have fundamentally different execution models. A trait forces identical signatures onto incompatible backends. Enum dispatch allows each backend to use its natural pattern without boxing overhead.

**Result:** `database/driver.rs` deleted. Dispatch remains in `data/query/mod.rs`.

---

## ADR-003: Single Process Architecture

**Date:** 2026-06  
**Status:** Implemented

**Context:** Considered splitting into frontend (egui) + backend (database server) processes.

**Decision:** Keep single-process architecture.

**Rationale:**
- Serialization tax on QueryResult (potentially millions of cells) is prohibitive
- SSH tunnel lifecycle management across processes is complex
- Competitors (TablePlus, DBeaver, DataGrip) are single-process
- SQLite is inherently in-process

**Result:** No process separation. All DB operations use tokio::spawn/block_on within the same process.

---

## ADR-004: self.sql Single Source

**Date:** 2026-06  
**Status:** Implemented

**Context:** Both `DbManagerApp::sql` and `QueryTab::sql` existed, with bidirectional sync via `sync_sql_to_active_tab()` and `sync_from_active_tab()`.

**Decision:** Eliminate `DbManagerApp::sql`. `QueryTab::sql` is the sole authority. Editor reads/writes through `active_sql()`/`set_active_sql()` delegating to tab manager.

**Result:** `self.sql` field removed. All 13 assignment sites migrated. Sync functions simplified.

---

## ADR-005: needs_repaint Flag

**Date:** 2026-06  
**Status:** Implemented

**Context:** Handler methods received `ctx: &egui::Context` only for `ctx.request_repaint()`, coupling Session-level logic to egui types.

**Decision:** Add `needs_repaint: bool` to Session. Handlers set `self.session.needs_repaint = true`. `handle_messages()` checks the flag after the loop and calls `ctx.request_repaint()`.

**Rationale:** Decouples handler logic from rendering types without introducing FrameEffects ceremony. Simple, testable, minimal.

**Result:** 15 `ctx.request_repaint()` calls replaced. Handler ctx parameter renamed to `_ctx`.
