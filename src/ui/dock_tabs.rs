//! egui_dock integration — resizable panel layout for the main workspace.
//!
//! NOTE: Circular dependency — ui/dock_tabs.rs ↔ app/mod.rs.
//! WorkspaceViewer (TabViewer impl) references DbManagerApp, while
//! DbManagerApp owns DockState<DockTab>. This is contained to Layer 4
//! (both modules are in the same layer) and does not cause compilation
//! issues. A cleaner design would move WorkspaceViewer to app/ and keep
//! DockTab + sync_all in ui/.

use crate::app::DbManagerApp;
use crate::core::RightInspectorTab;
use crate::state::{WorkbenchPlacement, WorkbenchSurfaceKind};
use egui_dock::tab_viewer::OnCloseResponse;
use egui_dock::{DockState, NodeIndex, SurfaceIndex, TabViewer};

#[derive(Clone, Debug, PartialEq)]
pub enum DockTab {
    Surface {
        kind: WorkbenchSurfaceKind,
        title: String,
    },
    SqlDocument {
        index: usize,
        title: String,
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

#[derive(Clone, Debug, PartialEq)]
pub enum AuxPanelKind {
    Help,
    Keybindings,
    History,
}

impl DockTab {
    pub fn surface(kind: WorkbenchSurfaceKind) -> Self {
        let title = kind.descriptor().title;
        Self::Surface { kind, title }
    }

    pub fn surface_with_title(kind: WorkbenchSurfaceKind, title: impl Into<String>) -> Self {
        Self::Surface {
            kind,
            title: title.into(),
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
    }])
}

pub fn default_surface_layout(
    active_query_tab_id: impl Into<String>,
    inspector_mode: RightInspectorTab,
) -> DockState<DockTab> {
    let mut state = DockState::new(vec![DockTab::surface_with_title(
        WorkbenchSurfaceKind::SqlDocument { index: 0 },
        "查询 1",
    )]);
    let query_tab_id = active_query_tab_id.into();
    let tree = state.main_surface_mut();
    let [center, _left] = tree.split_left(
        NodeIndex::root(),
        0.78,
        vec![DockTab::surface(WorkbenchSurfaceKind::Explorer)],
    );
    let [center, _right] = tree.split_right(
        center,
        0.72,
        vec![DockTab::surface(WorkbenchSurfaceKind::Inspector {
            mode: inspector_mode,
        })],
    );
    let _ = tree.split_below(
        center,
        0.68,
        vec![DockTab::surface(WorkbenchSurfaceKind::QueryResult {
            query_tab_id,
        })],
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
    match placement {
        WorkbenchPlacement::Center => {
            tree.push_to_focused_leaf(tab);
        }
        WorkbenchPlacement::Left => {
            let _ = tree.split_left(NodeIndex::root(), 0.78, vec![tab]);
        }
        WorkbenchPlacement::Right => {
            let _ = tree.split_right(NodeIndex::root(), 0.78, vec![tab]);
        }
        WorkbenchPlacement::Bottom => {
            let _ = tree.split_below(NodeIndex::root(), 0.68, vec![tab]);
        }
    }
    true
}

pub fn has_surface_tab(state: &DockState<DockTab>, kind: &WorkbenchSurfaceKind) -> bool {
    let target = kind.surface_id();
    state
        .get_surface(SurfaceIndex::main())
        .is_some_and(|surface| {
            surface
                .iter_all_tabs()
                .any(|(_, tab)| tab.surface_kind().surface_id() == target)
        })
}

// ── 同步：每帧渲染前调用 ──────────────────────────────────────────────

/// 同步 dock 布局与 app 状态（SQL documents、ER 图）
pub fn sync_all(state: &mut DockState<DockTab>, app: &DbManagerApp) {
    sync_sql_documents(state, app.tab_manager());
    sync_er_visibility(state, app.state.show_er_diagram);
}

fn sync_sql_documents(state: &mut DockState<DockTab>, tab_manager: &crate::ui::QueryTabManager) {
    let mgr_count = tab_manager.tabs.len();
    let use_surface_documents = uses_surface_documents(state);

    // 1. 移除 dock 中索引越界的 SQL document tab
    remove_tabs(state, |t| {
        matches!(t, DockTab::SqlDocument { index, .. } if *index >= mgr_count)
            || matches!(
                t,
                DockTab::Surface {
                    kind: WorkbenchSurfaceKind::SqlDocument { index },
                    ..
                } if *index >= mgr_count
            )
    });

    // 2. 修复剩余 tabs 的索引，使其与 tab_manager 顺序一致
    let tree = state.main_surface_mut();
    let mut next_index = 0usize;
    for node in tree.iter_mut() {
        if let Some(tabs) = node.tabs_mut() {
            for tab in tabs.iter_mut() {
                if let DockTab::SqlDocument { index, title } = tab {
                    if *index < mgr_count {
                        *title = tab_manager.tabs[*index].title.clone();
                    }
                    *index = next_index;
                    next_index += 1;
                } else if let DockTab::Surface {
                    kind: WorkbenchSurfaceKind::SqlDocument { index },
                    title,
                } = tab
                {
                    if *index < mgr_count {
                        *title = tab_manager.tabs[*index].title.clone();
                    }
                    *index = next_index;
                    next_index += 1;
                }
            }
        }
    }

    // 3. 添加缺失的 tabs — 找到已有的 SQL document leaf 并追加
    if next_index < mgr_count {
        // 找到第一个 SQL document leaf 节点
        let mut target: Option<NodeIndex> = None;
        for (i, node) in tree.iter().enumerate() {
            if node.is_leaf() && node.tabs().unwrap_or(&[]).iter().any(is_sql_document_tab) {
                target = Some(NodeIndex(i));
                break;
            }
        }
        // 如果找到了，追加到该 leaf；否则用 split_right 创建新的
        for i in next_index..mgr_count {
            let title = tab_manager.tabs[i].title.clone();
            let tab = if use_surface_documents {
                DockTab::surface_with_title(WorkbenchSurfaceKind::SqlDocument { index: i }, title)
            } else {
                DockTab::SqlDocument { index: i, title }
            };
            if let Some(ref node_idx) = target {
                tree.set_focused_node(*node_idx);
                tree.push_to_focused_leaf(tab);
            } else {
                let _ = tree.split_right(NodeIndex::root(), 0.3, vec![tab]);
            }
        }
    }
}

fn sync_er_visibility(state: &mut DockState<DockTab>, show: bool) {
    let use_surface_tabs = uses_surface_tabs(state);
    let has_er = {
        if let Some(surface) = state.get_surface(SurfaceIndex::main()) {
            surface.iter_all_tabs().any(|(_, t)| is_er_diagram_tab(t))
        } else {
            false
        }
    };

    match (show, has_er) {
        (true, false) => {
            let tab = if use_surface_tabs {
                DockTab::surface(WorkbenchSurfaceKind::ErDiagram)
            } else {
                DockTab::ErDiagram
            };
            let _ = state
                .main_surface_mut()
                .split_right(NodeIndex::root(), 0.3, vec![tab]);
        }
        (false, true) => remove_tabs(state, is_er_diagram_tab),
        _ => {}
    }
}

fn remove_tabs<F: FnMut(&DockTab) -> bool>(state: &mut DockState<DockTab>, mut predicate: F) {
    state.retain_tabs(|tab| !predicate(tab));
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
        .get_surface(SurfaceIndex::main())
        .is_some_and(|surface| {
            surface
                .iter_all_tabs()
                .any(|(_, tab)| matches!(tab, DockTab::Surface { .. }))
        })
}

fn uses_surface_documents(state: &DockState<DockTab>) -> bool {
    state
        .get_surface(SurfaceIndex::main())
        .is_some_and(|surface| {
            surface.iter_all_tabs().any(|(_, tab)| {
                matches!(
                    tab,
                    DockTab::Surface {
                        kind: WorkbenchSurfaceKind::SqlDocument { .. },
                        ..
                    }
                )
            })
        })
}

// ── TabViewer ─────────────────────────────────────────────────────────

pub struct WorkspaceViewer<'a> {
    pub app: &'a mut DbManagerApp,
}

impl TabViewer for WorkspaceViewer<'_> {
    type Tab = DockTab;

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
        self.app
            .render_workbench_surface_in_ui(ui, tab.surface_kind());
    }

    fn on_close(&mut self, tab: &mut Self::Tab) -> OnCloseResponse {
        match tab {
            DockTab::Surface {
                kind: WorkbenchSurfaceKind::SqlDocument { index },
                ..
            } => {
                if self.app.tab_manager().tabs.len() <= 1 {
                    return OnCloseResponse::Ignore;
                }
                self.app.on_dock_tab_close(*index);
                OnCloseResponse::Close
            }
            DockTab::Surface {
                kind: WorkbenchSurfaceKind::ErDiagram,
                ..
            } => {
                self.app.toggle_er_diagram_visibility();
                OnCloseResponse::Close
            }
            DockTab::Surface { .. } => OnCloseResponse::Close,
            DockTab::SqlDocument { index, .. } => {
                // 至少保留一个 SQL document
                if self.app.tab_manager().tabs.len() <= 1 {
                    return OnCloseResponse::Ignore;
                }
                // 清理：持久化状态、取消查询、移除工作区
                self.app.on_dock_tab_close(*index);
                OnCloseResponse::Close
            }
            DockTab::ErDiagram => {
                self.app.toggle_er_diagram_visibility();
                OnCloseResponse::Close
            }
            DockTab::TableData { .. }
            | DockTab::SchemaObject { .. }
            | DockTab::Welcome
            | DockTab::AuxPanel { .. } => OnCloseResponse::Close,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{WorkbenchPlacement, WorkbenchSurfaceRole};
    use crate::ui::QueryTabManager;

    fn all_tabs(state: &DockState<DockTab>) -> Vec<DockTab> {
        state
            .get_surface(SurfaceIndex::main())
            .expect("main surface should exist")
            .iter_all_tabs()
            .map(|(_, tab)| tab.clone())
            .collect()
    }

    #[test]
    fn default_layout_uses_sql_document() {
        let state = default_layout();

        assert_eq!(
            all_tabs(&state),
            vec![DockTab::SqlDocument {
                index: 0,
                title: "查询 1".to_string(),
            }]
        );
    }

    #[test]
    fn default_surface_layout_seeds_peer_workbench_surfaces() {
        let state = default_surface_layout("tab-a", RightInspectorTab::Schema);
        let surfaces: Vec<_> = all_tabs(&state)
            .into_iter()
            .map(|tab| tab.surface_kind())
            .collect();

        assert!(surfaces.contains(&WorkbenchSurfaceKind::Explorer));
        assert!(surfaces.contains(&WorkbenchSurfaceKind::SqlDocument { index: 0 }));
        assert!(surfaces.contains(&WorkbenchSurfaceKind::QueryResult {
            query_tab_id: "tab-a".to_string()
        }));
        assert!(surfaces.contains(&WorkbenchSurfaceKind::Inspector {
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
                DockTab::SqlDocument { index, title } => Some((index, title)),
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
    fn sync_sql_documents_removes_out_of_range_tabs() {
        let mut state = DockState::new(vec![
            DockTab::SqlDocument {
                index: 0,
                title: "keep".to_string(),
            },
            DockTab::SqlDocument {
                index: 99,
                title: "stale".to_string(),
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
            }
        );
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
    fn ensure_surface_tab_adds_surface_once_by_stable_identity() {
        let mut state = default_layout();
        let result_surface = WorkbenchSurfaceKind::QueryResult {
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
        assert_eq!(sql.default_placement, WorkbenchPlacement::Center);
        assert_eq!(sql.persistence_key, "sql:2");
        assert_eq!(explorer_like_aux.role, WorkbenchSurfaceRole::Utility);
        assert_eq!(explorer_like_aux.persistence_key, "history");
    }
}
