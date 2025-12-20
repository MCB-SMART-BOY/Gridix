//! UI 对话框测试

use gridix::ui::dialogs::{
    DialogResult, DialogSize, DialogButtons, DialogState,
    SimpleDialogState, DataDialogState,
    DialogStyle, FooterResult,
};

// ============================================================================
// Dialog Trait 测试
// ============================================================================

#[test]
fn test_dialog_result() {
    let result: DialogResult<i32> = DialogResult::Confirm(42);
    assert!(result.is_confirm());
    assert!(!result.is_cancel());
    assert!(!result.is_none());

    let result: DialogResult<i32> = DialogResult::Cancel;
    assert!(!result.is_confirm());
    assert!(result.is_cancel());

    let result: DialogResult<i32> = DialogResult::None;
    assert!(result.is_none());
}

#[test]
fn test_dialog_result_confirmed() {
    let result: DialogResult<i32> = DialogResult::Confirm(42);
    assert_eq!(result.confirmed(), Some(42));

    let result: DialogResult<i32> = DialogResult::Cancel;
    assert_eq!(result.confirmed(), None);
}

#[test]
fn test_dialog_result_map() {
    let result: DialogResult<i32> = DialogResult::Confirm(42);
    let mapped = result.map(|x| x * 2);
    assert_eq!(mapped.confirmed(), Some(84));

    let result: DialogResult<i32> = DialogResult::Cancel;
    let mapped = result.map(|x| x * 2);
    assert!(mapped.is_cancel());
}

#[test]
fn test_dialog_size() {
    assert_eq!(DialogSize::Small.width(), 320.0);
    assert_eq!(DialogSize::Medium.width(), 420.0);
    assert_eq!(DialogSize::Large.width(), 520.0);

    let custom = DialogSize::Custom { width: 600.0, max_height: 800.0 };
    assert_eq!(custom.width(), 600.0);
    assert_eq!(custom.max_height(), 800.0);
}

#[test]
fn test_dialog_buttons() {
    let default = DialogButtons::default();
    assert_eq!(default.confirm_label(), "确认 [Enter]");
    assert_eq!(default.cancel_label(), "取消 [Esc]");

    let danger = DialogButtons::danger("删除");
    assert!(danger.confirm_danger);
    assert_eq!(danger.confirm_label(), "删除 [y]");

    let close = DialogButtons::close_only();
    assert!(!close.show_cancel);
}

#[test]
fn test_simple_dialog_state() {
    let mut state = SimpleDialogState::default();
    assert!(!state.is_open());

    state.open();
    assert!(state.is_open());

    state.close();
    assert!(!state.is_open());
}

#[test]
fn test_data_dialog_state() {
    let mut state: DataDialogState<String> = DataDialogState::default();
    assert!(!state.is_open());

    state.open_with("test".to_string());
    assert!(state.is_open());
    assert_eq!(state.data(), "test");

    state.reset();
    assert!(!state.is_open());
    assert_eq!(state.data(), "");
}

// ============================================================================
// Dialog Common 测试
// ============================================================================

#[test]
fn test_dialog_style() {
    assert_eq!(DialogStyle::SMALL.width, 320.0);
    assert_eq!(DialogStyle::MEDIUM.width, 420.0);
    assert_eq!(DialogStyle::LARGE.width, 520.0);
}

#[test]
fn test_footer_result() {
    let none = FooterResult::NONE;
    assert!(!none.has_action());

    let confirmed = FooterResult::CONFIRMED;
    assert!(confirmed.has_action());
    assert!(confirmed.confirmed);

    let cancelled = FooterResult::CANCELLED;
    assert!(cancelled.has_action());
    assert!(cancelled.cancelled);
}
