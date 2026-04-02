# keymap.toml Spec | keymap.toml 规范

This document defines how key bindings should be stored outside `config.toml`.  
本文定义如何将快捷键从 `config.toml` 中独立出去。

## 1. Goal | 目标

- All bindings live in `~/.config/gridix/keymap.toml`.
  所有快捷键统一存放在 `~/.config/gridix/keymap.toml`。
- Bindings are scope-aware, not only action-aware.
  快捷键必须支持作用域，而不是只有动作名。
- Missing bindings are auto-filled from defaults.
  缺失的键位可从默认模板自动补齐。
- Invalid or conflicting entries must not silently break the app.
  非法或冲突配置不能静默破坏应用。

## 2. File Location | 文件位置

- Config: `~/.config/gridix/config.toml`
- Keymap: `~/.config/gridix/keymap.toml`

`config.toml` keeps UI, theme, and connection preferences.
`config.toml` 只保留 UI、主题、连接和偏好设置。

## 3. Data Model | 数据模型

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

[sidebar.tables]
down = "j"
up = "k"
back = "h"
enter = "l"
activate = "Enter"
refresh = "R"

[sidebar.filters.list]
down = "j"
up = "k"
back = "h"
edit_value = "l"
add_below = "a"
append = "A"
delete = "x"
toggle_enabled = "Space"
toggle_logic = "o"
next_column = "]"
prev_column = "["
next_operator = "="
prev_operator = "-"

[sidebar.filters.input]
leave_input = "Escape"
confirm = "Enter"

[grid.normal]
left = "h"
down = "j"
up = "k"
right = "l"
insert = "i"
save = "Ctrl+S"

[editor.insert]
execute = "Ctrl+Enter"
autocomplete = "Ctrl+Space"
confirm_completion = "Tab"
leave_insert = "Escape"
```

## 4. Binding Grammar | 绑定语法

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

## 5. Initialization Rules | 初始化规则

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

Do not rewrite the file automatically on every startup.
不要在每次启动时自动重写文件。

## 6. Conflict Rules | 冲突规则

Conflicts are checked per scope and against inherited scopes.
冲突需要在作用域内以及继承链上检查。

Types:
类型：

- exact same scope conflict
- parent scope shadowing child scope
- global scope shadowing high-frequency local text actions

Policy:
策略：

- exact same scope conflict: error
- parent shadowing child: warning or error depending on action class
- text-entry scopes should reject plain character commands

## 7. Migration From Current Config | 从当前配置迁移

Current state stores bindings in `AppConfig.keybindings`.
当前实现把快捷键存放在 `AppConfig.keybindings`。

Migration plan:
迁移方案：

1. Load legacy bindings from `config.toml` if `keymap.toml` is absent.
2. Generate `keymap.toml`.
3. Keep `config.toml` binding field for one compatibility release only.
4. Remove legacy field after migration window.

## 8. UI Requirements | 配置界面要求

The shortcut editor should not be a flat action table anymore.
快捷键设置界面不能再是纯平的动作表格。

Required layout:
建议布局：

- left: scope tree
- center: action list within selected scope
- right: current binding, conflicts, help text, reset controls

Must support:
必须支持：

- search by action
- search by key
- show default binding
- show source (`default` / `user`)
- open `keymap.toml` location

## 9. Test Requirements | 测试要求

- missing file initialization
- partial file merge
- invalid key diagnostics
- same-scope conflict detection
- inherited conflict detection
- legacy config migration

