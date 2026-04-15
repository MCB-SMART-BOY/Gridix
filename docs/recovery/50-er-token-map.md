# ER Token Map

## Scope

这份文档是 ER 主线进入实现前的最后一份准备材料。

目标：

- 盘清 ER 当前私有视觉 token
- 指定哪些 token 应回落到 Gridix 主主题系统
- 指定哪些 token 需要保留 ER 的画布特性

本文件不改代码，不直接改视觉。

当前状态补充：

- 第一波 shared token unification 已落地，范围覆盖 card/background、text、border、selection 和基础 toolbar chrome。
- 当前 `RenderColors` 已经不再只依赖 `is_dark()`，而是同时吃 `ThemePreset::colors()` 与 `egui::Visuals` helper。
- 第二波 token 也已落地：`grid_line / relation_line / pk_icon / fk_icon / table_shadow / text_type` 现在都已改成从主主题系统派生，而不是继续写死 RGB。

## 1. Confirmed Theme Sources

当前仓库里，Gridix 主 UI 的主题来源主要有两层：

### 1.1 Theme preset / theme colors

- [src/core/theme.rs](../../src/core/theme.rs)
  - `ThemePreset`
  - `ThemeColors`
  - `ThemeManager::apply()`

这里定义了：

- `bg_primary / bg_secondary / bg_tertiary`
- `fg_primary / fg_secondary / fg_muted`
- `accent / accent_hover`
- `warning / error / info / success`
- `border / border_hover`
- `selection / highlight`

### 1.2 Visuals helpers

- [src/ui/styles.rs](../../src/ui/styles.rs)
  - `theme_text()`
  - `theme_muted_text()`
  - `theme_disabled_text()`
  - `theme_accent()`
  - `theme_selection_fill()`
  - `theme_subtle_stroke()`

这些 helper 已经被 toolbar、SQL editor、dialog/picker 等高频 UI 使用。

## 2. Current ER Private Token Surface

ER 当前的私有色板集中在：

- [src/ui/components/er_diagram/render.rs](../../src/ui/components/er_diagram/render.rs)
  - `RenderColors::from_theme()`

它维护的字段有：

- `background`
- `grid_line`
- `table_bg`
- `table_header_bg`
- `table_border`
- `table_selected_border`
- `table_shadow`
- `text_primary`
- `text_secondary`
- `text_type`
- `pk_icon`
- `fk_icon`
- `relation_line`
- `row_separator`

同时，ER 的工具条按钮样式还是独立内联定义：

- `frame(false)`
- `min_size(...)`
- 图标文本颜色直接取 `theme_text(ui.visuals())`

这说明 ER 现在只有“文字颜色”与主 UI 有弱耦合，其余视觉大多是本地规则。

## 3. Mapping Strategy

### 3.1 Shared shell tokens

这些 token 应逐步回落到主主题系统：

| ER token | 目标来源 | 原因 |
|---|---|---|
| `background` | `visuals.panel_fill` 或 `ThemeColors.bg_primary` | ER 是 workspace pane，不应像独立黑盒 |
| `table_bg` | `visuals.window_fill` 或 `ThemeColors.bg_secondary` | 表卡片本质上是 ER 内部 card/surface |
| `table_header_bg` | `ThemeColors.bg_tertiary` 或 `visuals.widgets.open.bg_fill` | header 应属于主层级语义，而不是单独 hardcode |
| `table_border` | `theme_subtle_stroke(ui.visuals())` 或 `ThemeColors.border` | 边框应和 dialog/grid/tool shell 一致 |
| `table_selected_border` | `theme_accent(ui.visuals())` 或 `visuals.selection.stroke.color` | 选中应复用主 accent 语义 |
| `text_primary` | `theme_text(ui.visuals())` | 正文文字已在全局统一 |
| `text_secondary` | `theme_muted_text(ui.visuals())` | 次级信息应统一 |
| `row_separator` | `theme_subtle_stroke(ui.visuals())` 的低 alpha 版 | 行分隔线应和全局细描边风格一致 |

### 3.2 Shared interaction tokens

这些 token 应落回主交互语义：

| ER token | 目标来源 | 原因 |
|---|---|---|
| 工具条按钮 hover/selected 反馈 | `visuals.widgets.hovered/open/active` | 工具栏 chrome 应和全局一致 |
| 当前表选中填充/高亮 | `theme_selection_fill(ui.visuals(), alpha)` | 选中语义应统一 |
| loading / empty 文案颜色 | `theme_muted_text(ui.visuals())` | 空态与加载态的文案层级不应自成体系 |

### 3.3 ER-specific tokens to keep

这些 token 可以保留 ER 私有特性，但需要改成“相对主主题派生”，而不是绝对 RGB：

| ER token | 保留原因 | 约束 |
|---|---|---|
| `grid_line` | 这是画布感的一部分 | 应由背景色/描边色派生，不再写死黑白 |
| `table_shadow` | 卡片浮起感属于 ER 画布特征 | 可保留更深阴影，但应与 `DialogWindow::frame()` 的 shadow 规则同族 |
| `relation_line` | 关系线是 ER 专属语义 | 可保留独立色，但应基于 accent/border/info 派生 |
| `pk_icon` / `fk_icon` | 主键/外键需要一眼可辨 | 可以是专用语义色，但应受 light/dark 主题约束 |
| `text_type` | 数据类型文字需要次级但可读 | 可以保留单独层级，但应从 muted/accent 派生 |

## 4. Token Groups By Priority

### 4.1 First-wave token unification

第一刀最值得统一的不是全部，而是这 5 组：

当前状态：`已完成`

1. 文字层级
   - `text_primary`
   - `text_secondary`
   - `loading / empty`

2. 选中边框
   - `table_selected_border`

3. 普通边框与分隔线
   - `table_border`
   - `row_separator`

4. 工具条按钮 chrome
   - icon button hover/open/active 反馈

5. 卡片背景层级
   - `background`
   - `table_bg`
   - `table_header_bg`

这 5 组统一后，ER 就已经会明显更像 Gridix，而不需要先动关系线或图标色。

### 4.2 Second-wave token unification

第二刀再处理：

- `relation_line`
- `pk_icon / fk_icon`
- `table_shadow`
- `text_type`

因为这些更接近“ER 的画布特性”，不应一开始就被抹平。

当前状态：`已完成`

当前实现没有抹平这些语义，而是把它们改成“从主主题系统派生的 ER 私有 token”。

## 5. Concrete Mapping Recommendations

### 5.1 Toolbar chrome

当前问题：

- ER 工具条按钮全部是 `frame(false)` 的裸图标
- 只复用了文字颜色，没有复用按钮状态语义

建议：

- 保留紧凑图标按钮，但其 hover/open/selected 状态改用全局 widget visuals
- 让 ER 工具条看起来属于 Gridix toolbar family，而不是单独工具条

### 5.2 Empty / loading state

当前问题：

- 文案色和字体层级是 ER 内联定义

建议：

- 文本颜色统一落到 `theme_muted_text()`
- 若后续加入说明按钮或次级提示，再复用 `theme_text()` / `theme_accent()`

### 5.3 Table cards

当前问题：

- card/header/border/shadow 现在都是 ER 私有 RGB

建议：

- card background 从 `bg_secondary`
- header background 从 `bg_tertiary`
- normal border 从 `border`
- selected border 从 `accent`
- shadow 深度保留，但和 dialog/card 阴影家族一致

### 5.4 Relation lines and key markers

当前问题：

- 关系线和 `pk/fk` 标记现在虽然可读，但属于 ER 自说自话的色表

建议：

- 关系线优先派生自 `accent / border / info`
- PK/FK 标记保留独立语义，但统一到主主题可推导的色域，不再写死 Google 风格蓝/黄

## 6. Do Not Do First

在实现前，不建议先做：

- 全量重绘 ER toolbar 样式
- 直接换掉所有 RGB 而不建立映射表
- 先改关系线配色而不先统一文字、边框、卡片层级
- 先改视觉再补键盘 owner

## 7. Recommended First Implementation Order

当进入代码实现时，建议顺序是：

1. 收口显隐入口
2. 引入 ER 的显式 focus identity
3. 统一第一波 token：`已完成`
   - text
   - border
   - selection
   - card layers
   - toolbar chrome
4. 再处理 lifecycle / merge 语义
5. 最后统一第二波 token：`已完成`
   - relation lines
   - PK/FK markers
   - shadow nuance
   - type text

## 8. Exit Criteria Before Visual Patch

只有下面条件满足，才应进入 ER 视觉改造：

1. `show_er_diagram` 的入口已收口
2. 重复显隐字段已清理完毕，运行期显隐继续只由 `show_er_diagram` 承担
3. ER keyboard flow 已固定
4. 当前 token map 已作为实现依据固定下来
