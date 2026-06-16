# Gridix 优化路线图

## 已完成（v6.2.0）

### 架构重构
- [x] 6 层单向依赖：types → core → data → session → state → ui
- [x] `src/types.rs` — Layer -1 共享类型
- [x] `src/session/` — Session(30字段) + QueryTab + Message + FrameEffects类型
- [x] `src/state/` — UiState(37字段)
- [x] `self.sql` 双源 → 单一来源
- [x] `database/` → `data/` 重命名
- [x] DbManagerApp: ~100 → ~14 字段 (86 已迁移)

### 安全
- [x] SSL Required 模式验证证书
- [x] PG 默认 SSL: Disable→Prefer, MySQL: Disabled→Preferred
- [x] SSH 密码 `#[serde(skip_serializing)]`
- [x] `pub mod app` → `pub(crate) mod app`
- [x] Mutex poison 处理

### 代码质量
- [x] 11 clippy 错误 → 0
- [x] ~800 行死代码清理
- [x] syntect + once_cell + lazy_static 移除
- [x] parking_lot::Mutex → std::sync::Mutex
- [x] config save 节流（5秒 debounce）
- [x] 3 个维度审计 + 修复

### 基础设施
- [x] PostgreSQL CI 集成测试容器
- [x] cargo-tarpaulin 覆盖率 workflow
- [x] release.yml 质量门
- [x] tests/common/mod.rs 共享测试工具
- [x] 4 个重复测试文件删除

## 短期（v6.3.0）

### Session 完善
- [ ] Session 字段封装（pub → pub(crate)，仅暴露方法）
- [ ] 完成 UiState 迁移（剩余 ~10 字段 → 目标 4 字段）
- [ ] FrameEffects 最小接入（先做 1 个处理器：ImportDone）

### 代码组织
- [ ] keybindings.rs 拆分为模块（2448 → <1000行 × 3）
- [ ] input_router.rs 拆分（3369 → 子模块）
- [ ] keybindings_dialog.rs 拆分（3560 → 子模块）

### 质量
- [ ] data/query/sqlite.rs 基础测试（内存数据库，零依赖）
- [ ] AppError 替换跨模块边界的 `Result<_, String>`
- [ ] 配置文件添加 version 字段

## 中期（v7.0.0）

### 功能
- [ ] 查询计划可视化（EXPLAIN 树形/表格）
- [ ] Schema diff/对比工具
- [ ] 系统主题自动切换
- [ ] 大结果集虚拟滚动（>100K 行）

### 工程
- [ ] Session::poll_messages() → FrameEffects 完整实现
- [ ] 数据层驱动测试全覆盖
- [ ] 性能基准测试

## 长期

- [ ] 插件系统
- [ ] WebAssembly 构建
- [ ] 多窗口支持
- [ ] 无障碍支持
