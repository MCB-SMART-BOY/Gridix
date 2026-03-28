# Documentation Style Guide | 文档规范

## 1. Scope | 适用范围

This guide applies to `README.md` and all files under `docs/`.  
本规范适用于 `README.md` 与 `docs/` 下全部文档。

## 2. Language Policy | 语言策略

- Prefer bilingual in one page (EN + 中文).
  优先中英同页。
- Keep sentence pairs aligned in meaning.
  中英文语义保持对齐。
- Avoid mixing outdated and current terms in one section.
  同一章节避免新旧术语混用。

## 3. Structure Rules | 结构规则

- Use clear headings with stable numbering where needed.
  使用清晰标题，必要时使用稳定编号。
- Keep sections practical: what, when, how.
  章节内容尽量围绕“是什么、何时用、怎么做”。
- Prefer short paragraphs and checklist-style instructions.
  优先短段落与可执行清单。

## 4. Fact Accuracy Rules | 事实准确性

- Do not document shortcuts/features that are not implemented.
  未实现功能不得写入文档。
- Version-specific claims must be tied to changelog entries.
  版本相关描述应与变更日志一致。
- Paths/config keys must match code (`src/core/config.rs` etc.).
  路径与配置字段必须与代码一致（如 `src/core/config.rs`）。

## 5. Link & Navigation Rules | 链接与导航规范

- Every new doc must be linked from `docs/README.md`.
  新文档必须加入 `docs/README.md` 索引。
- User-facing docs should be discoverable from root `README.md`.
  面向用户的文档应在根 README 可见。
- No broken relative links.
  不允许失效相对链接。

## 6. Change Management | 变更管理

- Behavior change -> update docs in same PR.
  行为变更需在同一 PR 更新文档。
- User-visible change -> update `docs/CHANGELOG.md`.
  面向用户的变更需更新 `docs/CHANGELOG.md`。
- Topic/learning-path change -> update `docs/LEARNING_CURRICULUM.md`.
  学习路线变更需更新 `docs/LEARNING_CURRICULUM.md`。

## 7. Recommended Doc Checklist | 建议检查清单

Before merge, verify:
合并前检查：
- Links valid.
- Terminology consistent.
- Commands copy-pastable.
- Default behavior matches current release.

Recommended command:
建议命令：
```bash
python scripts/check_doc_links.py
```

合并前确认：
- 链接可用。
- 术语一致。
- 命令可直接复制执行。
- 默认行为与当前版本一致。
