//! Workbench runtime state.
//!
//! Persisted preferences live in `core::WorkbenchConfig`; this module stores
//! per-frame UI state used by the future editor-style shell.

use crate::core::{BottomPanelTab, RightInspectorTab, WorkbenchActivity, WorkbenchConfig};
use crate::ui::FocusArea;

const ALL_PLACEMENTS: &[WorkbenchPlacement] = &[
    WorkbenchPlacement::Center,
    WorkbenchPlacement::Left,
    WorkbenchPlacement::Right,
    WorkbenchPlacement::Bottom,
];
const DOCUMENT_PLACEMENTS: &[WorkbenchPlacement] = &[WorkbenchPlacement::Center];
const SIDE_OR_CENTER_PLACEMENTS: &[WorkbenchPlacement] = &[
    WorkbenchPlacement::Left,
    WorkbenchPlacement::Right,
    WorkbenchPlacement::Bottom,
    WorkbenchPlacement::Center,
];
const OUTPUT_PLACEMENTS: &[WorkbenchPlacement] = &[
    WorkbenchPlacement::Bottom,
    WorkbenchPlacement::Center,
    WorkbenchPlacement::Right,
    WorkbenchPlacement::Left,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkbenchSurfaceRole {
    Document,
    Data,
    Navigation,
    Output,
    Inspector,
    Utility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkbenchPlacement {
    Center,
    Left,
    Right,
    Bottom,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorkbenchSurfaceId(String);

impl WorkbenchSurfaceId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WorkbenchSurfaceKind {
    SqlDocument {
        index: usize,
    },
    QueryResult {
        query_tab_id: String,
    },
    Explain {
        query_tab_id: String,
    },
    TableData {
        connection: Option<String>,
        database: Option<String>,
        table: String,
    },
    ErDiagram,
    SchemaObject {
        title: String,
    },
    Explorer,
    Filters,
    Objects,
    History,
    Messages,
    Tasks,
    Inspector {
        mode: RightInspectorTab,
    },
    Settings,
    Help,
    Welcome,
}

#[derive(Debug, Clone)]
pub struct WorkbenchSurfaceDescriptor {
    pub kind: WorkbenchSurfaceKind,
    pub role: WorkbenchSurfaceRole,
    pub title: String,
    pub icon: &'static str,
    pub description: &'static str,
    pub command_id: Option<&'static str>,
    pub singleton: bool,
    pub default_placement: WorkbenchPlacement,
    pub allowed_placements: &'static [WorkbenchPlacement],
    pub persistence_key: String,
}

impl WorkbenchSurfaceDescriptor {
    pub fn tooltip(&self) -> String {
        let command = self
            .command_id
            .map(|id| format!("\nCommand: {}", id))
            .unwrap_or_default();
        format!("{}\n{}{}", self.title, self.description, command)
    }
}

impl WorkbenchSurfaceKind {
    pub fn from_activity(activity: WorkbenchActivity) -> Self {
        match activity {
            WorkbenchActivity::Explorer => Self::Explorer,
            WorkbenchActivity::Filters => Self::Filters,
            WorkbenchActivity::Objects => Self::Objects,
            WorkbenchActivity::History => Self::History,
            WorkbenchActivity::Help => Self::Help,
            WorkbenchActivity::Settings => Self::Settings,
        }
    }

    pub fn from_bottom_panel_tab(tab: BottomPanelTab, query_tab_id: impl Into<String>) -> Self {
        let query_tab_id = query_tab_id.into();
        match tab {
            BottomPanelTab::Results => Self::QueryResult { query_tab_id },
            BottomPanelTab::Messages => Self::Messages,
            BottomPanelTab::Explain => Self::Explain { query_tab_id },
            BottomPanelTab::History => Self::History,
            BottomPanelTab::Tasks => Self::Tasks,
        }
    }

    pub fn from_right_inspector_tab(tab: RightInspectorTab) -> Self {
        Self::Inspector { mode: tab }
    }

    pub fn descriptor(&self) -> WorkbenchSurfaceDescriptor {
        let metadata = self.metadata();
        WorkbenchSurfaceDescriptor {
            kind: self.clone(),
            role: metadata.role,
            title: self.title(metadata.title),
            icon: metadata.icon,
            description: metadata.description,
            command_id: metadata.command_id,
            singleton: metadata.singleton,
            default_placement: metadata.default_placement,
            allowed_placements: metadata.allowed_placements,
            persistence_key: self.persistence_key(),
        }
    }

    pub fn surface_id(&self) -> WorkbenchSurfaceId {
        WorkbenchSurfaceId::new(self.persistence_key())
    }

    fn title(&self, fallback: &'static str) -> String {
        match self {
            Self::SqlDocument { index } => format!("SQL {}", index + 1),
            Self::QueryResult { query_tab_id } => format!("Result {}", query_tab_id),
            Self::Explain { query_tab_id } => format!("Explain {}", query_tab_id),
            Self::TableData { table, .. } => table.clone(),
            Self::SchemaObject { title } => title.clone(),
            Self::Inspector { mode } => format!("Inspector {}", right_inspector_mode_key(*mode)),
            _ => fallback.to_string(),
        }
    }

    fn persistence_key(&self) -> String {
        match self {
            Self::SqlDocument { index } => format!("sql:{}", index),
            Self::QueryResult { query_tab_id } => format!("result:{}", query_tab_id),
            Self::Explain { query_tab_id } => format!("explain:{}", query_tab_id),
            Self::TableData {
                connection,
                database,
                table,
            } => format!(
                "table:{}:{}:{}",
                connection.as_deref().unwrap_or("_"),
                database.as_deref().unwrap_or("_"),
                table
            ),
            Self::ErDiagram => "er".to_string(),
            Self::SchemaObject { title } => format!("schema:{}", title),
            Self::Explorer => "explorer".to_string(),
            Self::Filters => "filters".to_string(),
            Self::Objects => "objects".to_string(),
            Self::History => "history".to_string(),
            Self::Messages => "messages".to_string(),
            Self::Tasks => "tasks".to_string(),
            Self::Inspector { mode } => format!("inspector:{}", right_inspector_mode_key(*mode)),
            Self::Settings => "settings".to_string(),
            Self::Help => "help".to_string(),
            Self::Welcome => "welcome".to_string(),
        }
    }

    fn metadata(&self) -> SurfaceMetadata {
        match self {
            Self::SqlDocument { .. } => SurfaceMetadata {
                title: "SQL",
                icon: "sql",
                description: "编辑和执行当前 SQL 文档",
                command_id: Some("workbench.surface.sql"),
                role: WorkbenchSurfaceRole::Document,
                singleton: false,
                default_placement: WorkbenchPlacement::Center,
                allowed_placements: DOCUMENT_PLACEMENTS,
            },
            Self::QueryResult { .. } => SurfaceMetadata {
                title: "Results",
                icon: "table",
                description: "显示绑定 SQL 文档的查询结果",
                command_id: Some("workbench.surface.results"),
                role: WorkbenchSurfaceRole::Output,
                singleton: false,
                default_placement: WorkbenchPlacement::Bottom,
                allowed_placements: OUTPUT_PLACEMENTS,
            },
            Self::Explain { .. } => SurfaceMetadata {
                title: "Explain",
                icon: "explain",
                description: "显示当前 SQL 文档的结构化执行计划",
                command_id: Some("workbench.surface.explain"),
                role: WorkbenchSurfaceRole::Output,
                singleton: false,
                default_placement: WorkbenchPlacement::Bottom,
                allowed_placements: OUTPUT_PLACEMENTS,
            },
            Self::TableData { .. } => SurfaceMetadata {
                title: "Table",
                icon: "grid",
                description: "浏览和编辑表数据",
                command_id: Some("workbench.surface.tableData"),
                role: WorkbenchSurfaceRole::Data,
                singleton: false,
                default_placement: WorkbenchPlacement::Center,
                allowed_placements: ALL_PLACEMENTS,
            },
            Self::ErDiagram => SurfaceMetadata {
                title: "ER",
                icon: "graph",
                description: "查看数据库实体关系图",
                command_id: Some("workbench.surface.er"),
                role: WorkbenchSurfaceRole::Document,
                singleton: true,
                default_placement: WorkbenchPlacement::Center,
                allowed_placements: ALL_PLACEMENTS,
            },
            Self::SchemaObject { .. } => SurfaceMetadata {
                title: "Schema",
                icon: "schema",
                description: "查看数据库对象定义",
                command_id: Some("workbench.surface.schemaObject"),
                role: WorkbenchSurfaceRole::Document,
                singleton: false,
                default_placement: WorkbenchPlacement::Center,
                allowed_placements: ALL_PLACEMENTS,
            },
            Self::Explorer => SurfaceMetadata {
                title: "Explorer",
                icon: "database-tree",
                description: "显示连接、数据库和表",
                command_id: Some("workbench.surface.explorer"),
                role: WorkbenchSurfaceRole::Navigation,
                singleton: true,
                default_placement: WorkbenchPlacement::Left,
                allowed_placements: SIDE_OR_CENTER_PLACEMENTS,
            },
            Self::Filters => SurfaceMetadata {
                title: "Filters",
                icon: "filter",
                description: "编辑当前结果集的筛选条件",
                command_id: Some("workbench.surface.filters"),
                role: WorkbenchSurfaceRole::Navigation,
                singleton: true,
                default_placement: WorkbenchPlacement::Left,
                allowed_placements: SIDE_OR_CENTER_PLACEMENTS,
            },
            Self::Objects => SurfaceMetadata {
                title: "Objects",
                icon: "objects",
                description: "浏览触发器、存储过程和数据库对象",
                command_id: Some("workbench.surface.objects"),
                role: WorkbenchSurfaceRole::Navigation,
                singleton: true,
                default_placement: WorkbenchPlacement::Left,
                allowed_placements: SIDE_OR_CENTER_PLACEMENTS,
            },
            Self::History => SurfaceMetadata {
                title: "History",
                icon: "history",
                description: "查看和复用查询历史",
                command_id: Some("workbench.surface.history"),
                role: WorkbenchSurfaceRole::Utility,
                singleton: true,
                default_placement: WorkbenchPlacement::Bottom,
                allowed_placements: ALL_PLACEMENTS,
            },
            Self::Messages => SurfaceMetadata {
                title: "Messages",
                icon: "log",
                description: "显示查询消息、错误和日志",
                command_id: Some("workbench.surface.messages"),
                role: WorkbenchSurfaceRole::Output,
                singleton: true,
                default_placement: WorkbenchPlacement::Bottom,
                allowed_placements: OUTPUT_PLACEMENTS,
            },
            Self::Tasks => SurfaceMetadata {
                title: "Tasks",
                icon: "tasks",
                description: "显示后台任务和进度",
                command_id: Some("workbench.surface.tasks"),
                role: WorkbenchSurfaceRole::Output,
                singleton: true,
                default_placement: WorkbenchPlacement::Bottom,
                allowed_placements: OUTPUT_PLACEMENTS,
            },
            Self::Inspector { .. } => SurfaceMetadata {
                title: "Inspector",
                icon: "info",
                description: "显示当前选择的属性、结构、行、单元格或连接详情",
                command_id: Some("workbench.surface.inspector"),
                role: WorkbenchSurfaceRole::Inspector,
                singleton: true,
                default_placement: WorkbenchPlacement::Right,
                allowed_placements: ALL_PLACEMENTS,
            },
            Self::Settings => SurfaceMetadata {
                title: "Settings",
                icon: "gear",
                description: "查看和编辑设置、主题与快捷键",
                command_id: Some("workbench.surface.settings"),
                role: WorkbenchSurfaceRole::Utility,
                singleton: true,
                default_placement: WorkbenchPlacement::Center,
                allowed_placements: ALL_PLACEMENTS,
            },
            Self::Help => SurfaceMetadata {
                title: "Help",
                icon: "help",
                description: "打开帮助、学习内容和使用指南",
                command_id: Some("workbench.surface.help"),
                role: WorkbenchSurfaceRole::Utility,
                singleton: true,
                default_placement: WorkbenchPlacement::Center,
                allowed_placements: ALL_PLACEMENTS,
            },
            Self::Welcome => SurfaceMetadata {
                title: "Welcome",
                icon: "welcome",
                description: "显示欢迎页和入门入口",
                command_id: Some("workbench.surface.welcome"),
                role: WorkbenchSurfaceRole::Utility,
                singleton: true,
                default_placement: WorkbenchPlacement::Center,
                allowed_placements: DOCUMENT_PLACEMENTS,
            },
        }
    }
}

struct SurfaceMetadata {
    title: &'static str,
    icon: &'static str,
    description: &'static str,
    command_id: Option<&'static str>,
    role: WorkbenchSurfaceRole,
    singleton: bool,
    default_placement: WorkbenchPlacement,
    allowed_placements: &'static [WorkbenchPlacement],
}

fn right_inspector_mode_key(mode: RightInspectorTab) -> &'static str {
    match mode {
        RightInspectorTab::Properties => "properties",
        RightInspectorTab::Schema => "schema",
        RightInspectorTab::Row => "row",
        RightInspectorTab::Cell => "cell",
        RightInspectorTab::ErSelection => "er",
        RightInspectorTab::Connection => "connection",
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum WorkbenchFocus {
    TopBar,
    ActivityBar,
    PrimarySidebar,
    #[default]
    EditorArea,
    BottomPanel,
    RightInspector,
    Surface(WorkbenchSurfaceId),
    Dialog,
}

impl WorkbenchFocus {
    pub fn from_focus_area(area: FocusArea) -> Self {
        match area {
            FocusArea::Toolbar => Self::TopBar,
            FocusArea::QueryTabs
            | FocusArea::DataGrid
            | FocusArea::ErDiagram
            | FocusArea::SqlEditor => Self::EditorArea,
            FocusArea::Sidebar => Self::PrimarySidebar,
            FocusArea::Dialog => Self::Dialog,
        }
    }

    pub fn for_surface(surface: &WorkbenchSurfaceKind) -> Self {
        Self::Surface(surface.surface_id())
    }

    pub fn fallback_focus_area(&self) -> FocusArea {
        match self {
            Self::TopBar => FocusArea::Toolbar,
            Self::ActivityBar | Self::PrimarySidebar => FocusArea::Sidebar,
            Self::EditorArea | Self::BottomPanel | Self::RightInspector | Self::Surface(_) => {
                FocusArea::DataGrid
            }
            Self::Dialog => FocusArea::Dialog,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TopBarState;

#[derive(Debug, Clone, Default)]
pub struct ActivityBarState;

#[derive(Debug, Clone)]
pub struct PrimarySidebarState {
    pub visible: bool,
    pub width: f32,
    pub is_resizing: bool,
}

impl Default for PrimarySidebarState {
    fn default() -> Self {
        Self {
            visible: true,
            width: 280.0,
            is_resizing: false,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct EditorAreaState;

#[derive(Debug, Clone)]
pub struct BottomPanelState {
    pub visible: bool,
    pub height: f32,
    pub active_tab: BottomPanelTab,
    pub is_resizing: bool,
}

impl Default for BottomPanelState {
    fn default() -> Self {
        Self {
            visible: true,
            height: 260.0,
            active_tab: BottomPanelTab::default(),
            is_resizing: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RightInspectorState {
    pub visible: bool,
    pub width: f32,
    pub active_tab: RightInspectorTab,
    pub schema_table: Option<String>,
    pub is_resizing: bool,
}

impl Default for RightInspectorState {
    fn default() -> Self {
        Self {
            visible: false,
            width: 320.0,
            active_tab: RightInspectorTab::default(),
            schema_table: None,
            is_resizing: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StatusBarState {
    pub visible: bool,
}

impl Default for StatusBarState {
    fn default() -> Self {
        Self { visible: true }
    }
}

#[derive(Debug, Clone)]
pub struct WorkbenchState {
    pub active_activity: WorkbenchActivity,
    pub focus: WorkbenchFocus,
    pub top_bar: TopBarState,
    pub activity_bar: ActivityBarState,
    pub primary_sidebar: PrimarySidebarState,
    pub editor_area: EditorAreaState,
    pub bottom_panel: BottomPanelState,
    pub right_inspector: RightInspectorState,
    pub status_bar: StatusBarState,
}

impl Default for WorkbenchState {
    fn default() -> Self {
        Self::from_config(&WorkbenchConfig::default())
    }
}

impl WorkbenchState {
    pub fn from_config(config: &WorkbenchConfig) -> Self {
        Self {
            active_activity: config.activity,
            focus: WorkbenchFocus::default(),
            top_bar: TopBarState,
            activity_bar: ActivityBarState,
            primary_sidebar: PrimarySidebarState {
                visible: config.sidebar.visible,
                width: config.sidebar.width,
                is_resizing: false,
            },
            editor_area: EditorAreaState,
            bottom_panel: BottomPanelState {
                visible: config.bottom_panel.visible,
                height: config.bottom_panel.height,
                active_tab: config.bottom_panel.active_tab,
                is_resizing: false,
            },
            right_inspector: RightInspectorState {
                visible: config.right_inspector.visible,
                width: config.right_inspector.width,
                active_tab: config.right_inspector.active_tab,
                schema_table: None,
                is_resizing: false,
            },
            status_bar: StatusBarState {
                visible: config.status_bar.visible,
            },
        }
    }

    pub fn set_focus_area(&mut self, area: FocusArea) {
        self.focus = WorkbenchFocus::from_focus_area(area);
    }

    pub fn set_focused_surface(&mut self, surface: &WorkbenchSurfaceKind) {
        self.focus = WorkbenchFocus::for_surface(surface);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{
        BottomPanelConfig, BottomPanelTab, PrimarySidebarConfig, RightInspectorConfig,
        RightInspectorTab, StatusBarConfig,
    };

    #[test]
    fn focus_bridge_maps_all_existing_focus_areas() {
        assert_eq!(
            WorkbenchFocus::from_focus_area(FocusArea::Toolbar),
            WorkbenchFocus::TopBar
        );
        assert_eq!(
            WorkbenchFocus::from_focus_area(FocusArea::QueryTabs),
            WorkbenchFocus::EditorArea
        );
        assert_eq!(
            WorkbenchFocus::from_focus_area(FocusArea::Sidebar),
            WorkbenchFocus::PrimarySidebar
        );
        assert_eq!(
            WorkbenchFocus::from_focus_area(FocusArea::DataGrid),
            WorkbenchFocus::EditorArea
        );
        assert_eq!(
            WorkbenchFocus::from_focus_area(FocusArea::ErDiagram),
            WorkbenchFocus::EditorArea
        );
        assert_eq!(
            WorkbenchFocus::from_focus_area(FocusArea::SqlEditor),
            WorkbenchFocus::EditorArea
        );
        assert_eq!(
            WorkbenchFocus::from_focus_area(FocusArea::Dialog),
            WorkbenchFocus::Dialog
        );
    }

    #[test]
    fn default_state_uses_valid_workbench_defaults() {
        let state = WorkbenchState::default();

        assert_eq!(state.active_activity, WorkbenchActivity::Explorer);
        assert_eq!(state.focus, WorkbenchFocus::EditorArea);
        assert!(state.primary_sidebar.visible);
        assert_eq!(state.primary_sidebar.width, 280.0);
        assert!(state.bottom_panel.visible);
        assert_eq!(state.bottom_panel.height, 260.0);
        assert_eq!(state.bottom_panel.active_tab, BottomPanelTab::Results);
        assert!(!state.right_inspector.visible);
        assert_eq!(state.right_inspector.width, 320.0);
        assert_eq!(
            state.right_inspector.active_tab,
            RightInspectorTab::Properties
        );
        assert!(state.status_bar.visible);
    }

    #[test]
    fn ui_state_default_includes_valid_workbench_state() {
        let state = crate::state::UiState::default();

        assert_eq!(state.workbench.active_activity, WorkbenchActivity::Explorer);
        assert_eq!(state.workbench.focus, WorkbenchFocus::EditorArea);
        assert!(state.workbench.primary_sidebar.visible);
        assert!(state.workbench.status_bar.visible);
    }

    #[test]
    fn state_can_be_seeded_from_config() {
        let config = WorkbenchConfig {
            activity: WorkbenchActivity::Filters,
            sidebar: PrimarySidebarConfig {
                visible: false,
                width: 333.0,
                ..Default::default()
            },
            bottom_panel: BottomPanelConfig {
                height: 222.0,
                ..Default::default()
            },
            right_inspector: RightInspectorConfig {
                visible: true,
                width: 444.0,
                ..Default::default()
            },
            status_bar: StatusBarConfig {
                visible: false,
                ..Default::default()
            },
            ..Default::default()
        };

        let state = WorkbenchState::from_config(&config);

        assert_eq!(state.active_activity, WorkbenchActivity::Filters);
        assert!(!state.primary_sidebar.visible);
        assert_eq!(state.primary_sidebar.width, 333.0);
        assert_eq!(state.bottom_panel.height, 222.0);
        assert!(state.right_inspector.visible);
        assert_eq!(state.right_inspector.width, 444.0);
        assert!(!state.status_bar.visible);
    }

    #[test]
    fn explorer_descriptor_is_navigation_surface_with_movable_placement() {
        let descriptor = WorkbenchSurfaceKind::Explorer.descriptor();

        assert_eq!(descriptor.role, WorkbenchSurfaceRole::Navigation);
        assert_eq!(descriptor.default_placement, WorkbenchPlacement::Left);
        assert!(
            descriptor
                .allowed_placements
                .contains(&WorkbenchPlacement::Right)
        );
        assert!(
            descriptor
                .allowed_placements
                .contains(&WorkbenchPlacement::Center)
        );
        assert!(descriptor.singleton);
        assert_eq!(descriptor.icon, "database-tree");
        assert_eq!(descriptor.command_id, Some("workbench.surface.explorer"));
    }

    #[test]
    fn result_and_table_surfaces_are_not_fixed_to_bottom_panel() {
        let result = WorkbenchSurfaceKind::QueryResult {
            query_tab_id: "q1".to_string(),
        }
        .descriptor();
        let explain = WorkbenchSurfaceKind::Explain {
            query_tab_id: "q1".to_string(),
        }
        .descriptor();
        let table = WorkbenchSurfaceKind::TableData {
            connection: Some("local".to_string()),
            database: Some("main".to_string()),
            table: "users".to_string(),
        }
        .descriptor();

        assert_eq!(result.role, WorkbenchSurfaceRole::Output);
        assert_eq!(result.default_placement, WorkbenchPlacement::Bottom);
        assert!(
            result
                .allowed_placements
                .contains(&WorkbenchPlacement::Center)
        );
        assert_eq!(explain.role, WorkbenchSurfaceRole::Output);
        assert_eq!(explain.default_placement, WorkbenchPlacement::Bottom);
        assert_eq!(explain.persistence_key, "explain:q1");
        assert_eq!(table.role, WorkbenchSurfaceRole::Data);
        assert_eq!(table.default_placement, WorkbenchPlacement::Center);
        assert!(
            table
                .allowed_placements
                .contains(&WorkbenchPlacement::Bottom)
        );
    }

    #[test]
    fn inspector_descriptor_defaults_right_but_allows_any_dock_placement() {
        let descriptor = WorkbenchSurfaceKind::Inspector {
            mode: RightInspectorTab::Cell,
        }
        .descriptor();

        assert_eq!(descriptor.role, WorkbenchSurfaceRole::Inspector);
        assert_eq!(descriptor.default_placement, WorkbenchPlacement::Right);
        assert!(
            descriptor
                .allowed_placements
                .contains(&WorkbenchPlacement::Left)
        );
        assert!(
            descriptor
                .allowed_placements
                .contains(&WorkbenchPlacement::Bottom)
        );
        assert_eq!(descriptor.persistence_key, "inspector:cell");
    }

    #[test]
    fn legacy_workbench_regions_map_to_surface_kinds() {
        assert_eq!(
            WorkbenchSurfaceKind::from_activity(WorkbenchActivity::Explorer),
            WorkbenchSurfaceKind::Explorer
        );
        assert_eq!(
            WorkbenchSurfaceKind::from_activity(WorkbenchActivity::Settings),
            WorkbenchSurfaceKind::Settings
        );
        assert_eq!(
            WorkbenchSurfaceKind::from_bottom_panel_tab(BottomPanelTab::Results, "tab-a"),
            WorkbenchSurfaceKind::QueryResult {
                query_tab_id: "tab-a".to_string()
            }
        );
        assert_eq!(
            WorkbenchSurfaceKind::from_bottom_panel_tab(BottomPanelTab::Explain, "tab-a"),
            WorkbenchSurfaceKind::Explain {
                query_tab_id: "tab-a".to_string()
            }
        );
        assert_eq!(
            WorkbenchSurfaceKind::from_right_inspector_tab(RightInspectorTab::Schema),
            WorkbenchSurfaceKind::Inspector {
                mode: RightInspectorTab::Schema
            }
        );
    }

    #[test]
    fn workbench_focus_can_target_surface_identity() {
        let mut state = WorkbenchState::default();
        let surface = WorkbenchSurfaceKind::TableData {
            connection: Some("local".to_string()),
            database: Some("main".to_string()),
            table: "users".to_string(),
        };

        state.set_focused_surface(&surface);

        assert_eq!(
            state.focus,
            WorkbenchFocus::Surface(WorkbenchSurfaceId::new("table:local:main:users"))
        );
        assert_eq!(state.focus.fallback_focus_area(), FocusArea::DataGrid);
    }

    #[test]
    fn surface_ids_are_stable_for_dynamic_identity() {
        let first = WorkbenchSurfaceKind::TableData {
            connection: Some("local".to_string()),
            database: Some("main".to_string()),
            table: "users".to_string(),
        }
        .surface_id();
        let second = WorkbenchSurfaceKind::TableData {
            connection: Some("local".to_string()),
            database: Some("main".to_string()),
            table: "orders".to_string(),
        }
        .surface_id();

        assert_eq!(first.as_str(), "table:local:main:users");
        assert_eq!(second.as_str(), "table:local:main:orders");
        assert_ne!(first, second);
    }

    #[test]
    fn descriptor_tooltip_satisfies_icon_only_metadata_contract() {
        let tooltip = WorkbenchSurfaceKind::Filters.descriptor().tooltip();

        assert!(tooltip.contains("Filters"));
        assert!(tooltip.contains("筛选"));
        assert!(tooltip.contains("Command: workbench.surface.filters"));
    }
}
