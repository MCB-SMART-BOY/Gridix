//! Editor-style workbench shell components.

mod activity_bar;
mod bottom_panel;
mod right_inspector;
mod shell;
mod status_bar;
mod surface;

pub use activity_bar::{WorkbenchActivityBar, WorkbenchActivityBarResponse};
pub use bottom_panel::{
    WorkbenchBottomPanel, WorkbenchBottomPanelResponse, bottom_panel_tab_label, bottom_panel_tabs,
};
pub use right_inspector::{
    WorkbenchRightInspector, WorkbenchRightInspectorResponse, right_inspector_tab_label,
    right_inspector_tabs,
};
pub use shell::WorkbenchShell;
pub use status_bar::WorkbenchStatusBarContent;
pub use surface::{
    SurfaceAction, WorkbenchSurfaceHeader, WorkbenchSurfaceHeaderResponse, surface_icon_button,
    surface_icon_glyph, surface_tooltip,
};
