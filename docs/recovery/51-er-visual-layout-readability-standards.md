# ER Visual Layout Readability Standards

## Scope

这份文档定义 Gridix 在 `ER generation redesign` 里接下来要追的**默认构图与可读性标准**。

目标不是继续围绕单一布局函数调参，而是先冻结：

- 什么样的默认 ER 完成态才算“合格”
- 当前 Gridix 与成熟数据库工具在默认构图上的差距在哪里
- 后续实现应该优先收哪些视觉/布局问题，而不是继续做猜测性补丁

本文件不实现代码，也不直接改键盘/焦点/显隐合同。

## Why This Exists

当前 ER 主线已经完成了：

- `ERGraph -> strategy selector -> layout apply`
- `DenseGraph` 独立 seed
- snapshot-aware `StableIncremental`
- geometry-aware edge anchors
- orthogonal lane assignment

但这些主要解决的是：

- 不要重叠
- 不要压线
- 不要把旧布局全部洗掉

它们还没有自动保证：

- 主关系簇占据主视觉锚点
- 断开组件被有意识地组织
- 孤立表被边缘化但不漂浮
- 长走廊边不会主导整张图
- 默认打开时一眼就能读出关系结构

当前问题已经不是“算法能不能算出来”，而是**默认构图标准还没冻结**。

## External Product Baseline

以下判断基于公开文档和公开界面特征，而不是对这些产品内部算法的臆测：

- DBeaver 把 diagrams 当成一等工作区能力来做，公开提供 `notation`、`routing`、`search`、`layout`、`outline` 等控制。
- DataGrip 把 diagrams 当成可分析、可导航、可导出的对象来做，强调 `find diagram elements`、`pan/zoom`、comments 和导出链。
- Navicat Data Modeler 明确把 `auto layout`、`align/distribute`、`layers`、notation、conceptual/logical/physical model 都当作基本能力。

对 Gridix 有用的结论不是“照抄它们的界面”，而是：

1. 成熟工具不会把可读性全部压在一次自动布局上。
2. 成熟工具默认会优先组织**主簇、组件、孤立表、边路由**，而不是只求节点彼此分开。
3. 成熟工具的默认完成态通常已经能表达“这张图主要在讲什么结构”。

## Current Gap Summary

结合当前 learning sample 与现有布局 contract，可以确认 Gridix 仍有这些结构性缺口：

1. 主簇容易被压成窄竖带。
2. 大量画布面积被浪费在无意义留白上。
3. 孤立表和弱关系簇更多是“被扔远”，而不是被有意识地安置到边缘区。
4. 长 L 形/正交走廊边虽然不再压线，但仍可能主导视觉重心。
5. 当前默认完成态更像“布局函数的输出”，还不像“经过整理的 ER 图”。

## Standard 1: Primary Cluster Anchoring

默认完成态必须先解决“主关系簇放在哪里”。

### Required

- 最大关系主簇必须优先占据主视觉区域。
- 当画布存在明显主簇和若干弱簇/孤立表时，主簇不能继续被边缘组件挤到次要区域。
- 主簇不应在画布可用宽度充足时继续退化成单一细长竖带。

### Reject

- 因为组件名、表名字典序或输入顺序，导致小型孤立簇抢占左上锚点。
- 画布大部分留白，而主簇缩成一列。

### Design Consequence

后续组件级 pack 必须把“最大主簇优先”当成显式规则，而不是 current heuristic 的偶然结果。

## Standard 2: Component Packing

断开的关系簇不是“只要分开就行”，而是必须被组织。

### Required

- 多组件图必须有稳定的 pack 规则：
  - 先按重要性排序
  - 再按目标行宽换排
  - 最后保持组件间清晰留白
- 完全不相关的组件不能继续共享一条单横带无限向右扩张。
- 多组件图必须优先减少“竖向塔式构图 + 巨大空白”的现象。

### Reject

- 一个大主簇在左下，另外几个小组件随机分散在大片空白里。
- 为了避免组件重叠而付出极端留白。

### Design Consequence

后续 `ComponentPacked` 的优化目标应从“分开组件”提升为“组织组件”。

## Standard 3: Isolated And Weakly Related Tables

孤立表和弱关系簇不是普通组件。

### Required

- 孤立表应被边缘化，但仍保持稳定、可解释的位置。
- 弱关系簇应尽量靠近语义最接近的主簇边缘，而不是随机漂在大面积空白里。
- 孤立表不应抢主视觉锚点。

### Reject

- 孤立表漂在画布上方中央，只因为名字更靠前。
- 完全无关的小表占据主阅读路径。

### Design Consequence

后续 pack/placement 需要显式区分：

- main cluster
- secondary cluster
- isolated table

而不是把它们都当普通组件。

## Standard 4: Dense Graph Composition

高密度图不能只做“分层 + refine”。

### Required

- `DenseGraph` 需要先表达：
  - root 区
  - core 区
  - leaf 区
  - bridge-heavy 的多层 core band
- 高密度图的 default completion state 必须优先减少：
  - 主簇横向摊开
  - core 全部压成一条带
  - bridge table 被甩到整个子层下面

### Reject

- 明明已有 `layer_hint`，但 bridge-heavy 图仍只形成一条 core strip。
- 高密度图只是“普通 relation 图 + 更多 force iterations”。

### Design Consequence

后续 DenseGraph 的优化方向应优先继续做：

- band 级组织
- cluster-level readable composition

而不是重新回到纯力导参数。

## Standard 5: Edge Routing Readability

边路由的目标不是“只要连上”，而是“不要主导整张图的噪声”。

### Required

- 上下堆叠表优先使用 top/bottom anchors。
- 并行边必须保持 lane separation。
- mixed L 形边必须避免共享同一根竖肘线。
- 关系线不应因默认 routing 而把整张图拉成长走廊。

### Reject

- 节点已经分层，但边仍全部走左右锚点，强行拉出长横折线。
- 多条边视觉上像只剩一条线。

### Design Consequence

在当前 lane/anchor contract 之上，后续 edge 主线如果继续推进，优先级应是：

1. cluster-aware route readability
2. lane grouping / bundling
3. 更大 pane / minimap 辅助

而不是回头重写几何锚点合同。

## Standard 6: Canvas Utilization

默认完成态必须对“画布利用率”负责。

### Required

- 在 fit-to-view 之后，主要可见区域应主要被节点与关系结构占据，而不是大面积空白。
- 若主簇已经很窄且留白巨大，应优先判定为构图问题，而不是“图已经不重叠所以算通过”。

### Reject

- 大量上下左右空白与一条细长主簇同时存在。
- 依赖用户手动拖动或缩放来“把图看起来变正常”。

## Standard 7: Default Completion State First

Gridix 不是建模器，当前阶段必须优先把**默认打开后的完成态**做好。

### Required

- 不要求用户先按 `Shift+L`、先切模式、先拖几下，图才变得可读。
- 当前主线所有布局优化都应先服务默认完成态，而不是服务“手动修图”。

### Reject

- 默认完成态只是“勉强能看”，把真正可读性交给手动 relayout。

## Evaluation Checklist

后续 ER 布局实现或 live smoke 应至少按这组问题审查：

1. 最大主簇是否占据主视觉锚点？
2. 主簇是否在可用宽度充足时仍缩成细长竖带？
3. 多组件图是否仍存在明显“漂浮组件 + 巨大留白”？
4. 孤立表是否抢占了左上或画面中心？
5. 长走廊边是否仍主导整张图？
6. DenseGraph 是否读得出 root/core/leaf 结构？
7. bridge-heavy 图是否已经利用多层 core band，而不是全部压成一条 strip？
8. 并行边和 mixed edge 是否仍在真实 schema 下塌线？
9. fit-to-view 后是否仍是“大面积空白 + 狭窄主簇”？

只要 1/2/3/4/9 中有任意一条明显失败，就不应把默认构图判为“成熟”。

## Implementation Order Consequence

基于这份标准，后续 ER 主线优先级应改成：

1. 主簇构图与组件级 pack
2. 孤立表 / 弱关系簇的边缘化安置
3. DenseGraph 的 cluster-level readable composition
4. edge readability 的更高阶策略
5. minimap / keep-layout / pin 等产品化能力

不建议继续优先做：

- 新一轮纯 force 参数调优
- 回头改 keyboard / focus / toggle
- 在没有反例时继续猜测性地微调 lane 规则

## Non-Goals For This Standard

这份标准暂不要求：

- conceptual/logical/physical model conversion
- notation 切换
- 真正建模器级别的 layer/object editing
- 全量 edge bundling 实现

它只要求 Gridix 的默认 ER 完成态**至少像一个被整理过的关系图**，而不是布局函数的直接输出。
