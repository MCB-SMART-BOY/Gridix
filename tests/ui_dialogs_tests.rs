//! UI 对话框测试

use gridix::ui::dialogs::{DialogStyle, FooterResult};

// ============================================================================
// Dialog Common 测试
// ============================================================================

#[test]
fn test_dialog_style() {
    assert_eq!(DialogStyle::SMALL.min_width, 300.0);
    assert_eq!(DialogStyle::SMALL.default_width, 360.0);
    assert_eq!(DialogStyle::MEDIUM.default_width, 520.0);
    assert_eq!(DialogStyle::LARGE.default_width, 700.0);
    assert_eq!(DialogStyle::WORKSPACE.max_width, 1480.0);
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
