//! egui_dock integration — resizable panel layout for the main workspace.
//!
//! NOTE: Circular dependency — ui/dock_tabs.rs ↔ app/mod.rs.
//! WorkspaceViewer (TabViewer impl) references DbManagerApp, while
//! DbManagerApp owns `DockState<DockTab>`. This is contained to Layer 4
//! (both modules are in the same layer) and does not cause compilation
//! issues. A cleaner design would move WorkspaceViewer to app/ and keep
//! DockTab + refresh_dock_from_session in ui/.

use crate::app::DbManagerApp;
use crate::core::RightInspectorTab;
use crate::state::{WorkbenchPlacement, WorkbenchSurfaceKind};
use egui_dock::tab_viewer::OnCloseResponse;
use egui_dock::{DockState, NodeIndex, TabViewer};

/// Canonical April-shell screenshot proportions for the dock workspace.
///
/// The fixed PrimarySidebar keeps its 280px shell width outside this dock tree.
/// These ratios only control movable dock surfaces: dominant center editor/data,
/// compact right inspector, and a lower SQL/output region.
pub const DEFAULT_LEFT_RETAIN_RATIO: f32 = 0.79;
pub const DEFAULT_RIGHT_RETAIN_RATIO: f32 = 0.73;
pub const DEFAULT_BOTTOM_RETAIN_RATIO: f32 = 0.69;

#[derive(Clone, Debug, PartialEq)]
pub enum DockTab {
    Surface {
        kind: WorkbenchSurfaceKind,
        title: String,
        document_id: Option<String>,
    },
    SqlDocument {
        index: usize,
        title: String,
        document_id: Option<String>,
    },
    TableData {
        title: String,
    },
    ErDiagram,
    SchemaObject {
        title: String,
    },
    Welcome,
    AuxPanel {
        kind: AuxPanelKind,
        title: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum AuxPanelKind {
    Help,
    Keybindings,
    History,
}

impl DockTab {
    pub fn surface(kind: WorkbenchSurfaceKind) -> Self {
        let title = kind.descriptor().title;
        Self::Surface {
            kind,
            title,
            document_id: None,
        }
    }

    pub fn surface_with_title(kind: WorkbenchSurfaceKind, title: impl Into<String>) -> Self {
        Self::Surface {
            kind,
            title: title.into(),
            document_id: None,
        }
    }

    pub fn surface_kind(&self) -> WorkbenchSurfaceKind {
        match self {
            Self::Surface { kind, .. } => kind.clone(),
            Self::SqlDocument { index, .. } => WorkbenchSurfaceKind::SqlDocument { index: *index },
            Self::TableData { title } => WorkbenchSurfaceKind::TableData {
                connection: None,
                database: None,
                table: title.clone(),
            },
            Self::ErDiagram => WorkbenchSurfaceKind::ErDiagram,
            Self::SchemaObject { title } => WorkbenchSurfaceKind::SchemaObject {
                title: title.clone(),
            },
            Self::Welcome => WorkbenchSurfaceKind::Welcome,
            Self::AuxPanel { kind, .. } => match kind {
                AuxPanelKind::Help => WorkbenchSurfaceKind::Help,
                AuxPanelKind::Keybindings => WorkbenchSurfaceKind::Settings,
                AuxPanelKind::History => WorkbenchSurfaceKind::History,
            },
        }
    }
}

pub fn default_layout() -> DockState<DockTab> {
    DockState::new(vec![DockTab::SqlDocument {
        index: 0,
        title: "查询 1".into(),
        document_id: None,
    }])
}

pub fn default_surface_layout(
    active_query_tab_id: impl Into<String>,
    _inspector_mode: RightInspectorTab,
) -> DockState<DockTab> {
    let query_tab_id = active_query_tab_id.into();
    let mut state = DockState::new(vec![DockTab::surface(
        WorkbenchSurfaceKind::SurfaceResult {
            query_tab_id: query_tab_id.clone(),
        },
    )]);
    let tree = state.main_surface_mut();
    let [center, _right] = tree.split_right(
        NodeIndex::root(),
        DEFAULT_RIGHT_RETAIN_RATIO,
        vec![DockTab::surface(WorkbenchSurfaceKind::ErDiagram)],
    );
    let _ = tree.split_below(
        center,
        DEFAULT_BOTTOM_RETAIN_RATIO,
        vec![DockTab::Surface {
            kind: WorkbenchSurfaceKind::SqlDocument { index: 0 },
            title: "查询 1".into(),
            document_id: Some(query_tab_id.clone()),
        }],
    );
    state
}

pub fn ensure_surface_tab(state: &mut DockState<DockTab>, kind: WorkbenchSurfaceKind) -> bool {
    if has_surface_tab(state, &kind) {
        return false;
    }

    let placement = kind.descriptor().default_placement;
    let tab = DockTab::surface(kind);
    let tree = state.main_surface_mut();
    if matches!(placement, WorkbenchPlacement::Center) {
        tree.push_to_focused_leaf(tab);
    } else {
        split_root_or_append(tree, placement, tab);
    }

    true
}

pub fn has_surface_tab(state: &DockState<DockTab>, kind: &WorkbenchSurfaceKind) -> bool {
    let target = kind.surface_id();
    state
        .iter_all_tabs()
        .any(|(_, tab)| tab.surface_kind().surface_id() == target)
}

// ── 同步：每帧渲染前调用 ──────────────────────────────────────────────

/// 从 session 刷新 dock 布局（SQL documents、ER 图可见性）。
/// 只读操作：dock 读取 session/state，不写回。
pub fn refresh_dock_from_session(state: &mut DockState<DockTab>, app: &DbManagerApp) {
    sync_sql_documents(state, app.tab_manager());
    sync_er_visibility(state, app.state.show_er_diagram);
}

pub(crate) fn sync_sql_documents(
    state: &mut DockState<DockTab>,
    tab_manager: &crate::ui::QueryTabManager,
) {
    let use_surface_documents = uses_surface_documents(state);
    remove_tabs(state, |tab| {
        sql_document_reference(tab).is_some_and(|(index, document_id)| {
            document_id.map_or_else(
                || index >= tab_manager.tabs.len(),
                |id| !tab_manager.tabs.iter().any(|query| query.id == id),
            )
        })
    });

    let mut present = std::collections::HashSet::new();
    for (_, node) in state.iter_all_nodes_mut() {
        if let Some(tabs) = node.tabs_mut() {
            for tab in tabs.iter_mut() {
                if let Some(id) = bind_sql_document(tab, tab_manager) {
                    present.insert(id);
                }
            }
        }
    }

    let tree = state.main_surface_mut();
    let target = tree.iter().enumerate().find_map(|(i, node)| {
        (node.is_leaf() && node.tabs().unwrap_or(&[]).iter().any(is_sql_document_tab))
            .then_some(NodeIndex(i))
    });
    for (index, query) in tab_manager.tabs.iter().enumerate() {
        if present.contains(query.id.as_str()) {
            continue;
        }
        let tab = if use_surface_documents {
            DockTab::Surface {
                kind: WorkbenchSurfaceKind::SqlDocument { index },
                title: query.title.clone(),
                document_id: Some(query.id.clone()),
            }
        } else {
            DockTab::SqlDocument {
                index,
                title: query.title.clone(),
                document_id: Some(query.id.clone()),
            }
        };
        if let Some(node) = target {
            tree.set_focused_node(node);
            tree.push_to_focused_leaf(tab);
        } else {
            split_root_or_append(tree, WorkbenchPlacement::Right, tab);
        }
    }
}

fn bind_sql_document<'a>(
    tab: &mut DockTab,
    tab_manager: &'a crate::ui::QueryTabManager,
) -> Option<&'a str> {
    let (index, title, document_id) = sql_document_binding(tab)?;
    if document_id.is_none() {
        *document_id = tab_manager.tabs.get(*index).map(|query| query.id.clone());
    }
    let (position, query) = tab_manager
        .tabs
        .iter()
        .enumerate()
        .find(|(_, query)| Some(&query.id) == document_id.as_ref())?;
    *index = position;
    if *title != query.title {
        title.clone_from(&query.title);
    }
    Some(&query.id)
}

fn sql_document_reference(tab: &DockTab) -> Option<(usize, Option<&str>)> {
    match tab {
        DockTab::SqlDocument {
            index, document_id, ..
        }
        | DockTab::Surface {
            kind: WorkbenchSurfaceKind::SqlDocument { index },
            document_id,
            ..
        } => Some((*index, document_id.as_deref())),
        _ => None,
    }
}

fn sql_document_binding(
    tab: &mut DockTab,
) -> Option<(&mut usize, &mut String, &mut Option<String>)> {
    match tab {
        DockTab::SqlDocument {
            index,
            title,
            document_id,
        }
        | DockTab::Surface {
            kind: WorkbenchSurfaceKind::SqlDocument { index },
            title,
            document_id,
        } => Some((index, title, document_id)),
        _ => None,
    }
}

pub fn activate_sql_document(state: &mut DockState<DockTab>, document_id: &str) {
    if let Some(path) = state.find_tab_from(|tab| {
        sql_document_reference(tab).is_some_and(|(_, id)| id == Some(document_id))
    }) {
        let _ = state.set_active_tab(path);
        state.set_focused_node_and_surface(path.node_path());
    }
}

fn resolve_sql_document_index(
    tab: &DockTab,
    tab_manager: &crate::ui::QueryTabManager,
) -> Option<usize> {
    let (index, document_id) = sql_document_reference(tab)?;
    match document_id {
        Some(id) => tab_manager.tabs.iter().position(|query| query.id == id),
        None => (index < tab_manager.tabs.len()).then_some(index),
    }
}

fn sync_er_visibility(state: &mut DockState<DockTab>, show: bool) {
    let use_surface_tabs = uses_surface_tabs(state);
    let has_er = state.iter_all_tabs().any(|(_, tab)| is_er_diagram_tab(tab));

    match (show, has_er) {
        (true, false) => {
            let tab = if use_surface_tabs {
                DockTab::surface(WorkbenchSurfaceKind::ErDiagram)
            } else {
                DockTab::ErDiagram
            };
            split_root_or_append(state.main_surface_mut(), WorkbenchPlacement::Right, tab);
        }
        (false, true) => remove_tabs(state, is_er_diagram_tab),
        _ => {}
    }
}

fn remove_tabs<F: FnMut(&DockTab) -> bool>(state: &mut DockState<DockTab>, mut predicate: F) {
    state.retain_tabs(|tab| !predicate(tab));
}

fn split_root_or_append(
    tree: &mut egui_dock::Tree<DockTab>,
    placement: WorkbenchPlacement,
    tab: DockTab,
) {
    if tree.root_node().is_none_or(|node| node.is_empty()) {
        tree.push_to_focused_leaf(tab);
        return;
    }

    match placement {
        WorkbenchPlacement::Center => tree.push_to_focused_leaf(tab),
        WorkbenchPlacement::Left => {
            let _ = tree.split_left(NodeIndex::root(), DEFAULT_LEFT_RETAIN_RATIO, vec![tab]);
        }
        WorkbenchPlacement::Right => {
            let _ = tree.split_right(NodeIndex::root(), DEFAULT_RIGHT_RETAIN_RATIO, vec![tab]);
        }
        WorkbenchPlacement::Bottom => {
            let _ = tree.split_below(NodeIndex::root(), DEFAULT_BOTTOM_RETAIN_RATIO, vec![tab]);
        }
    }
}

fn is_sql_document_tab(tab: &DockTab) -> bool {
    matches!(tab, DockTab::SqlDocument { .. })
        || matches!(
            tab,
            DockTab::Surface {
                kind: WorkbenchSurfaceKind::SqlDocument { .. },
                ..
            }
        )
}

fn is_er_diagram_tab(tab: &DockTab) -> bool {
    matches!(tab, DockTab::ErDiagram)
        || matches!(
            tab,
            DockTab::Surface {
                kind: WorkbenchSurfaceKind::ErDiagram,
                ..
            }
        )
}

fn uses_surface_tabs(state: &DockState<DockTab>) -> bool {
    state
        .iter_all_tabs()
        .any(|(_, tab)| matches!(tab, DockTab::Surface { .. }))
}

fn uses_surface_documents(state: &DockState<DockTab>) -> bool {
    state.iter_all_tabs().any(|(_, tab)| {
        matches!(
            tab,
            DockTab::Surface {
                kind: WorkbenchSurfaceKind::SqlDocument { .. },
                ..
            }
        )
    })
}

// ── TabViewer ─────────────────────────────────────────────────────────

fn sql_document_dock_id(tab_manager: &crate::ui::QueryTabManager, index: usize) -> egui::Id {
    match tab_manager.tabs.get(index) {
        Some(tab) => egui::Id::new(("dock-sql", tab.id.as_str())),
        None => egui::Id::new(("dock-sql-index", index)),
    }
}

pub struct WorkspaceViewer<'a> {
    pub app: &'a mut DbManagerApp,
}

impl TabViewer for WorkspaceViewer<'_> {
    type Tab = DockTab;

    fn id(&mut self, tab: &mut Self::Tab) -> egui::Id {
        match tab {
            DockTab::Surface {
                kind: WorkbenchSurfaceKind::SqlDocument { index },
                document_id,
                ..
            }
            | DockTab::SqlDocument {
                index, document_id, ..
            } => document_id.as_deref().map_or_else(
                || sql_document_dock_id(self.app.tab_manager(), *index),
                |id| egui::Id::new(("dock-sql", id)),
            ),
            DockTab::Surface { kind, .. } => egui::Id::new(("dock-surface", kind)),
            DockTab::TableData { title } => egui::Id::new(("dock-table", title)),
            DockTab::ErDiagram => egui::Id::new("dock-er"),
            DockTab::SchemaObject { title } => egui::Id::new(("dock-schema", title)),
            DockTab::Welcome => egui::Id::new("dock-welcome"),
            DockTab::AuxPanel { kind, .. } => egui::Id::new(("dock-aux", kind)),
        }
    }

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        match tab {
            DockTab::Surface { title, .. } => title.as_str().into(),
            DockTab::SqlDocument { title, .. } => title.as_str().into(),
            DockTab::TableData { title } => title.as_str().into(),
            DockTab::ErDiagram => "ER 图".into(),
            DockTab::SchemaObject { title } => title.as_str().into(),
            DockTab::Welcome => "欢迎".into(),
            DockTab::AuxPanel { title, .. } => title.as_str().into(),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        let kind = if is_sql_document_tab(tab) {
            let Some(index) = resolve_sql_document_index(tab, self.app.tab_manager()) else {
                return;
            };
            WorkbenchSurfaceKind::SqlDocument { index }
        } else {
            tab.surface_kind()
        };
        self.app.render_workbench_surface_in_ui(ui, kind);
    }

    fn on_tab_button(&mut self, tab: &mut Self::Tab, response: &egui::Response) {
        if response.clicked()
            && let Some(index) = resolve_sql_document_index(tab, self.app.tab_manager())
        {
            self.app.activate_query_tab(index);
        }
    }

    fn on_close(&mut self, tab: &mut Self::Tab) -> OnCloseResponse {
        if is_sql_document_tab(tab) {
            if self.app.tab_manager().tabs.len() <= 1 {
                return OnCloseResponse::Ignore;
            }
            if let Some(index) = resolve_sql_document_index(tab, self.app.tab_manager()) {
                self.app.on_dock_tab_close(index);
            }
            return OnCloseResponse::Close;
        }
        match tab {
            DockTab::Surface {
                kind: WorkbenchSurfaceKind::ErDiagram,
                ..
            } => {
                self.app.toggle_er_diagram_visibility();
                OnCloseResponse::Close
            }
            DockTab::ErDiagram => {
                self.app.toggle_er_diagram_visibility();
                OnCloseResponse::Close
            }
            _ => OnCloseResponse::Close,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{WorkbenchPlacement, WorkbenchSurfaceRole};
    use crate::ui::QueryTabManager;

    fn all_tabs(state: &DockState<DockTab>) -> Vec<DockTab> {
        state.iter_all_tabs().map(|(_, tab)| tab.clone()).collect()
    }
    #[test]
    fn default_layout_uses_sql_document() {
        let state = default_layout();

        assert_eq!(
            all_tabs(&state),
            vec![DockTab::SqlDocument {
                index: 0,
                title: "查询 1".to_string(),
                document_id: None,
            }]
        );
    }

    #[test]
    fn default_surface_ratios_match_april_shell_screenshot_baseline() {
        assert_eq!(DEFAULT_LEFT_RETAIN_RATIO, 0.79);
        assert_eq!(DEFAULT_RIGHT_RETAIN_RATIO, 0.73);
        assert_eq!(DEFAULT_BOTTOM_RETAIN_RATIO, 0.69);
    }

    #[test]
    fn default_surface_layout_seeds_results_editor_and_er_surfaces() {
        let state = default_surface_layout("tab-a", RightInspectorTab::Schema);
        let surfaces: Vec<_> = all_tabs(&state)
            .into_iter()
            .map(|tab| tab.surface_kind())
            .collect();

        assert!(!surfaces.contains(&WorkbenchSurfaceKind::Explorer));
        assert!(surfaces.contains(&WorkbenchSurfaceKind::SqlDocument { index: 0 }));
        assert!(surfaces.contains(&WorkbenchSurfaceKind::SurfaceResult {
            query_tab_id: "tab-a".to_string()
        }));
        assert!(surfaces.contains(&WorkbenchSurfaceKind::ErDiagram));
        assert!(!surfaces.contains(&WorkbenchSurfaceKind::Inspector {
            mode: RightInspectorTab::Schema
        }));
    }

    #[test]
    fn sync_sql_documents_adds_missing_tabs_and_updates_titles() {
        let mut state = default_layout();
        let mut manager = QueryTabManager::new();
        manager.tabs[0].title = "first".to_string();
        manager.new_tab();
        manager.tabs[1].title = "second".to_string();

        sync_sql_documents(&mut state, &manager);

        let docs: Vec<_> = all_tabs(&state)
            .into_iter()
            .filter_map(|tab| match tab {
                DockTab::SqlDocument { index, title, .. } => Some((index, title)),
                _ => None,
            })
            .collect();
        assert_eq!(
            docs,
            vec![(0, "first".to_string()), (1, "second".to_string())]
        );
    }

    #[test]
    fn sync_sql_documents_preserves_surface_tabs_when_adding_missing_docs() {
        let mut state = default_surface_layout("tab-a", RightInspectorTab::Properties);
        let mut manager = QueryTabManager::new();
        manager.tabs[0].id = "tab-a".to_string();
        manager.tabs[0].title = "first".to_string();
        manager.new_tab();
        manager.tabs[1].title = "second".to_string();

        sync_sql_documents(&mut state, &manager);

        let docs: Vec<_> = all_tabs(&state)
            .into_iter()
            .filter_map(|tab| match tab {
                DockTab::Surface {
                    kind: WorkbenchSurfaceKind::SqlDocument { index },
                    title,
                    ..
                } => Some((index, title)),
                _ => None,
            })
            .collect();
        assert_eq!(
            docs,
            vec![(0, "first".to_string()), (1, "second".to_string())]
        );
    }

    #[test]
    fn sql_document_dock_id_survives_preceding_tab_close() {
        let mut manager = QueryTabManager::new();
        manager.new_tab();
        manager.tabs[0].title = "same title".to_string();
        manager.tabs[1].title = "same title".to_string();

        let first_id = sql_document_dock_id(&manager, 0);
        let second_id = sql_document_dock_id(&manager, 1);
        assert_ne!(first_id, second_id);

        manager.close_tab(0);

        assert_eq!(sql_document_dock_id(&manager, 0), second_id);
    }

    #[test]
    fn sync_sql_documents_removes_out_of_range_tabs() {
        let mut state = DockState::new(vec![
            DockTab::SqlDocument {
                index: 0,
                title: "keep".to_string(),
                document_id: None,
            },
            DockTab::SqlDocument {
                index: 99,
                title: "stale".to_string(),
                document_id: None,
            },
        ]);
        let manager = QueryTabManager::new();

        sync_sql_documents(&mut state, &manager);

        let docs: Vec<_> = all_tabs(&state)
            .into_iter()
            .filter(|tab| matches!(tab, DockTab::SqlDocument { .. }))
            .collect();
        assert_eq!(docs.len(), 1);
        assert_eq!(
            docs[0],
            DockTab::SqlDocument {
                index: 0,
                title: "查询 1".to_string(),
                document_id: Some(manager.tabs[0].id.clone()),
            }
        );
    }

    #[test]
    fn dock_reorder_keeps_sql_identity_through_sync_and_close() {
        let mut manager = QueryTabManager::new();
        manager.tabs[0].sql = "SELECT 'A'".into();
        manager.tabs[0].title = "A".into();
        manager.new_tab();
        manager.tabs[1].sql = "SELECT 'B'".into();
        manager.tabs[1].title = "B".into();
        let first_id = manager.tabs[0].id.clone();
        let second_id = manager.tabs[1].id.clone();
        let mut dock = DockState::new(vec![
            DockTab::Surface {
                kind: WorkbenchSurfaceKind::SqlDocument { index: 1 },
                title: "B".into(),
                document_id: Some(second_id.clone()),
            },
            DockTab::Surface {
                kind: WorkbenchSurfaceKind::SqlDocument { index: 0 },
                title: "A".into(),
                document_id: Some(first_id),
            },
        ]);

        sync_sql_documents(&mut dock, &manager);
        sync_sql_documents(&mut dock, &manager);
        let docs: Vec<_> = all_tabs(&dock)
            .iter()
            .filter_map(sql_document_reference)
            .map(|(index, id)| (index, id.map(str::to_string)))
            .collect();
        assert_eq!(
            docs,
            vec![
                (1, Some(second_id.clone())),
                (0, Some(manager.tabs[0].id.clone()))
            ]
        );
        assert_eq!(manager.tabs[docs[0].0].sql, "SELECT 'B'");
        assert_eq!(manager.tabs[docs[1].0].sql, "SELECT 'A'");

        manager.close_tab(0);
        sync_sql_documents(&mut dock, &manager);
        let docs: Vec<_> = all_tabs(&dock)
            .iter()
            .filter_map(sql_document_reference)
            .map(|(index, id)| (index, id.map(str::to_string)))
            .collect();
        assert_eq!(docs, vec![(0, Some(second_id))]);
        assert_eq!(manager.tabs[docs[0].0].sql, "SELECT 'B'");
    }

    fn click_dock_document(viewer: &mut WorkspaceViewer<'_>, tab: &mut DockTab) {
        let ctx = egui::Context::default();
        let mut position = egui::Pos2::ZERO;
        for _ in 0..2 {
            ctx.begin_pass(egui::RawInput::default());
            egui::Area::new("dock_click_test".into()).show(&ctx, |ui| {
                position = ui.button("B").rect.center();
            });
            ctx.end_pass().textures_delta.clear();
        }
        ctx.begin_pass(egui::RawInput {
            events: vec![
                egui::Event::PointerMoved(position),
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            ..Default::default()
        });
        egui::Area::new("dock_click_test".into()).show(&ctx, |ui| {
            let response = ui.button("B");
            assert!(response.clicked());
            viewer.on_tab_button(tab, &response);
        });
        ctx.end_pass().textures_delta.clear();
    }

    #[test]
    fn detached_document_keeps_identity_content_and_close_target_after_preceding_close() {
        let mut app = DbManagerApp::new_for_test();
        for (index, title) in ["A", "B", "C"].iter().enumerate() {
            if index > 0 {
                app.session.tab_manager.new_tab();
            }
            app.session.tab_manager.tabs[index].sql = format!("SELECT '{title}'");
            app.session.tab_manager.tabs[index].title = (*title).into();
        }
        let [a_id, b_id, c_id] =
            std::array::from_fn(|index| app.session.tab_manager.tabs[index].id.clone());
        let mut dock = default_surface_layout(a_id.clone(), RightInspectorTab::Properties);
        sync_sql_documents(&mut dock, app.tab_manager());
        let b_path = dock
            .find_tab_from(|tab| {
                sql_document_reference(tab).is_some_and(|(_, id)| id == Some(b_id.as_str()))
            })
            .expect("B docked");
        let floating = dock.detach_tab(
            b_path,
            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(640.0, 480.0)),
        );
        sync_sql_documents(&mut dock, app.tab_manager());
        sync_sql_documents(&mut dock, app.tab_manager());
        assert_eq!(
            dock.iter_all_tabs()
                .filter(|(_, tab)| {
                    sql_document_reference(tab).is_some_and(|(_, id)| id == Some(b_id.as_str()))
                })
                .count(),
            1
        );
        let mut stale_b_tab = dock
            .iter_all_tabs()
            .find_map(|(_, tab)| {
                sql_document_reference(tab)
                    .is_some_and(|(_, id)| id == Some(b_id.as_str()))
                    .then(|| tab.clone())
            })
            .expect("B before A closes");

        let mut a_tab = dock
            .find_tab_from(|tab| {
                sql_document_reference(tab).is_some_and(|(_, id)| id == Some(a_id.as_str()))
            })
            .and_then(|path| dock.remove_tab(path))
            .expect("A docked");
        assert_eq!(
            WorkspaceViewer { app: &mut app }.on_close(&mut a_tab),
            OnCloseResponse::Close
        );
        sync_sql_documents(&mut dock, app.tab_manager());
        let b_path = dock
            .find_tab_from(|tab| {
                sql_document_reference(tab).is_some_and(|(_, id)| id == Some(b_id.as_str()))
            })
            .expect("B stays detached");
        assert_eq!(b_path.surface, floating);
        let b_tab = dock.remove_tab(b_path).expect("B tab");
        assert_eq!(
            b_tab,
            DockTab::Surface {
                kind: WorkbenchSurfaceKind::SqlDocument { index: 0 },
                title: "B".into(),
                document_id: Some(b_id.clone()),
            }
        );
        assert_eq!(
            sql_document_reference(&stale_b_tab),
            Some((1, Some(b_id.as_str())))
        );
        assert_eq!(
            resolve_sql_document_index(&stale_b_tab, app.tab_manager()),
            Some(0)
        );
        assert_eq!(app.tab_manager().tabs[0].sql, "SELECT 'B'");

        click_dock_document(&mut WorkspaceViewer { app: &mut app }, &mut stale_b_tab);
        assert_eq!(
            app.tab_manager().get_active().map(|tab| tab.id.as_str()),
            Some(b_id.as_str())
        );
        assert_eq!(
            WorkspaceViewer { app: &mut app }.on_close(&mut stale_b_tab),
            OnCloseResponse::Close
        );
        assert_eq!(app.tab_manager().tabs[0].id, c_id);
        assert_eq!(app.tab_manager().tabs.len(), 1);
    }

    #[test]
    fn sync_er_visibility_uses_surface_tab_when_surface_tree_is_active() {
        let mut state = default_surface_layout("tab-a", RightInspectorTab::Properties);

        sync_er_visibility(&mut state, true);

        assert!(all_tabs(&state).iter().any(|tab| matches!(
            tab,
            DockTab::Surface {
                kind: WorkbenchSurfaceKind::ErDiagram,
                ..
            }
        )));

        sync_er_visibility(&mut state, false);

        assert!(
            all_tabs(&state)
                .iter()
                .all(|tab| !matches!(tab.surface_kind(), WorkbenchSurfaceKind::ErDiagram))
        );
    }

    #[test]
    fn ensure_surface_tab_on_empty_tree_creates_a_leaf() {
        let mut state = default_layout();
        remove_tabs(&mut state, |_| true);

        assert!(ensure_surface_tab(
            &mut state,
            WorkbenchSurfaceKind::Explorer
        ));
        assert!(
            all_tabs(&state)
                .iter()
                .any(|tab| tab.surface_kind() == WorkbenchSurfaceKind::Explorer)
        );
    }

    #[test]
    fn ensure_surface_tab_adds_surface_once_by_stable_identity() {
        let mut state = default_layout();
        let result_surface = WorkbenchSurfaceKind::SurfaceResult {
            query_tab_id: "tab-a".to_string(),
        };

        assert!(ensure_surface_tab(&mut state, result_surface.clone()));
        assert!(!ensure_surface_tab(&mut state, result_surface.clone()));

        let matches: Vec<_> = all_tabs(&state)
            .into_iter()
            .filter(|tab| tab.surface_kind() == result_surface)
            .collect();
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn ensure_surface_tab_can_add_explorer_and_inspector_to_legacy_layout() {
        let mut state = default_layout();
        let inspector = WorkbenchSurfaceKind::Inspector {
            mode: RightInspectorTab::Properties,
        };

        assert!(ensure_surface_tab(
            &mut state,
            WorkbenchSurfaceKind::Explorer
        ));
        assert!(ensure_surface_tab(&mut state, inspector.clone()));

        let surfaces: Vec<_> = all_tabs(&state)
            .into_iter()
            .map(|tab| tab.surface_kind())
            .collect();
        assert!(surfaces.contains(&WorkbenchSurfaceKind::Explorer));
        assert!(surfaces.contains(&inspector));
        assert!(surfaces.contains(&WorkbenchSurfaceKind::SqlDocument { index: 0 }));
    }

    #[test]
    fn dock_tabs_bridge_to_workbench_surface_kinds() {
        let sql = DockTab::SqlDocument {
            index: 2,
            title: "Query".to_string(),
            document_id: None,
        }
        .surface_kind()
        .descriptor();
        let explorer_like_aux = DockTab::AuxPanel {
            kind: AuxPanelKind::History,
            title: "History".to_string(),
        }
        .surface_kind()
        .descriptor();

        assert_eq!(sql.role, WorkbenchSurfaceRole::Document);
        assert_eq!(sql.default_placement, WorkbenchPlacement::Bottom);
        assert_eq!(sql.persistence_key, "sql:2");
        assert_eq!(explorer_like_aux.role, WorkbenchSurfaceRole::Utility);
        assert_eq!(explorer_like_aux.persistence_key, "history");
    }
}
