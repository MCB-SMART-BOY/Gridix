# Gridix 优化路线图

## ✅ 已完成

### 架构重构
- [x] 6 层单向依赖：types(-1) → core(0) → data(1) → session(2) → state(3) → ui/app(4)
- [x] DbManagerApp: ~100 → ~11 字段 (~90 已迁移到 Session/UiState)
- [x] Session: ~30 字段，请求 ID 私有
- [x] UiState: ~60 字段 (theme, scale, focus, sidebar, editor, dialogs, grid, ER, search, welcome)
- [x] self.sql 双源消除，单一来源 = tab_manager
- [x] database/ → data/ 重命名
- [x] FrameEffects 类型定义 (session/frame_effects.rs)

### 安全
- [x] SSL Required 模式验证证书
- [x] PG 默认 Prefer, MySQL 默认 Preferred
- [x] SSH 密码 skip_serializing
- [x] pub(crate) mod app 对外保护
- [x] Mutex poison 处理

### 代码质量
- [x] 11 clippy → 0
- [x] ~800 行死代码删除
- [x] syntect/once_cell/lazy_static 依赖移除
- [x] parking_lot::Mutex → std::sync::Mutex
- [x] config save 5秒节流
- [x] 3 维度审计修复 (handler guards, cross-layer imports, state consistency)
- [x] AppError + ErrorKind 类型 (types.rs)
- [x] SQLite 驱动测试 (7 测试)

### CI/测试
- [x] PostgreSQL 集成测试 CI
- [x] cargo-tarpaulin 覆盖率 workflow
- [x] release.yml 质量门
- [x] tests/common/mod.rs 共享工具
- [x] 4 重复测试文件删除

### 文档
- [x] CLAUDE.md + 8 个 .claude/ 文件同步

## 短期（v6.3.0）

- [ ] FrameEffects 最小接线（1 个处理器 → ImportDone）
- [ ] Config 版本字段
- [ ] 继续 SQLite 测试 (execute, cancel, import)
- [ ] keybindings.rs 模块拆分（dir-based）
- [ ] keybindings_dialog.rs 拆分

## 中期（v7.0.0）

- [ ] Session::poll_messages() 完整实现
- [ ] 查询计划可视化
- [ ] 大结果集虚拟滚动
- [ ] Schema diff 工具
- [ ] 数据层测试全覆盖

## 长期

- [ ] 插件系统, WebAssembly, 多窗口, 无障碍
