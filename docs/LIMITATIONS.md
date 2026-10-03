# Known limitations

This document distinguishes confirmed product limitations from release-acceptance and test-coverage gaps. It does not describe confirmed defects unless one has been reproduced.

## Query cancellation

- PostgreSQL and MySQL support cooperative server-side cancellation for in-flight queries.
- SQLite cannot safely interrupt a synchronous `rusqlite` statement after it has started. Cancelling a long-running SQLite query therefore does not guarantee immediate termination.

## SQLite GUI release acceptance

The driven X11 journey was exercised on 2026-09-25; its create/edit/reopen steps
and a later export run have separate evidence:

| Journey step | Evidence | Status |
|---|---|---|
| Create a SQLite connection and run a query | result grid showing `SELECT * FROM "items" LIMIT 100` (2026-09-25) | captured |
| Edit and save a Grid cell | `gridix-driver assert-reopened acceptance.db items name after` passed against the file on disk (2026-09-25) | captured |
| Reopen the database and confirm the saved value | disconnect, reconnect, reload: grid shows `after` with no pending modification (2026-09-25) | captured |
| Export the result as CSV, JSON, and SQL | GUI `导出数据` → native save chooser → three non-empty files, independently checked with `gridix-driver assert-export` (2026-10-03) | captured in a separate run |

For the export run, a private D-Bus session with `xdg-desktop-portal` and its GTK
backend presented the native file chooser on Xvfb. The GUI result showed `items`
with the `after` row before export; the resulting CSV contained
`id,name,note\n1,after,\n`, JSON contained `"name": "after"` and `"note": null`,
and SQL contained `'after'` and `NULL`. Screenshots (`portal-app-table.png`,
`chooser-csv-working.png`, `csv-after-path.png`, `json-success.png`,
`sql-success.png`) and the three files are local, disposable artifacts in
`/tmp/gridix-acceptance-2480feb4-1baf-4f56-985e-c836fbcf66f1/`; they have
not been archived with a release.

This export run used an existing SQLite fixture already containing `after`, not
the database edited in the 2026-09-25 run. It closes the native-chooser/export
evidence gap, but does **not** establish one continuous create → edit/save →
reopen → export release-acceptance journey. Capture and retain that complete
journey before accepting a release. A missing native chooser remains an
environment-dependent feedback gap, not a confirmed data-loss defect.

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
state remain outside the driver's scope. The observed export run used a native
chooser and pointer input in addition to the driver's artifact assertions; the
complete same-database journey still requires retained, observed GUI evidence.

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

## SSH fixed local port across connections

SSH tunnels are isolated per connection instance so disconnecting one cannot close another's
active forwarding stream. If two live connections use the same nonzero `local_port`, only one
can bind it; use the default `local_port = 0` for independently allocated loopback ports.

## MySQL cancellation coverage boundaries

The direct MySQL 8.4 path is covered for `KILL QUERY`, observer permissions, marker disappearance, and a subsequent pool query. The following environments remain unverified:

- TLS connections;
- SSH-tunnel connections;
- execution-pool capacity pressure;
- reuse of the exact connection whose query was cancelled.

These are coverage boundaries, not known failures.

## Typed transfer boundaries

CSV/TSV/JSON cannot round-trip arbitrary binary values, and delimited text that would be reinterpreted as NULL, a number or boolean is rejected rather than silently changed. Use SQL export with a matching target dialect for these values. Wrapped imports reject transaction controls before any write; PostgreSQL/MySQL statements containing backslashes are rejected under transaction wrapping because connection-dependent escape semantics cannot be safely inferred. Disable transaction wrapping only if partial writes are acceptable. MySQL permits only INSERT/UPDATE/DELETE/REPLACE/SELECT and rejects executable comments. Non-transactional MySQL engines or indirect transaction effects cannot be given an atomic rollback guarantee.

Successful MySQL DML can emit conversion or truncation warnings under a non-strict SQL mode without failing the import. A successful import is not proof that MySQL persisted the original values unchanged; check target data or use an appropriate strict server mode for fidelity-sensitive imports.

## Table-grid refresh while editing

When a table grid has unsaved edits, inserts, deletes or an unconfirmed save, Gridix preserves its original result baseline and refuses to replace it with an ordinary query result. Save or explicitly discard the draft before refreshing the table; to run unrelated SQL meanwhile, open another query tab. This avoids applying row-indexed edits to different primary keys after the server's row order changes.

## Uncertain transaction acknowledgements

If a PostgreSQL or MySQL grid mutation batch loses its COMMIT or ROLLBACK acknowledgement, Gridix cannot determine from the transport failure whether the batch committed. A MySQL ROLLBACK warning after non-transactional writes is treated the same way: it retains and locks that table workspace's edits rather than claiming rollback or retrying. Verify database state independently, discard the uncertain draft explicitly, then refresh the table successfully before editing. A wrapped import with an uncertain COMMIT/ROLLBACK or a MySQL incomplete-rollback warning, and an unwrapped import with a lost statement acknowledgement, require explicit verification and acknowledgement in the import dialog before another import. Non-transactional MySQL engines remain outside the atomicity guarantee.

## SSH credential and tunnel acceptance

Configuration changes now report missing keyring credentials, retain credential references on persistence failures, rotate tunnel identity after password edits, and log keyring cleanup failures. Explicitly clearing a database password revokes its saved reference; config parser logs no source lines, and connection previews mask encoded passwords.

Tunnel shutdown now owns and closes active forwarding tasks. The SSH fixture tests for stopping an active forwarded connection and concurrent first-creation cleanup still require an observed run with a reachable SSH host and PostgreSQL backend; local tests without that fixture are not acceptance evidence.

## Narrow viewport dialogs

Fixed-width content can cause horizontal overflow in narrow viewports. The known low-frequency surfaces are:

- `CreateDbDialog`;
- `CreateUserDialog`;
- `ExportDialog`.

## Coverage and maintainability

Typed integration paths have stronger coverage than some remaining driver, Grid-filter, and UI input paths. Several UI/input modules are also oversized. Any behavior-preserving split should begin with characterization coverage rather than a structural rewrite.
