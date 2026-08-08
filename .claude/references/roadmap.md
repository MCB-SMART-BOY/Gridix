# Gridix 路线图

## ✅ v6.3.0 — 架构完成

- [x] 6 层单向依赖
- [x] DbManagerApp: ~100 → ~11 字段
- [x] Session: 30+ 字段 (async, tab, request tracking)
- [x] UiState: 60+ 字段 (theme, focus, editor, dialogs, grid, ER, search)
- [x] self.sql 消除
- [x] database → data 重命名
- [x] 安全: SSL, SSH, API, mutex
- [x] 死代码: ~800 行
- [x] Config: version, debounce
- [x] needs_repaint handler/egui 解耦
- [x] SQLite 驱动测试
- [x] 文档同步

## ✅ v7.1.0 — 合并与现代化

- [x] rustls 迁移 — 替换 native-tls/openssl，全面使用 rustls 0.23 技术栈
- [x] 分支整合 — dev、EDU、master 合并至唯一 main 分支
- [x] 状态迁移完成 — DbManagerApp 字段迁移至 UiState/Session
- [x] f32 类型修正 — 消除全部 f32 相关编译器警告
- [x] 代码清理 — 删除 check_doc_links.py、driver.sh，提取 sha256_hex 至 core/hash.rs
- [x] Workbench shell — 可停靠面板 (ActivityBar, BottomPanel, RightInspector, StatusBar)
- [x] ER 图重写 — 图模型、布局、视觉设计全面重构
- [x] Design token 系统 — UI 颜色主题化
- [x] 36 项 audit 修复 — 键盘、对话框、侧边栏、网格、ER、会话、连接池
- [x] 查询取消、网格保存修复、连接生命周期修复

## 🚧 Typed Runtime Convergence (T0–T7) — 进行中

- [x] T0: 状态基准线（本文档）
- [ ] T1: Query lifecycle clean cutover
- [ ] T2: Typed Execution production 查询主路径
- [ ] T3: SchemaCatalog application closure
- [ ] T4: Grid 完整迁移到 ResultSet + Typed Mutation
- [ ] T5: PostgreSQL/MySQL typed mutation
- [ ] T6: Final Convergence Gate



## 短期 — 功能发布

- [ ] 查询计划可视化 (EXPLAIN)
- [ ] Schema diff 工具
- [ ] 大结果集虚拟滚动
- [ ] 系统主题自动切换

## 中期 — 质量

- [ ] data/query/ 驱动测试全覆盖
- [ ] Session::poll_messages() 完整实现
- [ ] 超大文件拆分

## 长期

- [ ] 插件系统
- [ ] WebAssembly
- [ ] 多窗口
- [ ] 无障碍
