//! 边缘回归测试：覆盖高风险输入与状态机边界。

use gridix::core::{AutoComplete, CompletionKind, SessionState, TabState};
use gridix::database::DatabaseType;
use gridix::ui::{
    WelcomeOnboardingStatus, WelcomeOnboardingStep, WelcomeServiceState, WelcomeStatusSummary,
};

#[test]
fn autocomplete_is_case_insensitive_for_table_prefix() {
    let mut ac = AutoComplete::new();
    ac.set_tables(vec!["Users".to_string(), "orders".to_string()]);

    let completions = ac.get_completions("select * from us", 16);
    assert!(completions.iter().any(|c| c.label == "Users"));
}

#[test]
fn autocomplete_handles_unicode_cursor_without_panic() {
    let mut ac = AutoComplete::new();
    ac.set_tables(vec!["用户".to_string()]);

    // 这里的 cursor_pos 是字符索引，不是字节索引。
    let completions = ac.get_completions("SELECT * FROM 用户 WH", 18);
    assert!(!completions.is_empty());
}

#[test]
fn autocomplete_handles_cursor_out_of_bounds() {
    let ac = AutoComplete::new();
    let completions = ac.get_completions("SEL", usize::MAX);
    assert!(completions.iter().any(|c| c.label == "SELECT"));
}

#[test]
fn autocomplete_keyword_dedup_is_effective() {
    let ac = AutoComplete::new();
    let completions = ac.get_completions("NULL", 4);

    let keyword_nullif_count = completions
        .iter()
        .filter(|c| c.kind == CompletionKind::Keyword && c.label == "NULLIF")
        .count();
    assert_eq!(keyword_nullif_count, 1);
}

#[test]
fn autocomplete_function_completion_has_expected_insert_text() {
    let ac = AutoComplete::new();
    let completions = ac.get_completions("ROU", 3);
    let round = completions
        .iter()
        .find(|c| c.kind == CompletionKind::Function && c.label == "ROUND()")
        .expect("should include ROUND() completion");
    assert_eq!(round.insert_text, "ROUND(");
}

#[test]
fn autocomplete_column_context_prefers_columns_after_where() {
    let mut ac = AutoComplete::new();
    ac.set_columns(
        "users".to_string(),
        vec!["id".to_string(), "email".to_string()],
    );
    let completions = ac.get_completions("SELECT * FROM users WHERE i", 27);

    assert!(
        completions
            .iter()
            .any(|c| c.kind == CompletionKind::Column && c.label == "id")
    );
}

#[test]
fn autocomplete_result_count_is_limited() {
    let mut ac = AutoComplete::new();
    let many_tables = (0..64).map(|i| format!("table_{i}")).collect();
    ac.set_tables(many_tables);

    let completions = ac.get_completions("", 0);
    assert!(completions.len() <= 15);
}

#[test]
fn session_remove_tab_with_invalid_index_keeps_state() {
    let mut session = SessionState::new();
    session.add_tab(TabState::new("A", "SELECT 1"));
    session.add_tab(TabState::new("B", "SELECT 2"));

    session.remove_tab(99);
    assert_eq!(session.tab_count(), 2);
    assert_eq!(session.active_tab_index, 1);
}

#[test]
fn session_set_active_tab_out_of_range_is_ignored() {
    let mut session = SessionState::new();
    session.add_tab(TabState::new("A", "SELECT 1"));
    session.add_tab(TabState::new("B", "SELECT 2"));
    session.set_active_tab(0);

    session.set_active_tab(999);
    assert_eq!(session.active_tab_index, 0);
}

#[test]
fn session_remove_last_tab_results_in_empty_session() {
    let mut session = SessionState::new();
    session.add_tab(TabState::new("Only", "SELECT 1"));

    session.remove_tab(0);
    assert_eq!(session.tab_count(), 0);
    assert_eq!(session.active_tab_index, 0);
}

#[test]
fn welcome_onboarding_sqlite_path_skips_create_user_step() {
    let status = WelcomeOnboardingStatus {
        require_user_step: false,
        ..Default::default()
    };

    assert_eq!(status.total_steps(), 4);
    assert!(!status.steps().contains(&WelcomeOnboardingStep::CreateUser));
}

#[test]
fn welcome_onboarding_server_path_includes_create_user_step() {
    let status = WelcomeOnboardingStatus {
        require_user_step: true,
        ..Default::default()
    };

    assert_eq!(status.total_steps(), 5);
    assert!(status.steps().contains(&WelcomeOnboardingStep::CreateUser));
}

#[test]
fn welcome_onboarding_next_step_follows_expected_order() {
    let mut status = WelcomeOnboardingStatus {
        require_user_step: true,
        ..Default::default()
    };

    assert_eq!(
        status.next_step(),
        Some(WelcomeOnboardingStep::EnvironmentCheck)
    );
    status.environment_checked = true;
    assert_eq!(
        status.next_step(),
        Some(WelcomeOnboardingStep::CreateConnection)
    );
    status.connection_created = true;
    assert_eq!(
        status.next_step(),
        Some(WelcomeOnboardingStep::InitializeDatabase)
    );
    status.database_initialized = true;
    assert_eq!(status.next_step(), Some(WelcomeOnboardingStep::CreateUser));
    status.user_created = true;
    assert_eq!(
        status.next_step(),
        Some(WelcomeOnboardingStep::RunFirstQuery)
    );
    status.first_query_executed = true;
    assert!(status.next_step().is_none());
    assert!(status.is_complete());
}

#[test]
fn welcome_status_summary_maps_database_type_correctly() {
    let summary = WelcomeStatusSummary {
        sqlite: WelcomeServiceState::BuiltIn,
        postgres: WelcomeServiceState::InstalledNotRunning,
        mysql: WelcomeServiceState::Running,
    };

    assert_eq!(
        summary.state_for(DatabaseType::SQLite),
        WelcomeServiceState::BuiltIn
    );
    assert_eq!(
        summary.state_for(DatabaseType::PostgreSQL),
        WelcomeServiceState::InstalledNotRunning
    );
    assert_eq!(
        summary.state_for(DatabaseType::MySQL),
        WelcomeServiceState::Running
    );
}
