//! egui_dock integration — resizable panel layout for the main workspace.

use egui_dock::{DockState, NodeIndex, SurfaceIndex, TabViewer};
use egui_dock::tab_viewer::OnCloseResponse;
use crate::app::DbManagerApp;

#[derive(Clone, Debug, PartialEq)]
pub enum DockTab {
    QueryData { index: usize, title: String },
    SqlEditor,
    ErDiagram,
    AuxPanel { kind: AuxPanelKind, title: String },
}

#[derive(Clone, Debug, PartialEq)]
pub enum AuxPanelKind {
    Help,
    Keybindings,
    History,
}

pub fn default_layout() -> DockState<DockTab> {
    DockState::new(vec![DockTab::QueryData { index: 0, title: "查询 1".into() }])
}

// ── 同步：每帧渲染前调用 ──────────────────────────────────────────────

/// 同步 dock 布局与 app 状态（query tabs、ER 图、SQL 编辑器）
pub fn sync_all(state: &mut DockState<DockTab>, app: &DbManagerApp) {
    sync_query_tabs(state, app.tab_manager());
    sync_er_visibility(state, app.show_er_diagram());
    sync_sql_editor_visibility(state, app.show_sql_editor());
}

fn sync_query_tabs(state: &mut DockState<DockTab>, tab_manager: &crate::ui::QueryTabManager) {
    let mgr_count = tab_manager.tabs.len();

    // 1. 移除 dock 中索引越界的 QueryData tab
    remove_tabs(state, |t| {
        matches!(t, DockTab::QueryData { index, .. } if *index >= mgr_count)
    });

    // 2. 修复剩余 tabs 的索引，使其与 tab_manager 顺序一致
    let tree = state.main_surface_mut();
    let mut next_index = 0usize;
    for node in tree.iter_mut() {
        if let Some(tabs) = node.tabs_mut() {
            for tab in tabs.iter_mut() {
                if let DockTab::QueryData { index, title } = tab {
                    if *index < mgr_count {
                        *title = tab_manager.tabs[*index].title.clone();
                    }
                    *index = next_index;
                    next_index += 1;
                }
            }
        }
    }

    // 3. 添加缺失的 tabs — 找到已有的 QueryData leaf 并追加
    if next_index < mgr_count {
        // 找到第一个 QueryData leaf 节点
        let mut target: Option<NodeIndex> = None;
        for (i, node) in tree.iter().enumerate() {
            if node.is_leaf() && node.tabs().unwrap_or(&[]).iter()
                .any(|t| matches!(t, DockTab::QueryData { .. }))
            {
                target = Some(NodeIndex(i));
                break;
            }
        }
        // 如果找到了，追加到该 leaf；否则用 split_right 创建新的
        for i in next_index..mgr_count {
            let title = tab_manager.tabs[i].title.clone();
            let tab = DockTab::QueryData { index: i, title };
            if let Some(ref node_idx) = target {
                tree.push_to_focused_leaf(tab);
                // 确保焦点在正确的 leaf
                let _ = tree.set_focused_node(*node_idx);
            } else {
                let _ = tree.split_right(NodeIndex::root(), 0.3, vec![tab]);
            }
        }
    }
}

fn sync_er_visibility(state: &mut DockState<DockTab>, show: bool) {
    let has_er = {
        if let Some(surface) = state.get_surface(SurfaceIndex::main()) {
            surface.iter_all_tabs().any(|(_, t)| matches!(t, DockTab::ErDiagram))
        } else { false }
    };

    match (show, has_er) {
        (true, false) => {
            let _ = state.main_surface_mut()
                .split_right(NodeIndex::root(), 0.3, vec![DockTab::ErDiagram]);
        }
        (false, true) => remove_tabs(state, |t| matches!(t, DockTab::ErDiagram)),
        _ => {}
    }
}

fn sync_sql_editor_visibility(state: &mut DockState<DockTab>, show: bool) {
    let has_editor = {
        if let Some(surface) = state.get_surface(SurfaceIndex::main()) {
            surface.iter_all_tabs().any(|(_, t)| matches!(t, DockTab::SqlEditor))
        } else { false }
    };

    match (show, has_editor) {
        (true, false) => {
            let _ = state.main_surface_mut()
                .split_below(NodeIndex::root(), 0.25, vec![DockTab::SqlEditor]);
        }
        (false, true) => remove_tabs(state, |t| matches!(t, DockTab::SqlEditor)),
        _ => {}
    }
}

fn remove_tabs<F: Fn(&DockTab) -> bool>(state: &mut DockState<DockTab>, predicate: F) {
    let tree = state.main_surface_mut();
    let mut to_remove: Vec<NodeIndex> = Vec::new();
    for (i, node) in tree.iter().enumerate() {
        if node.is_leaf() {
            for tab in node.tabs().unwrap_or(&[]) {
                if predicate(tab) {
                    to_remove.push(NodeIndex(i));
                    break;
                }
            }
        }
    }
    // 从大到小排序，确保删除时索引保持稳定
    to_remove.sort_by_key(|idx| std::cmp::Reverse(idx.0));
    for idx in to_remove {
        tree.remove_leaf(idx);
    }
}

// ── TabViewer ─────────────────────────────────────────────────────────

pub struct WorkspaceViewer<'a> {
    pub app: &'a mut DbManagerApp,
}

impl TabViewer for WorkspaceViewer<'_> {
    type Tab = DockTab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        match tab {
            DockTab::QueryData { title, .. } => title.as_str().into(),
            DockTab::SqlEditor => "SQL 编辑器".into(),
            DockTab::ErDiagram => "ER 图".into(),
            DockTab::AuxPanel { title, .. } => title.as_str().into(),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        match tab {
            DockTab::QueryData { index, .. } => {
                if self.app.tab_manager().active_index != *index {
                    self.app.tab_manager_mut().active_index = *index;
                    self.app.sync_from_active_tab();
                }
                self.app.render_workspace_content(ui);
            }
            DockTab::SqlEditor => {
                let h = ui.available_height();
                let actions = self.app.render_sql_editor_in_ui(ui, h);
                self.app.handle_sql_editor_actions(actions);
            }
            DockTab::ErDiagram => {
                self.app.render_er_diagram_in_ui(ui);
            }
            DockTab::AuxPanel { kind, .. } => {
                self.app.render_aux_panel(ui, kind.clone());
            }
        }
    }

    fn on_close(&mut self, tab: &mut Self::Tab) -> OnCloseResponse {
        match tab {
            DockTab::QueryData { index, .. } => {
                // 至少保留一个查询标签
                if self.app.tab_manager().tabs.len() <= 1 {
                    return OnCloseResponse::Ignore;
                }
                // 清理：持久化状态、取消查询、移除工作区
                self.app.on_dock_tab_close(*index);
                OnCloseResponse::Close
            }
            DockTab::SqlEditor => {
                self.app.toggle_sql_editor_visibility();
                OnCloseResponse::Close
            }
            DockTab::ErDiagram => {
                self.app.toggle_er_diagram_visibility();
                OnCloseResponse::Close
            }
            DockTab::AuxPanel { .. } => OnCloseResponse::Close,
        }
    }
}
