# Known limitations

This document distinguishes confirmed product limitations from release-acceptance and test-coverage gaps. It does not describe confirmed defects unless one has been reproduced.

## Query cancellation

- PostgreSQL and MySQL support cooperative server-side cancellation for in-flight queries.
- SQLite cannot safely interrupt a synchronous `rusqlite` statement after it has started. Cancelling a long-running SQLite query therefore does not guarantee immediate termination.

## SQLite GUI release acceptance

The complete manual SQLite journey has not yet been captured as release evidence:

1. Create a SQLite connection and run a query.
2. Edit and save a Grid cell.
3. Reopen the database and confirm that the saved value persists.
4. Export the query result as CSV, JSON, and SQL.

This is an acceptance-evidence gap, not a confirmed product defect. `gridix-driver` supports only launch, keyboard input, screenshots, quit, and help; it cannot automate text entry, pointer actions, native file dialogs, or this end-to-end journey.

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
