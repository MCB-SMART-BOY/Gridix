# Bug ledger

From `docs/recovery/12-bug-ledger-4.1.0.md`. Current state: no active unblocked bugs.

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
