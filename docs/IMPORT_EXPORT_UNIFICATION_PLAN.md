# Import/Export Unification Plan | 导入导出统一化方案

This document describes how import/export should evolve from format-specific dialogs into one data transfer workflow.  
本文描述如何将当前按格式分裂的导入导出功能重构为统一的数据传输工作流。

## 1. Current Problems | 当前问题

- Import and export use different mental models.
  导入和导出的心智模型完全不同。
- CSV/JSON/SQL options are not symmetric.
  CSV/JSON/SQL 的配置项结构不对称。
- Preview, mapping, validation, and execution are not unified.
  预览、映射、校验、执行没有统一抽象。
- SQL export currently assumes one quoting style too often.
  SQL 导出对引用风格和方言的处理过于单一。

## 2. Product Goal | 产品目标

Users should think in terms of data transfer, not file format quirks.
用户应按“数据传输”理解功能，而不是按格式细节理解。

Unified stages:
统一阶段：

1. choose source/target
2. parse/inspect
3. map fields
4. validate types/null rules
5. preview
6. execute/export

## 3. Target Architecture | 目标架构

Introduce a shared transfer model:
引入统一传输模型：

- `TransferSession`
- `TransferFormat`
- `TransferSchema`
- `TransferMapping`
- `TransferPreview`
- `TransferExecutionPlan`

Import and export become two directions over the same pipeline.
导入和导出只是同一管线的两个方向。

## 4. Unified Capability Matrix | 统一能力矩阵

### 4.1 Formats | 格式

Initial scope:
第一阶段范围：

- CSV
- TSV
- JSON
- SQL

Later optional scope:
后续可扩展：

- Excel
- clipboard
- Parquet
- database-to-database copy

### 4.2 Shared options | 共享配置

- delimiter / quote / encoding
- table name / schema target
- null handling
- batch size
- transaction usage
- start row / row limit
- selected columns
- preview row count

## 5. Import Plan | 导入侧规划

Import pipeline should support:
导入链路应支持：

- preview before execution
- type inference with override
- column mapping to destination table
- create-table suggestion if target is missing
- error report per row or per batch

## 6. Export Plan | 导出侧规划

Export pipeline should support:
导出链路应支持：

- same column selection UI as import mapping view
- dialect-aware SQL export
- CSV/TSV/JSON options in same structure as import
- reusable export presets

## 7. UI Direction | 界面方向

Replace current one-window-per-format feeling with a staged flow:
把现在“按格式切换的大对话框”改为分阶段流程：

- stage 1: source/target + format
- stage 2: options + mapping
- stage 3: preview + execute

Keyboard requirements:
键盘要求：

- `h/l` switch stage
- `j/k` move inside current stage
- `Enter` confirm current stage action
- `Esc` back or close

## 8. Technical Work Items | 技术工作项

- unify preview structs for CSV/JSON/SQL
- separate parser result from execution plan
- add SQL dialect abstraction to export
- centralize validation and diagnostics
- reduce duplication between import/export dialogs

## 9. Risks | 风险

- UI complexity may increase if stages are not well separated.
  如果分阶段不清晰，UI 复杂度反而会上升。
- Mapping model needs careful design to avoid overfitting only CSV.
  映射模型必须避免只适配 CSV。
- SQL export dialect support may expand scope quickly.
  SQL 方言支持容易导致范围失控。

## 10. Acceptance Criteria | 验收标准

- Import and export share one option vocabulary.
  导入导出使用同一套配置术语。
- CSV/TSV/JSON/SQL all support preview before final action.
  CSV/TSV/JSON/SQL 在最终动作前都支持预览。
- SQL export can target SQLite/PostgreSQL/MySQL with explicit dialect handling.
  SQL 导出可明确面向 SQLite/PostgreSQL/MySQL 方言。

