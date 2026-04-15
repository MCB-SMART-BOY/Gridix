# Query Execution Recovery Ledger

## Scope

这份文档现在是查询执行主线的长期维护账本。  
原先分散的查询 trace、错误路径验证、镜像修复、取消语义分流与 SQL 草稿副作用修复，现都已并入这里。

后续凡是继续修这条主线，都应优先更新这份文档，而不是再追加新的短期 patch 日志。

## Current Control Trunk

当前端到端链路已经收口为：

1. `run_frame()` 渲染 SQL 编辑器，并在帧末通过 `handle_sql_editor_actions()` 收集执行意图。
2. `execute()` 为活动 tab 分配唯一 `request_id`，准备 tab 执行态，注册 pending task，并派发异步查询。
3. runtime 统一回送 `Message::QueryDone(...)`。
4. `handle_messages()` 在下一帧开头转交 `handle_query_done()`。
5. `handle_query_done()` 先写目标 tab，再只在“该 tab 仍是 active 且回包不是 stale”时更新 app-level render mirrors。
6. `sync_from_active_tab()` 负责在 tab 切换时恢复活动 tab 的镜像值。
7. render 通过活动 tab 的显式错误态和活动 mirror 决定中心区与状态栏显示。

关键文件：

- [src/ui/components/sql_editor.rs](../../src/ui/components/sql_editor.rs)
- [src/app/surfaces/render.rs](../../src/app/surfaces/render.rs)
- [src/app/runtime/database.rs](../../src/app/runtime/database.rs)
- [src/app/runtime/request_lifecycle.rs](../../src/app/runtime/request_lifecycle.rs)
- [src/app/runtime/handler.rs](../../src/app/runtime/handler.rs)
- [src/ui/components/query_tabs.rs](../../src/ui/components/query_tabs.rs)

## Current Authority Model

这条链路当前的权威边界是：

- `QueryTab` 持有 per-tab 事实：
  - `tab.sql`
  - `tab.result`
  - `tab.last_error`
  - `tab.last_message`
  - `tab.query_time_ms`
  - `tab.pending_request_id`
- app 级字段仍保留活动镜像：
  - `self.sql`
  - `self.result`
  - `self.last_query_time_ms`
- runtime request tracking 仍由 app 级集合负责：
  - `pending_query_tasks`
  - `pending_query_connections`
  - `pending_query_cancellers`
  - `user_cancelled_query_requests`

结论：

- `tab.*` 是 tab 级事实源。
- `self.result` / `self.last_query_time_ms` 是 active-tab render mirror。
- `self.sql` 仍是与 `tab.sql` 并行存在的编辑镜像，属于当前仍保留的结构性成本。

## Recovered Root Causes

### 1. 查询错误不再误渲染成 `Welcome`

已收口的问题：

- 旧错误路径曾把 active `self.result` 写成 `QueryResult::default()`
- render 又只靠 `columns.is_empty()` 判 `Welcome`
- 结果是失败查询被误当成空态

当前结果：

- `tab.last_error` 成为显式错误信号
- render 有独立 `QueryError` surface
- 非取消错误不再靠空 `QueryResult` 编码

### 2. 旧成功结果不再在失败后或新执行中复活

已收口的问题：

- 失败查询曾清空 `self.result` 但保留 `tab.result`
- 新执行曾清空 `self.result` 但保留旧 `tab.result`
- `sync_from_active_tab()` 会把旧 `tab.result` 再镜像回活动区

当前结果：

- 非取消失败会清掉目标 `tab.result`
- 新执行开始时会清掉 `tab.result`
- 切 tab 再切回不会复活旧成功结果

### 3. 活动 tab 的耗时与状态栏消息不再被非活动回包污染

已收口的问题：

- `handle_query_done()` 曾过早改写 `self.last_query_time_ms`
- SQL 编辑器状态栏曾优先读全局通知，而不是当前 tab 的 `last_message`

当前结果：

- `tab.query_time_ms` / `tab.last_message` 先落到目标 tab
- 只有 non-stale 且 active tab 回包才更新全局耗时 mirror
- SQL 编辑器优先显示 active tab 的状态消息

### 4. 用户取消与静默 stale 清理已经分离

已收口的问题：

- `pending_request_id` 过去同时承担“当前请求”和“取消判定”
- 导致用户明确取消的回包可能先被 stale gate 吞掉

当前结果：

- `user_cancelled_query_requests` 单独记录显式用户取消
- `cancel_query_request()` 与 `cancel_query_request_silently()` 已分流
- `handle_query_done()` 只丢弃真正的 stale 回包，不再吞掉显式取消语义

### 5. SQL 草稿同步不再混入 grid workspace 持久化

已收口的问题：

- `sync_sql_to_active_tab()` 过去顺带执行 `persist_active_grid_workspace()`
- 这让 SQL 文本编辑隐式触发跨 surface 状态写入

当前结果：

- `sync_sql_to_active_tab()` 只负责草稿同步
- tab 导航前的跨 surface 持久化改由 `persist_active_tab_state_for_navigation()` 执行

## Current Invariants

- 查询失败时，中心区必须进入显式错误态，而不是 `Welcome`。
- 新执行开始时，旧 `result / last_error / last_message / query_time_ms` 不能继续留在活动 tab 可见路径上。
- stale 或 inactive 回包不能越权改写 active tab 的渲染镜像。
- `QueryDone` 的用户取消语义和内部静默取消语义必须继续分开。
- tab 切换只允许通过 `sync_from_active_tab()` 恢复活动 mirror，不允许隐式跨 surface 写入。

## Remaining Structural Costs

这条主线当前没有继续挂着的已证实 open bug，但仍有 3 个结构性成本：

1. `self.sql` 与 `tab.sql` 仍是双源模型。
2. `self.result` 与 `self.last_query_time_ms` 仍是活动镜像，而不是单一权威源。
3. runtime 已支持“显式取消 vs 静默取消”，但当前仍没有稳定的显式“取消当前查询”UI 入口。

这些都应继续视为高风险边界，但在当前账本里不再算已证实 open bug。

## Validation Anchors

继续修这条主线时，至少要回归：

- `cargo test workspace_surface --lib`
- `cargo test prepare_tab_for_query_execution --lib`
- `cargo test sql_editor_status_message_prefers_active_tab_message --lib`
- `cargo test active_query_time_updates_only_for_non_stale_active_tab --lib`
- `cargo test cancelled_query_error_from_user_cancel_is_not_dropped_as_stale --lib`

如果变更触及请求派发、回包处理、tab 镜像或错误渲染，还应补跑：

- `cargo test --lib`
- `cargo test`

## Related Docs

- [10-master-recovery-plan.md](./10-master-recovery-plan.md)
- [11-core-flows-and-invariants.md](./11-core-flows-and-invariants.md)
- [12-bug-ledger-4.1.0.md](./12-bug-ledger-4.1.0.md)
