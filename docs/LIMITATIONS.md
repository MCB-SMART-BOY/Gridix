# Known limitations

This document distinguishes confirmed product limitations from release-acceptance and test-coverage gaps. It does not describe confirmed defects unless one has been reproduced.

## Query cancellation

- PostgreSQL and MySQL support cooperative server-side cancellation for in-flight queries.
- SQLite cannot safely interrupt a synchronous `rusqlite` statement after it has started. Cancelling a long-running SQLite query therefore does not guarantee immediate termination.

## SQLite GUI release acceptance

The driven X11 journey was exercised on 2026-09-25 with this outcome:

| Journey step | Evidence | Status |
|---|---|---|
| Create a SQLite connection and run a query | result grid showing `SELECT * FROM "items" LIMIT 100` | captured |
| Edit and save a Grid cell | `gridix-driver assert-reopened acceptance.db items name after` passed against the file on disk | captured |
| Reopen the database and confirm the saved value | disconnect, reconnect, reload: grid shows `after` with no pending modification | captured |
| Export the result as CSV, JSON, and SQL | — | not captured |

Step 4 remains open. Export asks for a destination through `rfd::FileDialog::save_file()`,
which needs a native file chooser (on Linux an `xdg-desktop-portal` file chooser). In a driven
Xvfb session that dialog is not presented to the application, so no path comes back: no export
file is written and the 导出数据 dialog stays open without a status message. Capturing the
three exports requires an operator in a desktop session where the native dialog can appear.

This is an acceptance-evidence boundary, not a confirmed product defect. `gridix-driver`
now provides deterministic text entry (`type`), window-relative pointer actions (`move` and
`click`), bounded state-based waits (`wait window` and `wait file`), exact-PID cleanup,
read-only assertions for exported artifacts (`assert-export`, `assert-file`) and persisted
SQLite values (`assert-reopened`). For example, a release operator can run:
```text
gridix-driver assert-reopened acceptance.db items name after
gridix-driver assert-export csv acceptance.csv after
gridix-driver assert-export json acceptance.json '"name":"after"'
gridix-driver assert-export sql acceptance.sql "'after'" NULL
```

Those commands only validate the files and database supplied to them; they do not claim that
the GUI created, edited, reopened, or exported them. Native file dialogs and semantic widget
state remain outside the driver's scope, so the complete journey still requires an observed GUI
run with retained screenshots and export artifacts.

For a non-interactive launch, `gridix-driver launch --detach` returns after the window is ready
and leaves the exact-PID session state for a later `quit`. When the regular `launch` command
receives stdin EOF, it follows the same keep-alive behavior instead of cleaning up the window
immediately. A self-managed Xvfb uses a per-run Xauthority cookie in a private 0700 temporary
directory and the driver removes it on `quit`; install `xauth` with the Xvfb dependencies.
Set `GRIDIX_DISPLAY` to choose the display used by both a self-managed Xvfb and Gridix/xdotool;
set `XVFB_MANAGED=1` only when that display is managed outside the driver. These lifecycle
options make smoke orchestration reproducible but do not expand the driver's scope into full
SQLite GUI acceptance.

## Export dialog without a native file chooser

When the platform cannot present a native save dialog, confirming `导出数据` leaves the dialog
open with no status message. `handle_export_with_config` writes `export_status` only after
`rfd::FileDialog::save_file()` returns a path, so a failed or cancelled file chooser is
indistinguishable from an export that never ran. This is a feedback gap rather than data loss;
the workbook state is untouched.

## MySQL cancellation coverage boundaries

The direct MySQL 8.4 path is covered for `KILL QUERY`, observer permissions, marker disappearance, and a subsequent pool query. The following environments remain unverified:

- TLS connections;
- SSH-tunnel connections;
- execution-pool capacity pressure;
- reuse of the exact connection whose query was cancelled.

These are coverage boundaries, not known failures.

## SSH credential handling hardening

The SSH connection path needs further hardening for keyring and credential-rotation behavior:

- report missing or unreadable keyring passwords as actionable configuration errors before opening a network connection;
- preserve the existing credential reference and report a warning when persisting a replacement password fails;
- change tunnel identity after a user edits a password without storing, logging, or hashing the password itself;
- log keyring cleanup failures during connection deletion while preserving best-effort connection cleanup.

## Narrow viewport dialogs

Fixed-width content can cause horizontal overflow in narrow viewports. The known low-frequency surfaces are:

- `CreateDbDialog`;
- `CreateUserDialog`;
- `ExportDialog`.

## Coverage and maintainability

Typed integration paths have stronger coverage than some remaining driver, Grid-filter, and UI input paths. Several UI/input modules are also oversized. Any behavior-preserving split should begin with characterization coverage rather than a structural rewrite.
