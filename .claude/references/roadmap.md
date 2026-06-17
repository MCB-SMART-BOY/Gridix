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
