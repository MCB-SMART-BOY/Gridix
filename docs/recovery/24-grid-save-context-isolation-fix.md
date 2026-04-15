# Grid Edit And Save Recovery Ledger

## Scope

这份文档现在是 grid 编辑与保存主线的长期维护账本。  
当前最关键的已证实根因仍然来自 `G41-B001`：

- `GridSaveDone` 回包必须只影响它所属的 workspace / tab
- 保存成功时既不能误清当前无关页面，也不能让原目标 workspace 的旧草稿在后续复活

## Current Control Trunk

当前链路已经收口为：

1. grid 编辑在 `DataGridState` 内积累：
   - `modified_cells`
   - `rows_to_delete`
   - `new_rows`
   - `pending_sql`
   - `pending_save`
   - `show_save_confirm`
2. `confirm_pending_sql()` 触发保存请求。
3. `execute_grid_save()` 为本次保存记录 `GridSaveContext`：
   - `workspace_id`
   - `tab_id`
   - `connection_name`
   - `database_name`
   - `table_name`
4. runtime 完成后回送 `GridSaveDone`。
5. `handle_grid_save_done()` 只在“当前活动 workspace + active tab 与回包上下文匹配”时清理当前 `grid_state`。
6. 无论用户是否已切走，都会清理目标 `workspace_id` 对应的持久化保存状态。
7. `refresh_table_after_grid_save()` 只会在上下文仍匹配时回到同一张表视图。

关键文件：

- [src/ui/components/grid/mod.rs](../../src/ui/components/grid/mod.rs)
- [src/ui/components/grid/actions.rs](../../src/ui/components/grid/actions.rs)
- [src/ui/components/grid/state.rs](../../src/ui/components/grid/state.rs)
- [src/app/mod.rs](../../src/app/mod.rs)
- [src/app/runtime/database.rs](../../src/app/runtime/database.rs)
- [src/app/runtime/handler.rs](../../src/app/runtime/handler.rs)
- [tests/grid_tests.rs](../../tests/grid_tests.rs)

## Recovered Root Cause

已收口的问题：

- `handle_grid_save_done()` 曾先清当前 `self.grid_state`
- 然后才验证回包所属 `workspace_id / tab_id` 是否仍是当前页面

这会导致两类错误：

1. 用户在保存回包返回前切到别的表或别的 tab 时，当前页面可能被无关回包误清。
2. 如果只阻止误清当前页面，但不清目标 workspace 的持久化保存状态，用户切回原表时旧草稿仍可能复活。

当前结果：

- 当前活动 grid 只有在上下文匹配时才会被清理
- 目标 `workspace_id` 对应的持久化保存状态会被显式清理
- `clear_save_state()` 会同时清：
  - 编辑痕迹
  - `pending_sql`
  - `pending_save`
  - `show_save_confirm`

## Current Invariants

- 保存成功不能误清无关 tab / workspace 的当前 grid 页面。
- 保存成功后，目标 workspace 的旧草稿不能在后续切回时复活。
- 保存失败时，编辑状态和待确认 SQL 不能被静默丢弃。
- `refresh_table_after_grid_save()` 只允许回到与保存上下文匹配的同一表视图。

## Remaining Limits

这条主线当前没有新的已证实 open bug，但仍有 2 个后续观察点：

1. 保存成功且用户已经切走时，是否还需要更明确的后台刷新缓存策略。
2. grid 保存链目前仍缺少更完整的 live 端到端回归，而不是只依赖状态级测试。

## Validation Anchors

继续改这条主线时，至少要回归：

- `cargo test clear_save_state --lib`
- `cargo test grid_save_context_matches_current --lib`
- `cargo test --test grid_tests test_clear_save_state`
- `cargo test --lib`
- `cargo test`

## Related Docs

- [10-master-recovery-plan.md](./10-master-recovery-plan.md)
- [11-core-flows-and-invariants.md](./11-core-flows-and-invariants.md)
- [12-bug-ledger-4.1.0.md](./12-bug-ledger-4.1.0.md)
