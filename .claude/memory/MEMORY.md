# Memory Index

- [Project Context](project-context.md) — Architecture state, constraints, active tech debt
- Gridix 2026-06-18 — Workbench refactor completed through Phase 7 RightInspector, then pivoted to Dockable Workbench v2 + UI Visual System v2. Phase B foundation is implemented. Phase C bridge now includes `WorkbenchFocus::Surface`, legacy activity/bottom/inspector-to-surface mappings, `Explain` surface, unified `DbManagerApp::render_workbench_surface_in_ui()`, `DockTab::Surface`, `default_surface_layout()`, idempotent `ensure_surface_tab()`, reveal/open action wiring for activity surfaces, BottomPanel surfaces, RightInspector surfaces, and ER, fixed-region fallback de-duplication, plus runtime startup on the surface dock seed. Fixed regions remain compatibility adapters. Next slice: migrate remaining fixed-region chrome into the unified surface shell and continue surface-first focus/config cleanup.
- User Preferences — (to be populated by user)
- Feedback Log — (to be populated during sessions)
