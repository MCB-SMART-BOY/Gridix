# Stage 3: Implement

## Entry Criteria
- [ ] Design or direct-change scope is clear
- [ ] Safety net is identified or intentionally skipped with reason

## Activities

### General Implementation Loop

1. Make the smallest coherent change.
2. Run the fastest useful check.
3. Add/adjust tests.
4. Repeat.
5. Stop before mixing unrelated cleanup.

### Refactor Pattern

```text
characterize -> extract -> adapt -> move -> switch callers -> delete old path -> verify
```

For field/module moves:
1. Add the new target type/module first.
2. Add adapter or re-export.
3. Move one group of call sites.
4. Compile.
5. Repeat.
6. Remove old path only after tests pass.

Avoid broad search/replace when field names are common.

### Rust Fast Loop

```bash
cargo check
cargo test -p <package> <focused_test>
```

Pre-commit Rust gate:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

### Project Overlay

Apply local rules after the general process. For Gridix, check `rules/*.md` and preserve documented invariants.

Workbench surface migration:
1. Add or update `WorkbenchSurfaceKind` descriptor metadata first.
2. Preserve legacy adapters until the new dock tree is verified.
3. Keep `DockTab::surface_kind()` in sync during the transition.
4. Use `SurfaceAction`/shared tooltip contract for icon-only surface controls.
5. Route dock-tab content through the unified surface renderer before moving placement into a new dock tree.
6. Preserve legacy `FocusArea` keyboard behavior while adding `WorkbenchFocus::Surface` for surface identity.
7. Use `ensure_surface_tab()` for reveal/open migration and test duplicate reveal behavior by stable surface identity.
8. Once reveal/open paths are dock-backed, de-duplicate fixed fallback rendering before deleting compatibility regions.
9. After fallback de-duplication is implemented, switch runtime startup to the tested surface dock seed with startup/focus/config tests.
10. After the runtime seed switch is verified, migrate remaining fixed-region chrome into the shared surface shell before deleting compatibility regions.

For Gridix project-wide refactors, finish each coherent phase by updating the relevant `~/.codex/` memory, workflow references, rules, and skills in the same local change set.

## Exit Criteria
- [ ] Targeted checks pass
- [ ] No unrelated changes mixed in
- [ ] New behavior has tests or documented manual verification
- [ ] Local project rules are satisfied
- [ ] Project workflow knowledge is synchronized when architecture, UI layout, config, tests, or behavior changed

## Artifacts
- Working code changes
- Tests or measurements for the change
