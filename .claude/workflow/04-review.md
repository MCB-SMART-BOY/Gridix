# Stage 4: Review

## Entry Criteria
- [ ] All implementation complete
- [ ] `cargo check` passes

## Self-Review Checklist

### Architecture
- [ ] Layer dependency direction maintained
- [ ] No cross-layer imports (check each new `use` statement)
- [ ] `self.session.xxx` for Session fields, `self.state.xxx` for State fields
- [ ] No `self.sql` usage — uses `active_sql()`/`set_active_sql()`

### Correctness
- [ ] New async handlers have request_id stale guard
- [ ] `clear_result()` used for clearing result mirror + tab
- [ ] `save_config_debounced()` used (not direct `save_config`)
- [ ] `needs_repaint = true` in handlers (not `ctx.request_repaint()`)
- [ ] New DialogId variant handled in ALL host.rs match arms

### Style
- [ ] Follows existing patterns (emoji icons, theme colors)
- [ ] Chinese `//!` module docs, English identifiers
- [ ] `// =====...=====` section separators
- [ ] Commit message follows `type(scope): description` format

### Documentation
- [ ] CLAUDE.md updated if architecture changed
- [ ] Relevant `.claude/rules/` updated
- [ ] Roadmap updated if feature complete
- [ ] ADR created if architectural decision made

## Verification Commands

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo run --bin check-doc-links
```

## Common Issues (from audit history)

1. **Stale field references**: `self.field` on DbManagerApp after field moved to state/
   → Fix: `self.state.field`

2. **DialogId exhaustive match**: New variant not in host.rs scope_path/is_dialog_visible/open_dialog/close_dialog
   → Fix: Add to all four match sites

3. **Multiline self.field**: sed can't catch `self\n    .field` patterns
   → Fix: Manual replacement or cargo check verification

4. **ActionContext corruption**: sed replaces `self.focus_area` in ActionContext (which has its own field)
   → Fix: ActionContext methods in action_system.rs:196-245 should NOT use self.state prefix

## Exit Criteria
- [ ] All checklist items pass
- [ ] All verification commands pass
- [ ] No regression in existing tests
