//! Connection/database transitions must not relabel grid drafts or query results.

use super::DbManagerApp;
use crate::app::GridSubmittedEdits;
use crate::app::action::action_system::AppAction;
use crate::data::ImportExecutionReport;
use crate::data::{ConnectionConfig, DatabaseType};
use crate::domain::execution::ExecutionOutcome;
use crate::domain::ids::{ConnectionId, TableViewId, TaskId};
use crate::domain::mutation::{ColumnRef, InputValue, Mutation, MutationBatch};
use crate::domain::result::ResultColumn;
use crate::domain::result::ResultSet;
use crate::domain::value::{DbTypeFamily, DbTypeInfo, DbValue};
use crate::session::runtime_event::{RuntimeEvent, RuntimeOutcome};
use crate::session::task_registry::{OperationKey, TaskKind};
use eframe::egui;
use std::sync::Arc;

fn add_connections(app: &mut DbManagerApp, database: Option<&str>) {
    for name in ["first", "second"] {
        app.session
            .manager
            .add(ConnectionConfig::new(name, DatabaseType::SQLite));
        if let Some(database) = database {
            app.session
                .manager
                .connections
                .get_mut(name)
                .unwrap()
                .set_database(database.into(), vec![]);
        }
    }
    app.session.manager.active = Some("first".into());
}

fn activate_second_connection(app: &mut DbManagerApp) {
    app.switch_grid_workspace(None);
    app.session.manager.active = Some("second".into());
    app.switch_grid_workspace(Some("users".into()));
}

#[test]
fn connect_refresh_keeps_network_connection_database_selection() {
    let mut app = DbManagerApp::new_for_test();
    app.session
        .manager
        .add(ConnectionConfig::new("pg", DatabaseType::PostgreSQL));
    let connection_id = app.session.manager.connections["pg"].id;
    app.session
        .manager
        .connections
        .get_mut("pg")
        .unwrap()
        .set_database("db1".into(), vec!["users".into()]);
    app.session.pending_connect_requests.insert("pg".into());

    app.handle_connected_with_tables(
        &egui::Context::default(),
        connection_id,
        "pg".into(),
        Ok(vec!["db1".into(), "db2".into()]),
    );

    let connection = &app.session.manager.connections["pg"];
    assert_eq!(connection.selected_database.as_deref(), Some("db1"));
    assert_eq!(connection.databases, vec!["db1", "db2"]);
    assert!(connection.tables.is_empty());
}

#[test]
fn table_delete_only_clears_target_connection_drafts() {
    let mut app = DbManagerApp::new_for_test();
    add_connections(&mut app, None);
    app.switch_grid_workspace(Some("users".into()));
    app.state
        .grid_state
        .new_rows
        .push(vec!["deleted table draft".into()]);
    let first_id = app.session.manager.connections["first"].id;
    activate_second_connection(&mut app);
    app.state
        .grid_state
        .new_rows
        .push(vec!["other connection draft".into()]);
    app.persist_active_grid_workspace();

    app.handle_table_dropped(
        &egui::Context::default(),
        first_id,
        "first".into(),
        None,
        "users".into(),
        Ok(()),
    );
    assert_eq!(
        app.state.grid_state.new_rows,
        vec![vec!["other connection draft"]]
    );
    app.switch_grid_workspace(None);
    app.session.manager.active = Some("first".into());
    app.switch_grid_workspace(Some("users".into()));
    assert!(app.state.grid_state.new_rows.is_empty());
}

#[test]
fn active_table_delete_does_not_restore_drafts_to_recreated_table() {
    let mut app = DbManagerApp::new_for_test();
    app.state.show_er_diagram = false;
    app.session
        .manager
        .add(ConnectionConfig::new("first", DatabaseType::SQLite));
    app.session.manager.active = Some("first".into());
    let connection_id = app.session.manager.get_active().unwrap().id;
    app.switch_grid_workspace(Some("users".into()));
    app.state
        .grid_state
        .new_rows
        .push(vec!["deleted draft".into()]);
    app.state.grid_state.save_in_flight = true;

    app.handle_table_dropped(
        &egui::Context::default(),
        connection_id,
        "first".into(),
        None,
        "users".into(),
        Ok(()),
    );
    app.switch_grid_workspace(Some("users".into()));
    assert!(app.state.grid_state.new_rows.is_empty());
    assert!(!app.state.grid_state.save_in_flight);
}

#[test]
fn database_delete_only_clears_target_connection_drafts() {
    let mut app = DbManagerApp::new_for_test();
    add_connections(&mut app, Some("db1"));
    app.switch_grid_workspace(Some("users".into()));
    app.state
        .grid_state
        .new_rows
        .push(vec!["deleted db draft".into()]);
    activate_second_connection(&mut app);
    app.state
        .grid_state
        .new_rows
        .push(vec!["other db draft".into()]);
    app.persist_active_grid_workspace();
    let first_id = app.session.manager.connections["first"].id;

    app.handle_database_dropped(
        &egui::Context::default(),
        first_id,
        "first".into(),
        "db1".into(),
        Ok(()),
    );
    assert_eq!(app.state.grid_state.new_rows, vec![vec!["other db draft"]]);
    app.switch_grid_workspace(None);
    app.session.manager.active = Some("first".into());
    app.switch_grid_workspace(Some("users".into()));
    assert!(app.state.grid_state.new_rows.is_empty());
}

#[test]
fn stale_tab_result_cannot_bind_to_second_connection() {
    let mut app = DbManagerApp::new_for_test();
    add_connections(&mut app, None);
    let first_id = app.session.manager.connections["first"].id;
    let result = Arc::new(ResultSet::empty());
    let tab = &mut app.session.tab_manager.tabs[0];
    tab.result_set = Some(result.clone());
    tab.result_origin = Some((first_id, None));
    tab.selected_table = Some("users".into());
    tab.uses_grid_workspace = true;
    app.session.tab_manager.new_tab();
    app.switch_grid_workspace(None);
    app.session.manager.active = Some("second".into());

    app.activate_query_tab(0);
    assert!(app.state.selected_table.is_none());
    assert!(app.state.grid_state.result_set.is_none());
    assert!(!app.active_grid_workspace_enabled);

    app.session.manager.active = Some("first".into());
    app.sync_from_active_tab();
    assert_eq!(app.state.selected_table.as_deref(), Some("users"));
    assert!(Arc::ptr_eq(
        app.state.grid_state.result_set.as_ref().unwrap(),
        &result
    ));
}

fn result_with_ids(ids: &[i64]) -> Arc<ResultSet> {
    Arc::new(ResultSet {
        columns: Arc::new([ResultColumn {
            name: "id".into(),
            type_info: DbTypeInfo {
                family: DbTypeFamily::Integer,
                native_name: "INTEGER".into(),
                nullable: None,
            },
        }]),
        cells: ids.iter().copied().map(DbValue::Int).collect(),
        row_count: ids.len(),
        completeness: crate::domain::result::ResultCompleteness::Complete,
    })
}

fn bind_old_table_result(app: &mut DbManagerApp, result: &Arc<ResultSet>) {
    let connection_id = app.session.manager.get_active().expect("connection").id;
    let tab = app.session.tab_manager.get_active_mut().expect("tab");
    tab.result_set = Some(result.clone());
    tab.result_origin = Some((connection_id, None));
    tab.selected_table = Some("users".into());
    tab.uses_grid_workspace = true;
    app.switch_grid_workspace(Some("users".into()));
    app.state.grid_state.result_set = Some(result.clone());
}

#[test]
fn execute_with_modified_row_keeps_original_primary_key_baseline() {
    let mut app = DbManagerApp::new_for_test();
    add_connections(&mut app, None);
    let original = result_with_ids(&[11, 22]);
    bind_old_table_result(&mut app, &original);
    app.state
        .grid_state
        .modified_cells
        .insert((0, 0), "updated".into());

    assert!(
        app.execute("SELECT * FROM users ORDER BY id DESC".into())
            .is_none()
    );
    assert!(Arc::ptr_eq(
        app.state
            .grid_state
            .result_set
            .as_ref()
            .expect("draft baseline"),
        &original
    ));
    assert!(Arc::ptr_eq(
        app.session
            .tab_manager
            .get_active()
            .unwrap()
            .result_set
            .as_ref()
            .unwrap(),
        &original
    ));
    assert_eq!(
        app.state.grid_state.result_set.as_ref().unwrap().cell(0, 0),
        &DbValue::Int(11)
    );
    assert_eq!(app.state.grid_state.modified_cells[&(0, 0)], "updated");
}

#[test]
fn refresh_with_pending_deletion_keeps_original_row_and_draft() {
    let mut app = DbManagerApp::new_for_test();
    add_connections(&mut app, None);
    let original = result_with_ids(&[11, 22]);
    bind_old_table_result(&mut app, &original);
    app.state.grid_state.rows_to_delete.push(0);

    app.dispatch_app_action(&egui::Context::default(), AppAction::RefreshSelectedTable);
    assert_eq!(app.state.grid_state.rows_to_delete, [0]);
    assert!(Arc::ptr_eq(
        app.state
            .grid_state
            .result_set
            .as_ref()
            .expect("original baseline"),
        &original
    ));
    assert!(
        app.session
            .tab_manager
            .get_active()
            .unwrap()
            .pending_request_id
            .is_none()
    );
}

#[test]
fn query_completion_after_grid_edit_keeps_existing_baseline_and_draft() {
    let mut app = DbManagerApp::new_for_test();
    add_connections(&mut app, None);
    let original = result_with_ids(&[11, 22]);
    bind_old_table_result(&mut app, &original);
    let request_id = app
        .execute("SELECT * FROM users ORDER BY id DESC".into())
        .expect("query");
    assert!(Arc::ptr_eq(
        app.state
            .grid_state
            .result_set
            .as_ref()
            .expect("visible old rows"),
        &original
    ));
    app.state.grid_state.rows_to_delete.push(0);
    let connection_id = app.session.manager.get_active().unwrap().id;
    let tab_id = app.session.tab_manager.get_active().unwrap().id.clone();
    let document = super::super::request_lifecycle::query_document_id(&tab_id);
    let key = OperationKey::Query {
        connection: connection_id,
        document,
    };
    let (task_id, _) = app
        .session
        .task_registry
        .register(key.clone(), TaskKind::Query);
    app.handle_runtime_event(
        &egui::Context::default(),
        RuntimeEvent {
            task_id,
            key,
            outcome: RuntimeOutcome::ExecutionFinished {
                document,
                request_id,
                sql: "SELECT * FROM users ORDER BY id DESC".into(),
                connection_name: "first".into(),
                tab_id,
                result: Ok(ExecutionOutcome::single_result(
                    (*result_with_ids(&[22, 11])).clone(),
                )),
                transaction_completion: None,
                elapsed_ms: 1,
            },
        },
    );
    assert_eq!(app.state.grid_state.rows_to_delete, [0]);
    assert!(Arc::ptr_eq(
        app.state.grid_state.result_set.as_ref().unwrap(),
        &original
    ));
    assert!(Arc::ptr_eq(
        app.session
            .tab_manager
            .get_active()
            .unwrap()
            .result_set
            .as_ref()
            .unwrap(),
        &original
    ));
}

#[test]
fn sidebar_table_query_binds_tab_and_connection_switch_preserves_old_tab() {
    let mut app = DbManagerApp::new_for_test();
    add_connections(&mut app, None);
    app.switch_grid_workspace(Some("users".into()));
    app.dispatch_app_action(&egui::Context::default(), AppAction::QuerySelectedTable);
    let tab = app.session.tab_manager.get_active().unwrap();
    assert_eq!(tab.selected_table.as_deref(), Some("users"));
    assert!(tab.uses_grid_workspace);
    let first_id = app.session.manager.connections["first"].id;
    let original = result_with_ids(&[11]);
    app.session.tab_manager.get_active_mut().unwrap().result_set = Some(original.clone());
    app.session
        .tab_manager
        .get_active_mut()
        .unwrap()
        .result_origin = Some((first_id, None));
    app.state.grid_state.result_set = Some(original.clone());
    app.state.grid_state.new_rows.push(vec!["draft".into()]);

    app.open_new_query_tab();
    app.connect("second".into());
    app.activate_query_tab(0);
    assert!(app.state.grid_state.result_set.is_none());
    assert!(app.state.selected_table.is_none());
    assert_eq!(
        app.session
            .tab_manager
            .get_active()
            .unwrap()
            .selected_table
            .as_deref(),
        Some("users")
    );
    app.connect("first".into());
    app.sync_from_active_tab();
    assert_eq!(app.state.selected_table.as_deref(), Some("users"));
    assert_eq!(app.state.grid_state.new_rows, [vec!["draft"]]);
    assert!(Arc::ptr_eq(
        app.state.grid_state.result_set.as_ref().unwrap(),
        &original
    ));
}

fn add_network_connections(app: &mut DbManagerApp, db_type: DatabaseType) {
    for name in ["first", "second"] {
        app.session
            .manager
            .add(ConnectionConfig::new(name, db_type));
        app.session
            .manager
            .connections
            .get_mut(name)
            .expect("connection")
            .set_database("app_db".into(), vec!["users".into()]);
    }
    app.session.manager.active = Some("first".into());
}

fn select_users_from_sidebar(app: &mut DbManagerApp, ctx: &egui::Context) {
    app.handle_sidebar_actions(
        ctx,
        crate::ui::SidebarActions {
            query_table: Some("users".into()),
            ..Default::default()
        },
    );
}

fn bind_active_query_result(app: &mut DbManagerApp, result: &Arc<ResultSet>) {
    let connection = app.session.manager.get_active().expect("connection");
    let tab = app.session.tab_manager.get_active_mut().expect("query tab");
    tab.result_origin = Some((connection.id, connection.selected_database.clone()));
    tab.result_set = Some(result.clone());
    tab.pending_request_id = None;
    tab.executing = false;
    app.state.grid_state.result_set = Some(result.clone());
}

fn insert_users_batch() -> MutationBatch {
    let mut batch = MutationBatch::new();
    batch.mutations.push(Mutation::Insert {
        table: ColumnRef {
            name: "users".into(),
        },
        columns: vec![ColumnRef { name: "id".into() }],
        values: vec![InputValue::Value(DbValue::Int(12))],
    });
    batch
}

#[test]
fn sidebar_restored_draft_rebinds_original_result_for_save_and_tab_navigation() {
    for db_type in [DatabaseType::PostgreSQL, DatabaseType::MySQL] {
        let mut app = DbManagerApp::new_for_test();
        add_network_connections(&mut app, db_type);
        let ctx = egui::Context::default();
        select_users_from_sidebar(&mut app, &ctx);
        let a_result = result_with_ids(&[11]);
        bind_active_query_result(&mut app, &a_result);
        app.state.grid_state.new_rows.push(vec!["12".into()]);
        let a_id = app.session.manager.get_active().unwrap().id;

        app.switch_grid_workspace(None);
        app.session.manager.active = Some("second".into());
        select_users_from_sidebar(&mut app, &ctx);
        let b_result = result_with_ids(&[22]);
        bind_active_query_result(&mut app, &b_result);
        let b_id = app.session.manager.get_active().unwrap().id;
        assert_eq!(
            app.session.tab_manager.get_active().unwrap().result_origin,
            Some((b_id, Some("app_db".into())))
        );

        app.switch_grid_workspace(None);
        app.session.manager.active = Some("first".into());
        select_users_from_sidebar(&mut app, &ctx);
        assert_eq!(app.state.grid_state.new_rows, [vec!["12"]]);
        assert!(Arc::ptr_eq(
            app.state.grid_state.result_set.as_ref().unwrap(),
            &a_result
        ));
        let tab = app.session.tab_manager.get_active().unwrap();
        assert_eq!(tab.result_origin, Some((a_id, Some("app_db".into()))));
        assert_eq!(tab.selected_table.as_deref(), Some("users"));
        assert!(tab.uses_grid_workspace);
        assert!(Arc::ptr_eq(tab.result_set.as_ref().unwrap(), &a_result));
        assert!(tab.pending_request_id.is_none());

        app.execute_grid_save_typed("users".into(), insert_users_batch());
        assert_eq!(app.pending_grid_saves.len(), 1, "save identity accepted");
        assert!(app.state.grid_state.save_in_flight);
        app.open_new_query_tab();
        app.activate_query_tab(0);
        assert_eq!(app.state.grid_state.new_rows, [vec!["12"]]);
        assert!(app.state.grid_state.save_in_flight);
        assert!(Arc::ptr_eq(
            app.state.grid_state.result_set.as_ref().unwrap(),
            &a_result
        ));
        assert!(Arc::ptr_eq(
            app.session
                .tab_manager
                .get_active()
                .unwrap()
                .result_set
                .as_ref()
                .unwrap(),
            &a_result
        ));
    }
}

#[test]
fn sidebar_restored_draft_rejects_foreign_tab_result_even_when_baseline_arc_matches() {
    let mut app = DbManagerApp::new_for_test();
    add_network_connections(&mut app, DatabaseType::MySQL);
    let ctx = egui::Context::default();
    select_users_from_sidebar(&mut app, &ctx);
    let shared_result = result_with_ids(&[11]);
    bind_active_query_result(&mut app, &shared_result);
    app.state.grid_state.new_rows.push(vec!["12".into()]);

    app.switch_grid_workspace(None);
    app.session.manager.active = Some("second".into());
    select_users_from_sidebar(&mut app, &ctx);
    bind_active_query_result(&mut app, &shared_result);
    let foreign_id = app.session.manager.get_active().unwrap().id;

    app.switch_grid_workspace(None);
    app.session.manager.active = Some("first".into());
    select_users_from_sidebar(&mut app, &ctx);
    assert_eq!(app.state.grid_state.new_rows, [vec!["12"]]);
    assert_eq!(
        app.session.tab_manager.get_active().unwrap().result_origin,
        Some((foreign_id, Some("app_db".into())))
    );
    app.execute_grid_save_typed("users".into(), insert_users_batch());
    assert!(app.pending_grid_saves.is_empty());
}

#[test]
fn sidebar_restored_pending_save_rebinds_original_result_without_requery() {
    let mut app = DbManagerApp::new_for_test();
    add_network_connections(&mut app, DatabaseType::PostgreSQL);
    let ctx = egui::Context::default();
    select_users_from_sidebar(&mut app, &ctx);
    let original = result_with_ids(&[11]);
    bind_active_query_result(&mut app, &original);
    let first_id = app.session.manager.get_active().unwrap().id;
    app.state.grid_state.new_rows.push(vec!["12".into()]);
    record_pending_save(&mut app);

    app.switch_grid_workspace(None);
    app.session.manager.active = Some("second".into());
    select_users_from_sidebar(&mut app, &ctx);
    let foreign = result_with_ids(&[22]);
    bind_active_query_result(&mut app, &foreign);
    app.switch_grid_workspace(None);
    app.session.manager.active = Some("first".into());
    select_users_from_sidebar(&mut app, &ctx);
    assert!(app.state.grid_state.save_in_flight);
    assert_eq!(app.pending_grid_saves.len(), 1);
    assert!(app.state.grid_state.has_changes());
    assert!(Arc::ptr_eq(
        app.session
            .tab_manager
            .get_active()
            .unwrap()
            .result_set
            .as_ref()
            .unwrap(),
        &original
    ));
    assert_eq!(
        app.session.tab_manager.get_active().unwrap().result_origin,
        Some((first_id, Some("app_db".into())))
    );
    assert!(
        app.session
            .tab_manager
            .get_active()
            .unwrap()
            .pending_request_id
            .is_none()
    );
}

fn deliver_foreign_query_result(
    app: &mut DbManagerApp,
    connection_id: ConnectionId,
    tab_id: String,
    request_id: u64,
    task_id: TaskId,
) {
    let document = super::super::request_lifecycle::query_document_id(&tab_id);
    app.handle_runtime_event(
        &egui::Context::default(),
        RuntimeEvent {
            task_id,
            key: OperationKey::Query {
                connection: connection_id,
                document,
            },
            outcome: RuntimeOutcome::ExecutionFinished {
                document,
                request_id,
                sql: "SELECT * FROM users".into(),
                connection_name: "second".into(),
                tab_id,
                result: Ok(ExecutionOutcome::single_result(
                    (*result_with_ids(&[22])).clone(),
                )),
                transaction_completion: None,
                elapsed_ms: 1,
            },
        },
    );
}

#[test]
fn sidebar_restored_draft_ignores_pending_foreign_query_before_tab_navigation() {
    const FOREIGN_REQUEST_ID: u64 = 42;
    let mut app = DbManagerApp::new_for_test();
    add_network_connections(&mut app, DatabaseType::PostgreSQL);
    let ctx = egui::Context::default();
    select_users_from_sidebar(&mut app, &ctx);
    bind_active_query_result(&mut app, &result_with_ids(&[11]));
    app.state.grid_state.rows_to_delete.push(0);
    app.switch_grid_workspace(None);
    app.session.manager.active = Some("second".into());
    select_users_from_sidebar(&mut app, &ctx);
    bind_active_query_result(&mut app, &result_with_ids(&[22]));

    let connection_id = app.session.manager.get_active().unwrap().id;
    let tab_id = app.session.tab_manager.get_active().unwrap().id.clone();
    let document = super::super::request_lifecycle::query_document_id(&tab_id);
    let (task_id, _) = app.session.task_registry.register(
        OperationKey::Query {
            connection: connection_id,
            document,
        },
        TaskKind::Query,
    );
    app.session
        .tab_manager
        .get_active_mut()
        .unwrap()
        .pending_request_id = Some(FOREIGN_REQUEST_ID);
    app.switch_grid_workspace(None);
    app.session.manager.active = Some("first".into());
    select_users_from_sidebar(&mut app, &ctx);
    assert_eq!(app.state.grid_state.rows_to_delete, [0]);

    app.switch_grid_workspace(None);
    app.session.manager.active = Some("second".into());
    deliver_foreign_query_result(&mut app, connection_id, tab_id, FOREIGN_REQUEST_ID, task_id);
    app.switch_grid_workspace(None);
    app.session.manager.active = Some("first".into());
    app.open_new_query_tab();
    app.activate_query_tab(0);
    assert_eq!(app.state.grid_state.rows_to_delete, [0]);
    assert_eq!(
        app.state.grid_state.result_set.as_ref().unwrap().cell(0, 0),
        &DbValue::Int(11),
    );
}

#[test]
fn sidebar_same_name_recreated_connection_does_not_rebind_old_draft() {
    let mut app = DbManagerApp::new_for_test();
    add_network_connections(&mut app, DatabaseType::PostgreSQL);
    let ctx = egui::Context::default();
    select_users_from_sidebar(&mut app, &ctx);
    let old_result = result_with_ids(&[11]);
    bind_active_query_result(&mut app, &old_result);
    app.state.grid_state.new_rows.push(vec!["old draft".into()]);
    let old_workspace = app.active_grid_workspace_id().expect("old workspace");
    let old_connection_id = app.session.manager.get_active().unwrap().id;

    let mut replacement = ConnectionConfig::new("first", DatabaseType::PostgreSQL);
    replacement.database = "app_db".into();
    app.session.manager.add(replacement);
    app.session
        .manager
        .connections
        .get_mut("first")
        .unwrap()
        .set_database("app_db".into(), vec!["users".into()]);
    assert_ne!(
        app.session.manager.get_active().unwrap().id,
        old_connection_id
    );

    select_users_from_sidebar(&mut app, &ctx);
    assert!(app.state.grid_state.new_rows.is_empty());
    assert!(app.state.grid_state.result_set.is_none());
    assert!(app.grid_workspaces.states[&old_workspace].has_changes());
    assert!(Arc::ptr_eq(
        app.grid_workspaces.states[&old_workspace]
            .result_set
            .as_ref()
            .unwrap(),
        &old_result
    ));
}

#[test]
fn sidebar_query_other_table_with_draft_keeps_old_tab_recoverable() {
    let mut app = DbManagerApp::new_for_test();
    add_connections(&mut app, None);
    let original = result_with_ids(&[11]);
    bind_old_table_result(&mut app, &original);
    app.state.grid_state.new_rows.push(vec!["unsaved".into()]);
    let old_tab_id = app.session.tab_manager.get_active().unwrap().id.clone();
    let ctx = egui::Context::default();
    app.handle_sidebar_actions(
        &ctx,
        crate::ui::SidebarActions {
            query_table: Some("orders".into()),
            ..Default::default()
        },
    );
    assert_eq!(app.state.selected_table.as_deref(), Some("users"));
    assert!(Arc::ptr_eq(
        app.state.grid_state.result_set.as_ref().unwrap(),
        &original
    ));
    assert_eq!(app.state.grid_state.new_rows, [vec!["unsaved"]]);

    app.open_new_query_tab();
    app.handle_sidebar_actions(
        &ctx,
        crate::ui::SidebarActions {
            query_table: Some("orders".into()),
            ..Default::default()
        },
    );
    assert_eq!(
        app.session
            .tab_manager
            .get_active()
            .unwrap()
            .selected_table
            .as_deref(),
        Some("orders")
    );
    app.activate_query_tab(0);
    assert_eq!(app.session.tab_manager.get_active().unwrap().id, old_tab_id);
    assert_eq!(app.state.selected_table.as_deref(), Some("users"));
    assert_eq!(app.state.grid_state.new_rows, [vec!["unsaved"]]);
    assert!(Arc::ptr_eq(
        app.state.grid_state.result_set.as_ref().unwrap(),
        &original
    ));
}

#[test]
fn sidebar_other_connection_selection_cannot_relabel_original_grid_draft() {
    let mut app = DbManagerApp::new_for_test();
    add_connections(&mut app, None);
    let original = result_with_ids(&[11]);
    bind_old_table_result(&mut app, &original);
    app.state
        .grid_state
        .new_rows
        .push(vec!["old connection draft".into()]);
    let previous_connection = app.session.manager.active.clone();
    app.session.manager.active = Some("second".into());
    let mut actions = crate::ui::SidebarActions {
        query_table: Some("users".into()),
        ..Default::default()
    };
    app.reconcile_sidebar_selection(previous_connection, Some("users".into()), &mut actions);
    assert!(actions.query_table.is_none());
    assert_eq!(app.session.manager.active.as_deref(), Some("first"));
    assert_eq!(app.state.selected_table.as_deref(), Some("users"));
    assert_eq!(
        app.state.grid_state.new_rows,
        [vec!["old connection draft"]]
    );
    assert!(Arc::ptr_eq(
        app.state.grid_state.result_set.as_ref().unwrap(),
        &original
    ));
    assert_eq!(
        app.session
            .tab_manager
            .get_active()
            .unwrap()
            .selected_table
            .as_deref(),
        Some("users")
    );
}

#[test]
fn sidebar_database_selection_under_other_connection_keeps_original_draft() {
    let mut app = DbManagerApp::new_for_test();
    add_connections(&mut app, Some("db1"));
    app.switch_grid_workspace(Some("users".into()));
    app.state
        .grid_state
        .new_rows
        .push(vec!["old db draft".into()]);
    let previous_connection = app.session.manager.active.clone();
    app.session.manager.active = Some("second".into());
    let mut actions = crate::ui::SidebarActions {
        select_database: Some("db2".into()),
        ..Default::default()
    };
    app.reconcile_sidebar_selection(previous_connection, None, &mut actions);
    assert_eq!(app.session.manager.active.as_deref(), Some("second"));
    assert!(app.state.selected_table.is_none());
    assert!(app.state.grid_state.new_rows.is_empty());
    app.session.manager.active = Some("first".into());
    app.switch_grid_workspace(Some("users".into()));
    assert_eq!(app.state.grid_state.new_rows, [vec!["old db draft"]]);
}

fn record_pending_save(app: &mut DbManagerApp) -> (TaskId, TableViewId) {
    let workspace_id = app.active_grid_workspace_id().expect("table workspace");
    let connection = app.session.manager.get_active().expect("connection");
    let connection_id = connection.id;
    let table_view = TableViewId::from_components(
        connection_id,
        &connection.config.database,
        &workspace_id.tab_id,
        &workspace_id.table_name,
    );
    let (task_id, _) = app
        .session
        .task_registry
        .register(OperationKey::GridSave { table_view }, TaskKind::GridSave);
    app.pending_grid_saves.insert(
        task_id,
        GridSubmittedEdits::capture(workspace_id, connection_id, &app.state.grid_state),
    );
    app.state.grid_state.save_in_flight = true;
    (task_id, table_view)
}

fn deliver_committed_save(app: &mut DbManagerApp, (task_id, table_view): (TaskId, TableViewId)) {
    let mut report = ImportExecutionReport::new(1);
    report.succeeded = 1;
    app.handle_runtime_event(
        &egui::Context::default(),
        RuntimeEvent {
            task_id,
            key: OperationKey::GridSave { table_view },
            outcome: RuntimeOutcome::GridSaved {
                table_view,
                table: "users".into(),
                result: Ok(report),
                elapsed_ms: 1,
            },
        },
    );
}

#[test]
fn table_recreated_before_old_save_reply_keeps_new_draft_locked() {
    let mut app = DbManagerApp::new_for_test();
    app.state.show_er_diagram = false;
    add_connections(&mut app, None);
    app.switch_grid_workspace(Some("users".into()));
    app.state
        .grid_state
        .new_rows
        .push(vec!["same insert".into()]);
    let old_task = record_pending_save(&mut app);
    let connection_id = app.session.manager.get_active().unwrap().id;

    app.handle_table_dropped(
        &egui::Context::default(),
        connection_id,
        "first".into(),
        None,
        "users".into(),
        Ok(()),
    );
    app.switch_grid_workspace(Some("users".into()));
    app.state
        .grid_state
        .new_rows
        .push(vec!["same insert".into()]);
    let new_task = record_pending_save(&mut app);
    deliver_committed_save(&mut app, old_task);

    assert_eq!(app.state.grid_state.new_rows, [vec!["same insert"]]);
    assert!(app.state.grid_state.save_in_flight);
    assert!(!app.state.grid_state.needs_refresh_after_save);
    assert!(app.pending_grid_saves.contains_key(&new_task.0));
    assert!(
        app.session
            .notifications
            .latest_message()
            .unwrap()
            .contains("已保存")
    );
}

#[test]
fn database_recreated_before_old_save_reply_keeps_new_draft_locked() {
    let mut app = DbManagerApp::new_for_test();
    add_connections(&mut app, Some("db1"));
    app.switch_grid_workspace(Some("users".into()));
    app.state
        .grid_state
        .new_rows
        .push(vec!["same insert".into()]);
    let old_task = record_pending_save(&mut app);
    let connection_id = app.session.manager.get_active().unwrap().id;

    app.handle_database_dropped(
        &egui::Context::default(),
        connection_id,
        "first".into(),
        "db1".into(),
        Ok(()),
    );
    app.session
        .manager
        .connections
        .get_mut("first")
        .unwrap()
        .set_database("db1".into(), vec!["users".into()]);
    app.switch_grid_workspace(Some("users".into()));
    app.state
        .grid_state
        .new_rows
        .push(vec!["same insert".into()]);
    let new_task = record_pending_save(&mut app);
    deliver_committed_save(&mut app, old_task);

    assert_eq!(app.state.grid_state.new_rows, [vec!["same insert"]]);
    assert!(app.state.grid_state.save_in_flight);
    assert!(!app.state.grid_state.needs_refresh_after_save);
    assert!(app.pending_grid_saves.contains_key(&new_task.0));
}

#[test]
fn active_tables_old_database_reply_cannot_replace_new_database_tables() {
    let mut app = DbManagerApp::new_for_test();
    app.session
        .manager
        .add(ConnectionConfig::new("pg", DatabaseType::PostgreSQL));
    app.session.manager.active = Some("pg".into());
    app.session
        .manager
        .connections
        .get_mut("pg")
        .unwrap()
        .set_database("old_db".into(), vec!["old_table".into()]);
    let connection_id = app.session.manager.get_active().unwrap().id;
    let key = OperationKey::ActiveTables {
        connection: connection_id,
    };
    let (old_task, _) = app
        .session
        .task_registry
        .register(key.clone(), TaskKind::ActiveTables);
    app.session
        .manager
        .connections
        .get_mut("pg")
        .unwrap()
        .set_database("new_db".into(), vec!["new_table".into()]);
    app.session
        .autocomplete
        .set_tables(vec!["new_table".into()]);

    app.handle_runtime_event(
        &egui::Context::default(),
        RuntimeEvent {
            task_id: old_task,
            key: key.clone(),
            outcome: RuntimeOutcome::ActiveTablesReloaded {
                connection: connection_id,
                conn_name: "pg".into(),
                database: "old_db".into(),
                result: Ok(vec!["stale_table".into()]),
            },
        },
    );
    assert_eq!(
        app.session.manager.get_active().unwrap().tables,
        ["new_table"]
    );
    let completions = app.session.autocomplete.get_completions("new", 3);
    assert!(completions.iter().any(|item| item.label == "new_table"));

    let (new_task, _) = app
        .session
        .task_registry
        .register(key.clone(), TaskKind::ActiveTables);
    app.handle_runtime_event(
        &egui::Context::default(),
        RuntimeEvent {
            task_id: new_task,
            key,
            outcome: RuntimeOutcome::ActiveTablesReloaded {
                connection: connection_id,
                conn_name: "pg".into(),
                database: "new_db".into(),
                result: Ok(vec!["fresh_table".into()]),
            },
        },
    );
    assert_eq!(
        app.session.manager.get_active().unwrap().tables,
        ["fresh_table"]
    );
}
