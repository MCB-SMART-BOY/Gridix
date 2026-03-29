# Database Learning Curriculum | 数据库学习体系

## 1. Purpose | 目标

This document defines the structured database learning path used by Gridix help panel.  
本文定义 Gridix 帮助面板中的数据库知识学习体系。

Design goals:
- Teach concepts in dependency order.
  按依赖顺序教学。
- Connect each topic to in-app practice.
  每个知识点都能对应到应用内操作。
- Keep roadmap visible even for not-yet-implemented lessons.
  即使未实现完整课程，也在路线图中保留可见性。

## 2. Stage Map | 阶段地图

| Stage | Focus | Status |
|---|---|---|
| Fundamentals / 基础概念 | table/row/column, types, NULL | Available / 已可学 |
| Query Basics / 查询基础 | SELECT, WHERE, LIKE, GROUP BY | Available / 已可学 |
| Relationship Model / 关系模型 | PK/FK, JOIN | Available / 已可学 |
| Mutations / 写操作 | INSERT, constraints, UPDATE/DELETE, transactions | Available / 已可学 |
| Design Quality / 设计质量 | schema design, views, indexes | Planned / 规划中 |
| Advanced / 进阶专题 | subqueries, window functions, procedures, plans, backup/permission | Planned/Advanced / 规划中或进阶 |

## 3. Core Learning Path | 主干学习路径

Recommended sequence:
推荐学习顺序：
1. `数据库、表、行、列`
2. `数据类型`
3. `NULL 与空值`
4. `SELECT 基础`
5. `WHERE 与 ORDER BY`
6. `LIKE 模糊匹配`
7. `GROUP BY 聚合`
8. `主键、外键、关系`
9. `JOIN 关联查询`
10. `INSERT 新增数据`
11. `约束与默认值`
12. `UPDATE 与 DELETE`
13. `事务`

## 4. Topic-to-Action Contract | 知识点与操作契约

For each **available** topic, Gridix should provide:
对于每个 **已可学** 知识点，Gridix 应至少提供：
- Concept explanation card.
  概念说明卡片。
- Manual practice steps.
  手动练习步骤。
- Optional one-click demo action.
  可选的一键演示动作。
- Next-topic recommendation.
  下一步学习建议。

## 5. Planned Topics (Visible on Roadmap) | 路线图中可见的规划主题

Even when not fully implemented, these topics should remain visible:
即使未完整实现，也建议保持在路线图中可见：
- 表设计与规范化
- 视图
- 索引与查询性能
- 子查询
- 窗口函数
- 触发器与存储过程
- 查询执行计划与优化
- 备份、恢复与权限

## 6. Acceptance Criteria | 验收标准

### For current available stages
- Each topic has clear prerequisite text.
  每个知识点有明确前置要求。
- Each topic has at least one executable SQL example.
  每个知识点至少有一条可执行 SQL 示例。
- Learning navigation can move overview -> roadmap -> topic detail.
  学习导航支持总览 -> 路线图 -> 详情。

### For planned stages
- Topic appears on roadmap with status marker (`规划中` or `进阶`).
  知识点在路线图中显示状态标识（`规划中` 或 `进阶`）。
- Topic has concise scope statement (what this topic teaches).
  主题有简要范围说明（学什么）。

## 7. Maintenance Rules | 维护规则

- When adding/removing topics in code, update this doc in the same PR.
  代码里增删知识点时，同一 PR 必须同步更新本文档。
- Keep stage names consistent with help dialog enums.
  阶段命名保持与帮助界面枚举一致。
- If topic status changes, update roadmap text and changelog.
  知识点状态变更时，同步更新路线图文案与变更日志。
