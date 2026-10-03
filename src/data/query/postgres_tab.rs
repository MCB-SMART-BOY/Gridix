//! One PostgreSQL backend actor per (connection, SQL document).
//!
//! The actor owns its client and an explicit transaction's borrow across requests. Closing
//! the actor drops the backend socket, so PostgreSQL rolls back any uncommitted transaction.

use super::postgres::{cancel_pg_query, execute_typed_with_client, execute_typed_with_transaction};
use crate::data::pool::{PgDedicatedConnection, PoolManager};
use crate::data::{ConnectionConfig, DbError};
use crate::domain::execution::ExecutionOutcome;
use crate::domain::ids::{ConnectionId, DocumentId};
use std::collections::HashMap;
use std::future::Future;
use std::sync::{LazyLock, Mutex};
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;

pub(super) type TabSessionKey = (ConnectionId, DocumentId);
pub type PgTabQueryReceiver = oneshot::Receiver<Result<ExecutionOutcome, DbError>>;

struct TabQuery {
    sql: String,
    cancellation: CancellationToken,
    reply: oneshot::Sender<Result<ExecutionOutcome, DbError>>,
}

struct TabSession {
    sender: mpsc::UnboundedSender<TabQuery>,
    actor: tokio::task::JoinHandle<()>,
    config_key: String,
}

impl Drop for TabSession {
    fn drop(&mut self) {
        self.actor.abort();
    }
}

static TAB_SESSIONS: LazyLock<Mutex<HashMap<TabSessionKey, TabSession>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

enum TabQueryResult {
    Complete(Result<ExecutionOutcome, DbError>),
    CloseBackend(DbError),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TransactionCommand {
    Begin,
    Commit,
    Rollback,
    Unsupported,
}

fn skip_pg_comments(sql: &str, mut cursor: usize) -> Option<usize> {
    let bytes = sql.as_bytes();
    while cursor < bytes.len() {
        if bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        } else if bytes[cursor..].starts_with(b"--") {
            cursor += 2;
            while cursor < bytes.len() && !matches!(bytes[cursor], b'\r' | b'\n') {
                cursor += 1;
            }
        } else if bytes[cursor..].starts_with(b"/*") {
            cursor += 2;
            let mut depth = 1;
            while cursor + 1 < bytes.len() && depth > 0 {
                if bytes[cursor..].starts_with(b"/*") {
                    depth += 1;
                } else if bytes[cursor..].starts_with(b"*/") {
                    depth -= 1;
                } else {
                    cursor += 1;
                    continue;
                }
                cursor += 2;
            }
            if depth > 0 {
                return None;
            }
        } else {
            break;
        }
    }
    Some(cursor)
}

/// Only one transaction-control statement per execution is supported. Other forms must not
/// be silently shortened to BEGIN/COMMIT/ROLLBACK or sent to a configuration-shared backend.
pub(crate) fn transaction_command(sql: &str) -> Option<TransactionCommand> {
    let Some(start) = skip_pg_comments(sql, 0) else {
        return Some(TransactionCommand::Unsupported);
    };
    // An empty leading statement can otherwise hide a control command from the actor.
    if sql.as_bytes().get(start) == Some(&b';') {
        return Some(TransactionCommand::Unsupported);
    }
    let mut cursor = start;
    let first = super::read_sql_keyword(sql, &mut cursor)?;
    let is_control = matches!(
        first.as_str(),
        "begin" | "start" | "commit" | "end" | "rollback" | "abort" | "savepoint" | "release"
    ) || first == "prepare" && {
        let Some(next) = skip_pg_comments(sql, cursor) else {
            return Some(TransactionCommand::Unsupported);
        };
        cursor = next;
        super::read_sql_keyword(sql, &mut cursor).as_deref() == Some("transaction")
    };
    if !is_control {
        return None;
    }
    let statement = sql[start..].trim();
    let statement = statement.strip_suffix(';').unwrap_or(statement).trim();
    let command = if statement.eq_ignore_ascii_case("BEGIN")
        || statement.eq_ignore_ascii_case("BEGIN TRANSACTION")
        || statement.eq_ignore_ascii_case("BEGIN WORK")
        || statement.eq_ignore_ascii_case("START TRANSACTION")
    {
        TransactionCommand::Begin
    } else if statement.eq_ignore_ascii_case("COMMIT")
        || statement.eq_ignore_ascii_case("COMMIT WORK")
        || statement.eq_ignore_ascii_case("END")
    {
        TransactionCommand::Commit
    } else if statement.eq_ignore_ascii_case("ROLLBACK")
        || statement.eq_ignore_ascii_case("ROLLBACK WORK")
        || statement.eq_ignore_ascii_case("ABORT")
    {
        TransactionCommand::Rollback
    } else {
        TransactionCommand::Unsupported
    };
    Some(command)
}

fn get_tab_sender(
    config: &ConnectionConfig,
    key: TabSessionKey,
    runtime: &tokio::runtime::Handle,
) -> mpsc::UnboundedSender<TabQuery> {
    let mut sessions = TAB_SESSIONS
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let config_key = config.pool_key();
    if let Some(session) = sessions.get(&key)
        && session.config_key == config_key
        && !session.sender.is_closed()
    {
        return session.sender.clone();
    }
    sessions.remove(&key);
    let (sender, receiver) = mpsc::unbounded_channel();
    let config = config.clone();
    let actor = runtime.spawn(async move { run_tab_actor(config, receiver).await });
    sessions.insert(
        key,
        TabSession {
            sender: sender.clone(),
            actor,
            config_key,
        },
    );
    sender
}

pub(super) fn enqueue_query(
    config: &ConnectionConfig,
    sql: &str,
    key: TabSessionKey,
    cancellation: &CancellationToken,
    runtime: &tokio::runtime::Handle,
) -> Result<PgTabQueryReceiver, DbError> {
    if cancellation.is_cancelled() {
        return Err(DbError::Cancelled);
    }
    let (reply, response) = oneshot::channel();
    get_tab_sender(config, key, runtime)
        .send(TabQuery {
            sql: sql.to_owned(),
            cancellation: cancellation.clone(),
            reply,
        })
        .map_err(|_| {
            DbError::Connection("PostgreSQL SQL Tab backend closed before dispatch".into())
        })?;
    Ok(response)
}

pub(super) async fn await_query(response: PgTabQueryReceiver) -> Result<ExecutionOutcome, DbError> {
    response.await.map_err(|_| {
        DbError::Connection("PostgreSQL SQL Tab backend closed during execution".into())
    })?
}

pub(super) fn close_tab(key: TabSessionKey) {
    TAB_SESSIONS
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .remove(&key);
}

pub(super) fn close_connection(connection: ConnectionId) {
    TAB_SESSIONS
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .retain(|(id, _), _| *id != connection);
}

pub(super) fn close_document(document: DocumentId) {
    TAB_SESSIONS
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .retain(|(_, id), _| *id != document);
}

async fn open_tab_backend(
    config: &ConnectionConfig,
) -> Result<(PgDedicatedConnection, ConnectionConfig), DbError> {
    let (effective_config, _tunnel) = super::setup_ssh_tunnel_if_enabled(config).await?;
    let backend = PoolManager::connect_pg_dedicated(&effective_config).await?;
    Ok((backend, effective_config))
}

fn reject_before_dispatch(
    query: TabQuery,
    error: DbError,
    receiver: &mut mpsc::UnboundedReceiver<TabQuery>,
) -> bool {
    let should_close_backend = transaction_command(&query.sql).is_some();
    let _ = query.reply.send(Err(error));
    if should_close_backend {
        receiver.close();
    }
    should_close_backend
}

fn reject_queued_after_backend_close(receiver: &mut mpsc::UnboundedReceiver<TabQuery>) {
    receiver.close();
    while let Ok(query) = receiver.try_recv() {
        let _ = query.reply.send(Err(DbError::Connection(
            "PostgreSQL SQL Tab backend closed; queued SQL was not executed".into(),
        )));
    }
}

async fn open_pending_tab_backend(
    config: &ConnectionConfig,
    receiver: &mut mpsc::UnboundedReceiver<TabQuery>,
) -> Option<(PgDedicatedConnection, ConnectionConfig, TabQuery)> {
    let timeout =
        std::time::Duration::from_secs(crate::core::constants::database::QUERY_TIMEOUT_SECS);
    while let Some(query) = receiver.recv().await {
        if query.cancellation.is_cancelled() {
            if reject_before_dispatch(query, DbError::Cancelled, receiver) {
                return None;
            }
            continue;
        }
        let setup = tokio::select! {
            result = tokio::time::timeout(timeout, open_tab_backend(config)) => {
                result.unwrap_or_else(|_| Err(DbError::Timeout {
                    operation: "connect PostgreSQL SQL Tab backend",
                    duration: timeout,
                }))
            }
            _ = query.cancellation.cancelled() => {
                if reject_before_dispatch(query, DbError::Cancelled, receiver) {
                    return None;
                }
                continue;
            }
        };
        match setup {
            Ok((backend, effective_config)) => return Some((backend, effective_config, query)),
            Err(error) => {
                if reject_before_dispatch(query, error, receiver) {
                    return None;
                }
            }
        }
    }
    None
}

async fn run_tab_actor(config: ConnectionConfig, mut receiver: mpsc::UnboundedReceiver<TabQuery>) {
    let Some((mut backend, effective_config, first_query)) =
        open_pending_tab_backend(&config, &mut receiver).await
    else {
        return;
    };
    let mut next_query = Some(first_query);
    loop {
        let query = match next_query.take() {
            Some(query) => query,
            None => match receiver.recv().await {
                Some(query) => query,
                None => return,
            },
        };
        if backend.client.is_closed() {
            let _ = query.reply.send(Err(DbError::Connection(
                "PostgreSQL SQL Tab backend closed before dispatch".into(),
            )));
            reject_queued_after_backend_close(&mut receiver);
            return;
        }
        if query.cancellation.is_cancelled() {
            if reject_before_dispatch(query, DbError::Cancelled, &mut receiver) {
                return;
            }
            continue;
        }
        let command = transaction_command(&query.sql);
        if command == Some(TransactionCommand::Begin) {
            if !begin_tab_transaction(&mut backend.client, &effective_config, &mut receiver, query)
                .await
            {
                reject_queued_after_backend_close(&mut receiver);
                return;
            }
            continue;
        }

        match command {
            Some(TransactionCommand::Unsupported) => {
                reject_queued_after_backend_close(&mut receiver);
                let _ = query.reply.send(Err(unsupported_control()));
                return;
            }
            Some(TransactionCommand::Commit | TransactionCommand::Rollback) => {
                reject_queued_after_backend_close(&mut receiver);
                let _ = query.reply.send(Err(DbError::Query(
                    "PostgreSQL SQL Tab has no active transaction".into(),
                )));
                return;
            }
            _ => {
                if !dispatch_autocommit(&mut backend, &effective_config, &mut receiver, query).await
                {
                    return;
                }
            }
        }
    }
}

async fn dispatch_autocommit(
    backend: &mut PgDedicatedConnection,
    config: &ConnectionConfig,
    receiver: &mut mpsc::UnboundedReceiver<TabQuery>,
    query: TabQuery,
) -> bool {
    let result = execute_autocommit(&mut backend.client, config, &query).await;
    let is_closed = backend.client.is_closed();
    match result {
        TabQueryResult::Complete(result) => {
            let _ = query.reply.send(result);
            if is_closed {
                reject_queued_after_backend_close(receiver);
            }
            !is_closed
        }
        TabQueryResult::CloseBackend(error) => {
            reject_queued_after_backend_close(receiver);
            let _ = query.reply.send(Err(error));
            false
        }
    }
}

async fn begin_tab_transaction(
    client: &mut tokio_postgres::Client,
    config: &ConnectionConfig,
    receiver: &mut mpsc::UnboundedReceiver<TabQuery>,
    query: TabQuery,
) -> bool {
    let transaction = tokio::select! {
        biased;
        result = client.transaction() => result,
        _ = query.cancellation.cancelled() => {
            // BEGIN may have reached the server; closing the socket resolves its state.
            let _ = query.reply.send(Err(DbError::Cancelled));
            return false;
        }
    };
    let transaction = match transaction {
        Ok(transaction) => transaction,
        Err(error) => {
            let _ = query
                .reply
                .send(Err(DbError::Query(format!("PG BEGIN: {error}"))));
            return false;
        }
    };
    if query.cancellation.is_cancelled() {
        // Never wait for a second server round trip to roll back a cancelled BEGIN.
        let _ = query.reply.send(Err(DbError::Cancelled));
        return false;
    }
    let _ = query.reply.send(Ok(ExecutionOutcome::affected_rows(0)));
    run_manual_transaction(transaction, config, receiver).await
}

async fn run_manual_transaction(
    transaction: tokio_postgres::Transaction<'_>,
    config: &ConnectionConfig,
    receiver: &mut mpsc::UnboundedReceiver<TabQuery>,
) -> bool {
    let mut has_aborted_transaction = false;
    while let Some(query) = receiver.recv().await {
        if transaction.client().is_closed() {
            let _ = query.reply.send(Err(DbError::Connection(
                "PostgreSQL SQL Tab transaction backend closed before dispatch".into(),
            )));
            return false;
        }
        if query.cancellation.is_cancelled() {
            let _ = query.reply.send(Err(DbError::Cancelled));
            continue;
        }
        match transaction_command(&query.sql) {
            Some(TransactionCommand::Commit) if has_aborted_transaction => {
                let _ = query.reply.send(Err(DbError::Query(
                    "PostgreSQL transaction is aborted; explicitly ROLLBACK before continuing"
                        .into(),
                )));
            }
            Some(TransactionCommand::Commit) => {
                let result =
                    execute_transaction_control(&query, "COMMIT", transaction.commit()).await;
                let should_keep_backend = matches!(result, TabQueryResult::Complete(Ok(_)));
                reply_control(query, result);
                return should_keep_backend;
            }
            Some(TransactionCommand::Rollback) => {
                let result =
                    execute_transaction_control(&query, "ROLLBACK", transaction.rollback()).await;
                let should_keep_backend = matches!(result, TabQueryResult::Complete(Ok(_)));
                reply_control(query, result);
                return should_keep_backend;
            }
            Some(TransactionCommand::Begin | TransactionCommand::Unsupported) => {
                let _ = query.reply.send(Err(unsupported_control()));
            }
            None => {
                if !execute_transaction_sql(
                    &transaction,
                    config,
                    query,
                    &mut has_aborted_transaction,
                )
                .await
                {
                    return false;
                }
            }
        }
    }
    true
}

async fn execute_transaction_sql(
    transaction: &tokio_postgres::Transaction<'_>,
    config: &ConnectionConfig,
    query: TabQuery,
    has_aborted_transaction: &mut bool,
) -> bool {
    let mut has_server_error = false;
    let mut has_decode_error = false;
    let statement = async {
        match execute_typed_with_transaction(transaction, &query.sql).await {
            Ok(outcome) => Ok(outcome),
            Err(error) => {
                has_server_error = matches!(error, super::postgres::PgQueryError::Server(_));
                has_decode_error = matches!(error, super::postgres::PgQueryError::Decode(_));
                Err(error.into_error())
            }
        }
    };
    let result = execute_cancellable(config, transaction.cancel_token(), &query, statement).await;
    let is_closed = transaction.client().is_closed();
    match result {
        TabQueryResult::Complete(result) => {
            let should_close = is_closed || matches!(&result, Err(DbError::Connection(_)));
            if has_server_error || (matches!(&result, Err(DbError::Cancelled)) && !has_decode_error)
            {
                *has_aborted_transaction = true;
            }
            let _ = query.reply.send(result);
            !should_close
        }
        TabQueryResult::CloseBackend(error) => {
            let _ = query.reply.send(Err(error));
            false
        }
    }
}

async fn execute_transaction_control(
    query: &TabQuery,
    command: &'static str,
    operation: impl Future<Output = Result<(), tokio_postgres::Error>>,
) -> TabQueryResult {
    tokio::select! {
        biased;
        result = operation => match result {
            Ok(()) => TabQueryResult::Complete(Ok(ExecutionOutcome::affected_rows(0))),
            Err(source) => TabQueryResult::CloseBackend(DbError::TransactionOutcomeUnknown {
                command,
                source: Some(source),
            }),
        },
        _ = query.cancellation.cancelled() => TabQueryResult::CloseBackend(
            DbError::TransactionOutcomeUnknown { command, source: None }
        ),
    }
}

fn reply_control(query: TabQuery, result: TabQueryResult) {
    let result = match result {
        TabQueryResult::Complete(result) => result,
        TabQueryResult::CloseBackend(error) => Err(error),
    };
    let _ = query.reply.send(result);
}

async fn execute_autocommit(
    client: &mut tokio_postgres::Client,
    config: &ConnectionConfig,
    query: &TabQuery,
) -> TabQueryResult {
    execute_cancellable(
        config,
        client.cancel_token(),
        query,
        execute_typed_with_client(client, &query.sql),
    )
    .await
}

fn resolve_cancelled_query(
    cancel_result: Result<(), DbError>,
    query_result: Result<ExecutionOutcome, DbError>,
) -> Result<ExecutionOutcome, DbError> {
    if cancel_result.is_err() {
        tracing::warn!("PostgreSQL SQL Tab cancel request failed; preserving query result");
        return query_result;
    }
    match query_result {
        Err(_) => Err(DbError::Cancelled),
        success => success,
    }
}

async fn execute_cancellable(
    config: &ConnectionConfig,
    token: tokio_postgres::CancelToken,
    request: &TabQuery,
    query: impl Future<Output = Result<ExecutionOutcome, DbError>>,
) -> TabQueryResult {
    tokio::pin!(query);
    tokio::select! {
        biased;
        result = &mut query => TabQueryResult::Complete(result),
        _ = request.cancellation.cancelled() => {
            let grace = std::time::Duration::from_secs(
                crate::core::constants::database::QUERY_CANCEL_GRACE_SECS,
            );
            let cancel_result = tokio::select! {
                result = tokio::time::timeout(grace, cancel_pg_query(config, token)) =>
                    match result {
                        Ok(result) => result,
                        Err(_) => return TabQueryResult::CloseBackend(DbError::Timeout {
                            operation: "send PostgreSQL SQL Tab cancellation",
                            duration: grace,
                        }),
                    },
                result = &mut query => return TabQueryResult::Complete(result),
            };
            let query_result = match tokio::time::timeout(grace, query).await {
                Ok(result) => result,
                Err(_) => return TabQueryResult::CloseBackend(DbError::Timeout {
                    operation: "await PostgreSQL SQL Tab cancellation",
                    duration: grace,
                }),
            };
            TabQueryResult::Complete(resolve_cancelled_query(cancel_result, query_result))
        }
    }
}

fn unsupported_control() -> DbError {
    DbError::Query("PostgreSQL SQL Tab supports one standalone BEGIN, COMMIT, or ROLLBACK statement per execution; transaction options or multi-statement transaction controls are unsupported".into())
}

#[cfg(test)]
mod tests {
    use super::{TransactionCommand, resolve_cancelled_query, transaction_command};

    #[test]
    fn transaction_command_single_begin_recognized() {
        assert!(transaction_command(" /* note */ BEGIN; ") == Some(TransactionCommand::Begin));
    }

    #[test]
    fn transaction_command_nested_comments_preserve_first_keyword() {
        assert!(
            transaction_command("/* outer /* inner */ tail */ ROLLBACK")
                == Some(TransactionCommand::Rollback)
        );
        assert!(
            transaction_command("/* outer /* inner */ */ BEGIN") == Some(TransactionCommand::Begin)
        );
        assert!(transaction_command("/* unterminated") == Some(TransactionCommand::Unsupported));
    }

    #[test]
    fn transaction_command_cr_and_crlf_comments_preserve_control_keywords() {
        for prefix in ["-- note\r", "-- note\r\n"] {
            assert_eq!(
                transaction_command(&format!("{prefix}BEGIN")),
                Some(TransactionCommand::Begin)
            );
            assert_eq!(
                transaction_command(&format!("{prefix}COMMIT")),
                Some(TransactionCommand::Commit)
            );
            assert_eq!(
                transaction_command(&format!("{prefix}ROLLBACK")),
                Some(TransactionCommand::Rollback)
            );
        }
    }

    #[test]
    fn resolve_cancelled_query_failed_cancel_preserves_server_result() {
        use crate::data::DbError;
        use crate::domain::execution::{ExecutionOutcome, StatementOutcome};
        let failed_cancel = || Err(DbError::Connection("control channel unavailable".into()));
        let outcome =
            resolve_cancelled_query(failed_cancel(), Ok(ExecutionOutcome::affected_rows(1)))
                .expect("server success remains authoritative");
        assert!(matches!(
            outcome.statements.as_slice(),
            [StatementOutcome::AffectedRows { rows: 1 }]
        ));
        assert!(matches!(
            resolve_cancelled_query(failed_cancel(), Err(DbError::Query("server error".into()))),
            Err(DbError::Query(_))
        ));
    }

    #[test]
    fn transaction_command_multiple_statements_rejected() {
        assert!(
            transaction_command("BEGIN; INSERT INTO t VALUES (1)")
                == Some(TransactionCommand::Unsupported)
        );
    }

    #[test]
    fn transaction_command_with_options_rejected() {
        assert!(
            transaction_command("BEGIN ISOLATION LEVEL SERIALIZABLE")
                == Some(TransactionCommand::Unsupported)
        );
    }

    #[test]
    fn transaction_command_leading_empty_statement_rejected() {
        for sql in [";ROLLBACK", " /* note */ ; COMMIT", "-- note\n; BEGIN"] {
            assert!(transaction_command(sql) == Some(TransactionCommand::Unsupported));
        }
    }

    #[test]
    fn transaction_command_savepoint_and_prepare_transaction_rejected() {
        for sql in [
            "SAVEPOINT foo",
            "RELEASE SAVEPOINT foo",
            "PREPARE TRANSACTION 'gid'",
        ] {
            assert!(transaction_command(sql) == Some(TransactionCommand::Unsupported));
        }
    }
}
