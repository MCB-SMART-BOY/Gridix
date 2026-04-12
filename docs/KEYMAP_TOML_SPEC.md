# keymap.toml Spec | keymap.toml 规范

This document defines how key bindings are stored outside `config.toml` and merged into runtime bindings.
本文定义如何将快捷键从 `config.toml` 中独立出去，并合并为运行时键位。

## 1. Goal | 目标

- All bindings live in `~/.config/gridix/keymap.toml`.
  所有快捷键统一存放在 `~/.config/gridix/keymap.toml`。
- Bindings are scope-aware, not only action-aware.
  快捷键必须支持作用域，而不是只有动作名。
- Missing bindings are auto-filled from defaults.
  缺失的键位可从默认模板自动补齐。
- Invalid or conflicting entries must not silently break the app.
  非法或冲突配置不能静默破坏应用。
- `next_focus_area` / `prev_focus_area` are fallback actions, not global-first hard-coded keys.
  `next_focus_area` / `prev_focus_area` 是 fallback action，不是 global-first 的硬编码按键。

`v4.1.0` implementation note:
`v4.1.0` 实现说明：
- Runtime dispatch first resolves an input owner, then reads scoped bindings such as `dialog.common.confirm`, `dialog.export.format_csv`, `dialog.import.refresh`, or `sidebar.filters.input.leave_input`.
  运行时会先解析输入所有者，再读取 `dialog.common.confirm`、`dialog.export.format_csv`、`dialog.import.refresh` 或 `sidebar.filters.input.leave_input` 这类作用域绑定。
- Scoped command metadata lives in `src/core/commands.rs`; legacy local shortcut helpers now read command ids, descriptions, categories, and default bindings from that registry.
  作用域命令元数据位于 `src/core/commands.rs`；遗留局部快捷键 helper 现在从该注册表读取命令 id、说明、分类与默认键位。
- `DialogShortcutContext` exposes direct command-id helpers such as `consume_command` and `resolve_commands`; new dialog input code should prefer these over adding new enum-only shortcut paths.
  `DialogShortcutContext` 提供 `consume_command` 与 `resolve_commands` 等直接 command-id helper；新的对话框输入代码应优先使用这些 helper，而不是继续新增只依赖枚举的快捷键路径。
- Legacy local shortcut helpers still exist as a compatibility layer while component input handlers migrate to direct command dispatch.
  遗留局部快捷键 helper 仍作为兼容层存在，直到组件输入 handler 迁移到直接命令分发。

## 2. Runtime Data Flow | 运行时数据流

```text
defaults
  -> keymap.toml initialization/load
  -> partial merge + backfill in memory
  -> runtime bindings
  -> diagnostics
```

- Defaults come from `src/core/keybindings.rs` and `src/core/commands.rs`.
  默认值来自 `src/core/keybindings.rs` 与 `src/core/commands.rs`。
- If `~/.config/gridix/keymap.toml` is missing, Gridix creates the directory, writes a default template, and keeps runtime bindings equal to defaults.
  如果 `~/.config/gridix/keymap.toml` 不存在，Gridix 会创建目录、写入默认模板，并让运行时键位等于默认值。
- If the file exists, Gridix loads valid entries, keeps warnings/errors in diagnostics, and fills missing values from defaults in memory only.
  如果文件存在，Gridix 会加载合法项，把警告/错误保存在 diagnostics 中，并只在内存里补齐默认缺失值。
- The app does not rewrite `keymap.toml` on every startup.
  应用不会在每次启动时重写 `keymap.toml`。

## 3. File Location | 文件位置

- Config: `~/.config/gridix/config.toml`
- Keymap: `~/.config/gridix/keymap.toml`

`config.toml` keeps UI, theme, and connection preferences.
`config.toml` 只保留 UI、主题、连接和偏好设置。

## 4. Data Model | 数据模型

Each section represents a scope.
每个 section 表示一个作用域。

Example:
示例：

```toml
[global]
show_help = "F1"
new_connection = "Ctrl+N"
next_focus_area = "Tab"
prev_focus_area = "Shift+Tab"
open_keybindings = "Alt+K"

[dialog.common]
confirm = "Enter"
dismiss = ["Escape", "Q"]

[dialog.help]
scroll_up = ["K", "ArrowUp"]
scroll_down = ["J", "ArrowDown"]

[grid.normal]
copy_row = ["yy", "Y"]

[editor.insert]
execute = "Ctrl+Enter"
confirm_completion = "Tab"
history_prev = "Shift+ArrowUp"

[toolbar]
refresh = "F5"
```

Sections such as `[toolbar]`, `[query_tabs]`, `[sidebar.tables]`, and `[grid.normal]` are scope-aware action override sections. They do not bypass the router; they only provide scoped bindings that the runtime resolves before workspace fallback.
像 `[toolbar]`、`[query_tabs]`、`[sidebar.tables]`、`[grid.normal]` 这样的 section 属于 scope-aware action override。它们不会绕过输入路由，只是为运行时提供局部绑定；这些绑定会在 workspace fallback 之前按作用域解析。

Text-entry scopes such as `[editor.insert]` and `[sidebar.filters.input]` only expose actions that remain valid in text-entry mode, for example `clear_command_line` or `show_help`. Command-mode-only actions should stay out of those sections, and plain-character bindings are rejected with diagnostics.
像 `[editor.insert]`、`[sidebar.filters.input]` 这样的文本输入作用域，只应该暴露在 text-entry 模式下仍然有效的动作，例如 `clear_command_line` 或 `show_help`。仅适用于 command mode 的动作不应出现在这些 section 中，普通字符绑定也会被 diagnostics 拒绝。

`next_focus_area` and `prev_focus_area` are workspace fallback actions. They stay behind dialog scopes, text-entry child scopes, completion UIs, and local scope handlers.
`next_focus_area` 与 `prev_focus_area` 是 workspace fallback action。它们必须排在 dialog 作用域、文本输入子作用域、补全 UI 和局部作用域处理之后。

`editor.insert.confirm_completion = "Tab"` must outrank `next_focus_area = "Tab"`.
`editor.insert.confirm_completion = "Tab"` 必须优先于 `next_focus_area = "Tab"`。

`editor.insert.execute` / `editor.insert.explain` are still editor-scoped commands. They only fire when the SQL editor owns input; they must not degrade into workspace/global fallback behavior.
`editor.insert.execute` / `editor.insert.explain` 仍然是编辑器局部命令。只有 SQL 编辑器拥有输入权时它们才会触发，不能退化成 workspace/global fallback 行为。

## 5. Binding Grammar | 绑定语法

Allowed syntax:
允许的语法：

- `F1`
- `Ctrl+N`
- `Ctrl+Shift+Tab`
- `Alt+L`
- `Space`
- `Escape`
- `]`

Rules:
规则：

- Case-insensitive parsing
  解析大小写不敏感
- Output should be normalized
  写回时应规范化格式
- Reserved unsupported keys must be rejected with diagnostics
  不支持的按键必须报错并给出诊断

## 6. Initialization Rules | 初始化规则

When `keymap.toml` is missing:
当 `keymap.toml` 不存在时：

- create config directory if needed
- write default keymap template
- keep runtime bindings equal to defaults

When `keymap.toml` exists:
当 `keymap.toml` 已存在时：

- load all valid entries
- ignore unknown sections/actions with warning
- fill missing bindings from defaults in memory
- keep a diagnostics list for UI display
- keep the on-disk file untouched unless the user explicitly saves

Do not rewrite the file automatically on every startup.
不要在每次启动时自动重写文件。

## 7. Conflict Rules | 冲突规则

Conflicts are checked per scope and against inherited scopes.
冲突需要在作用域内以及继承链上检查。

Types:
类型：

- exact same scope conflict
- parent scope shadowing child scope
- global scope shadowing high-frequency local text actions
- text-entry scopes reject plain character commands

Policy:
策略：

- exact same scope conflict: error
- parent shadowing child: warning
- global/workspace fallback shadowing text-entry commands: warning
- text-entry scopes rejecting plain character commands: error

Examples:
示例：

- `dialog.common.dismiss = "Esc"` and `dialog.help.scroll_up = "Esc"` -> parent-shadowing warning
  `dialog.common.dismiss = "Esc"` 与 `dialog.help.scroll_up = "Esc"` -> 父子遮蔽 warning
- `editor.insert.confirm_completion = "J"` -> rejected as plain-character text-entry command
  `editor.insert.confirm_completion = "J"` -> 作为文本输入作用域普通字符命令被拒绝
- `next_focus_area = "Tab"` and `editor.insert.confirm_completion = "Tab"` -> valid, but runtime still resolves completion first
  `next_focus_area = "Tab"` 与 `editor.insert.confirm_completion = "Tab"` -> 配置有效，但运行时仍会先走补全确认

## 8. Migration From Current Config | 从当前配置迁移

Current state stores bindings in `AppConfig.keybindings`.
当前实现把快捷键存放在 `AppConfig.keybindings`。

Migration plan:
迁移方案：

1. On startup, prefer `keymap.toml`.
2. If `keymap.toml` is missing, initialize it from defaults, not from legacy inline bindings.
3. Keep reading `AppConfig.keybindings` only as a compatibility source and emit a migration diagnostic when legacy customizations still exist.
4. TODO: add an explicit import/migrate affordance in the shortcut editor, then remove the legacy field after the compatibility window.

## 9. UI Requirements | 配置界面要求

The shortcut editor should not be a flat action table anymore.
快捷键设置界面不能再是纯平的动作表格。

Required layout:
建议布局：

- fixed-size picker dialog
- left: scope / root selector
- center: action list within current layer
- right: current binding, conflicts, help text, reset controls

Interaction model:
交互模型：

- single click opens the next layer
- `j/k` move within the active pane
- `h` goes back to the previous layer
- `l` / `Enter` opens the current item
- no pane may grow the dialog width beyond the fixed window size

Must support:
必须支持：

- search by action
- search by key
- show default binding
- show source (`default` / `user`)
- diagnostics placeholder for parser/runtime issues
- explicit legacy-import affordance when old `config.toml.keybindings` customizations still exist
- copy the current `keymap.toml` path from the dialog

Current TODO:
当前待补：

- edit conflicts inline instead of only surfacing text diagnostics
- open `keymap.toml` location from the dialog, not only copy the path
- full binding-recording controls for every scoped action

## 10. Test Requirements | 测试要求

- missing file initialization
- partial file merge
- invalid key diagnostics
- same-scope conflict detection
- inherited conflict detection
- `editor.insert.confirm_completion(Tab)` outranks `next_focus_area(Tab)`
- legacy config migration
