//! 面板组件

mod history_panel;
mod sidebar;

pub use history_panel::{HistoryPanel, HistoryPanelState};
pub use sidebar::{Sidebar, SidebarActions, SidebarFocusTransfer, SidebarPanelState};
