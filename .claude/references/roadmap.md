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

## 短期 — Workbench UI 重设计

- [x] Phase 0 基线安全检查完成，完整测试和文档链接检查通过
- [x] Phase 1 Workbench 持久化配置基础完成
- [x] Phase 2 引入 WorkbenchState、shell wrapper 和 StatusBar 兼容层
- [x] Phase 3 顶部 Toolbar 移出 QueryData dock tab，成为全局 TopBar
- [x] Phase 4 引入 ActivityBar + PrimarySidebar 活动模型
- [x] Phase 5 将查询结果/错误/Explain 占位归入 BottomPanel
- [x] Phase 6 重命名并稳定 EditorArea dock tab 语义
- [x] Phase 7 引入 RightInspector 右侧检查器
- [x] Dockable Workbench v2 基础：`WorkbenchSurfaceKind/Role/Placement/Id`、descriptor 元数据和 `DockTab::surface_kind()` 过渡桥接
- [x] UI Visual System v2 基础：统一 `WorkbenchSurfaceHeader`/`SurfaceAction`、图标按钮 tooltip contract，并接入 BottomPanel/RightInspector 关闭控件
- [x] Dockable Workbench v2 Phase C 桥接：`WorkbenchFocus::Surface`、legacy 区域到 surface 映射、统一 surface renderer、`DockTab::ui()` 走 surface 渲染入口
- [x] Dockable Workbench v2 Phase C 种子布局：`DockTab::Surface`、`default_surface_layout()`、`ensure_surface_tab()`，并修复新增 SQL dock tab 可能 push 到错误 leaf 的同步风险
- [x] Dockable Workbench v2 Phase C 动作接线：Activity/BottomPanel/RightInspector/ER reveal-open 路径接入 `ensure_surface_tab()`，并保持固定区域 fallback
- [x] Dockable Workbench v2 Phase C fallback 去重：当等价 surface 已在 dock tree 中存在时，固定 PrimarySidebar/BottomPanel/RightInspector fallback 不再占用布局空间或重复渲染内容
- [x] Dockable Workbench v2 Phase C 运行时默认布局：应用启动和渲染 borrow 替换兜底都使用 `default_surface_layout()` surface seed
- [x] Dockable Workbench v2 Phase C 导航 surface 可用化：Explorer/Filters/Objects dock surface 复用真实 Sidebar 渲染器，不再显示兼容占位文本
- [x] Dockable Workbench v2 Phase C 视觉/比例校准：默认 dock split 比例命名化并调整为 VS Code/Zed 式编辑器优先布局，默认运行布局移除重复左侧 ActivityBar/SurfaceRail
- [x] Workbench April Shell v3 修正：默认布局基于 4 月稳定 PrimarySidebar，`default_surface_layout()` seed Results center / SQL editor bottom / ER right，顶栏侧边栏按钮不再增删 dock tabs
- [x] Workbench April Shell v3 默认比例锁定：按用户 2026-06-19 截图固定 `280px` PrimarySidebar、dock 中心/右侧 `0.73/0.27`、上/下 `0.69/0.31`
- [ ] Dockable Workbench v2：将 Explorer/Filters/Objects/History/Settings/Results/Tables/Inspector 统一为可拖拽 surface
- [ ] UI Visual System v2：统一面板骨架、图标优先、tooltip/快捷键提示、极简默认布局
- [ ] Phase 8 减少 Help/History/Settings 的阻塞式浮动面板
- [ ] 按 `references/project-refactor-execution-plan.md` 继续分阶段执行项目级重构
- [ ] 按 `references/workbench-ui-design.md` 和 `references/workbench-ui-refactor-spec.md` 引入稳定 WorkbenchShell
- [x] 配置层持久化 sidebar/bottom/right panel 的宽度、高度和可见性
- [x] 将持久化 workbench 配置接入运行时 state
- [x] BottomPanel 高度拖拽结果接入 debounce 保存
- [x] RightInspector 宽度拖拽结果接入 debounce 保存
- [ ] 将剩余 workbench 布局拖拽结果接入 debounce 保存

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
