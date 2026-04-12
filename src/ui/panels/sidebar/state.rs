//! 侧边栏状态定义

use crate::database::{RoutineInfo, TriggerInfo};
use crate::ui::SidebarSection;

/// 侧边栏各区域的选中索引
#[derive(Debug, Clone, Default)]
pub struct SidebarSelectionState {
    /// 连接列表选中索引
    pub connections: usize,
    /// 数据库列表选中索引
    pub databases: usize,
    /// 表列表选中索引
    pub tables: usize,
    /// 触发器列表选中索引
    pub triggers: usize,
    /// 存储过程/函数列表选中索引
    pub routines: usize,
    /// 筛选条件选中索引
    pub filters: usize,
}

impl SidebarSelectionState {
    /// 重置数据库相关的选中索引（切换连接时调用）
    pub fn reset_for_connection_change(&mut self) {
        self.databases = 0;
        self.tables = 0;
        self.triggers = 0;
        self.routines = 0;
    }

    /// 重置表相关的选中索引（切换数据库时调用）
    pub fn reset_for_database_change(&mut self) {
        self.tables = 0;
        self.triggers = 0;
        self.routines = 0;
    }
}

/// 筛选工作区的局部模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SidebarFilterWorkspaceMode {
    /// 在筛选规则列表中导航
    #[default]
    List,
    /// 正在编辑筛选值输入框
    Input,
}

/// 侧边栏工作流状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SidebarWorkflowState {
    /// 是否允许在列表边界通过 `j/k` 跨 section 流转
    pub edge_transfer: bool,
    /// 当前筛选工作区局部模式
    pub filter_workspace: SidebarFilterWorkspaceMode,
}

impl Default for SidebarWorkflowState {
    fn default() -> Self {
        Self {
            edge_transfer: true,
            filter_workspace: SidebarFilterWorkspaceMode::List,
        }
    }
}

/// 侧边栏工作流 reducer 所需的只读上下文。
///
/// 这里刻意只放影响 focus graph 的事实，避免 reducer 依赖 egui 或数据库管理器。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SidebarWorkflowContext {
    pub show_connections: bool,
    pub show_filters: bool,
    pub show_triggers: bool,
    pub show_routines: bool,
    pub has_databases: bool,
    pub has_tables: bool,
}

impl SidebarWorkflowContext {
    pub const fn new(
        show_connections: bool,
        show_filters: bool,
        show_triggers: bool,
        show_routines: bool,
        has_databases: bool,
        has_tables: bool,
    ) -> Self {
        Self {
            show_connections,
            show_filters,
            show_triggers,
            show_routines,
            has_databases,
            has_tables,
        }
    }

    fn section_is_available(self, section: SidebarSection) -> bool {
        match section {
            SidebarSection::Connections => self.show_connections,
            SidebarSection::Databases => self.show_connections && self.has_databases,
            SidebarSection::Tables => self.show_connections && self.has_tables,
            SidebarSection::Filters => self.show_filters,
            SidebarSection::Triggers => self.show_triggers,
            SidebarSection::Routines => self.show_routines,
        }
    }

    fn next_section(self, current: SidebarSection) -> Option<SidebarSection> {
        let current_index = SIDEBAR_FOCUS_ORDER
            .iter()
            .position(|section| *section == current)?;

        SIDEBAR_FOCUS_ORDER
            .iter()
            .copied()
            .skip(current_index + 1)
            .find(|section| self.section_is_available(*section))
    }

    fn previous_section(self, current: SidebarSection) -> Option<SidebarSection> {
        let current_index = SIDEBAR_FOCUS_ORDER
            .iter()
            .position(|section| *section == current)?;

        SIDEBAR_FOCUS_ORDER[..current_index]
            .iter()
            .rev()
            .copied()
            .find(|section| self.section_is_available(*section))
    }

    fn layer_right_target(self, current: SidebarSection) -> Option<SidebarWorkflowEffect> {
        match current {
            SidebarSection::Connections if self.section_is_available(SidebarSection::Databases) => {
                Some(SidebarWorkflowEffect::SectionChanged(
                    SidebarSection::Databases,
                ))
            }
            SidebarSection::Databases if self.section_is_available(SidebarSection::Tables) => Some(
                SidebarWorkflowEffect::SectionChanged(SidebarSection::Tables),
            ),
            SidebarSection::Tables => Some(SidebarWorkflowEffect::FocusTransferToDataGrid),
            _ => None,
        }
    }
}

const SIDEBAR_FOCUS_ORDER: [SidebarSection; 6] = [
    SidebarSection::Connections,
    SidebarSection::Databases,
    SidebarSection::Tables,
    SidebarSection::Filters,
    SidebarSection::Triggers,
    SidebarSection::Routines,
];

/// 侧边栏工作流层动作。
///
/// UI 层负责把按键/点击翻译成这些动作；reducer 只负责 section、edge transfer
/// 和 filters.list / filters.input 的状态语义。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarWorkflowAction {
    FocusSection(SidebarSection),
    MoveLeft {
        current: SidebarSection,
    },
    MoveRight {
        current: SidebarSection,
        selected_filter_index: usize,
        filter_needs_value: bool,
    },
    EdgeNext {
        current: SidebarSection,
    },
    EdgePrevious {
        current: SidebarSection,
    },
    EnterFilterInput {
        index: usize,
        filter_needs_value: bool,
    },
    ExitFilterInput,
}

/// reducer 输出的副作用请求，仍由现有 SidebarActions 兼容层执行。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarWorkflowEffect {
    SectionChanged(SidebarSection),
    FocusFilterInput(usize),
    FocusTransferToDataGrid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SidebarWorkflowReduction {
    pub effect: Option<SidebarWorkflowEffect>,
}

impl SidebarWorkflowReduction {
    fn section_changed(section: SidebarSection) -> Self {
        Self {
            effect: Some(SidebarWorkflowEffect::SectionChanged(section)),
        }
    }

    fn focus_filter_input(index: usize) -> Self {
        Self {
            effect: Some(SidebarWorkflowEffect::FocusFilterInput(index)),
        }
    }
}

pub fn reduce_sidebar_workflow(
    workflow: &mut SidebarWorkflowState,
    context: SidebarWorkflowContext,
    action: SidebarWorkflowAction,
) -> SidebarWorkflowReduction {
    match action {
        SidebarWorkflowAction::FocusSection(section) => {
            workflow.filter_workspace = SidebarFilterWorkspaceMode::List;
            if context.section_is_available(section) {
                SidebarWorkflowReduction::section_changed(section)
            } else {
                SidebarWorkflowReduction::default()
            }
        }
        SidebarWorkflowAction::MoveLeft { current } => {
            workflow.filter_workspace = SidebarFilterWorkspaceMode::List;
            context
                .previous_section(current)
                .map(SidebarWorkflowReduction::section_changed)
                .unwrap_or_default()
        }
        SidebarWorkflowAction::MoveRight {
            current,
            selected_filter_index,
            filter_needs_value,
        } => {
            if current == SidebarSection::Filters && filter_needs_value {
                workflow.filter_workspace = SidebarFilterWorkspaceMode::Input;
                return SidebarWorkflowReduction::focus_filter_input(selected_filter_index);
            }

            workflow.filter_workspace = SidebarFilterWorkspaceMode::List;
            context
                .layer_right_target(current)
                .map(|effect| SidebarWorkflowReduction {
                    effect: Some(effect),
                })
                .unwrap_or_default()
        }
        SidebarWorkflowAction::EdgeNext { current } => {
            if !workflow.edge_transfer {
                return SidebarWorkflowReduction::default();
            }

            context
                .next_section(current)
                .map(SidebarWorkflowReduction::section_changed)
                .unwrap_or_default()
        }
        SidebarWorkflowAction::EdgePrevious { current } => {
            if !workflow.edge_transfer {
                return SidebarWorkflowReduction::default();
            }

            context
                .previous_section(current)
                .map(SidebarWorkflowReduction::section_changed)
                .unwrap_or_default()
        }
        SidebarWorkflowAction::EnterFilterInput {
            index,
            filter_needs_value,
        } => {
            if filter_needs_value {
                workflow.filter_workspace = SidebarFilterWorkspaceMode::Input;
                SidebarWorkflowReduction::focus_filter_input(index)
            } else {
                workflow.filter_workspace = SidebarFilterWorkspaceMode::List;
                SidebarWorkflowReduction::default()
            }
        }
        SidebarWorkflowAction::ExitFilterInput => {
            workflow.filter_workspace = SidebarFilterWorkspaceMode::List;
            SidebarWorkflowReduction::default()
        }
    }
}

/// 侧边栏面板状态
#[derive(Debug, Clone)]
pub struct SidebarPanelState {
    // ===== 连接面板 =====
    /// 连接列表面板是否显示
    pub show_connections: bool,
    /// 连接面板高度比例 (相对于可用空间)
    pub connections_ratio: f32,

    // ===== 触发器面板 =====
    /// 触发器面板是否显示
    pub show_triggers: bool,
    /// 触发器面板高度比例
    pub triggers_ratio: f32,
    /// 触发器列表
    pub triggers: Vec<TriggerInfo>,
    /// 触发器列表中的选中索引（保留向后兼容）
    pub trigger_selected_index: usize,
    /// 是否正在加载触发器
    pub loading_triggers: bool,

    // ===== 存储过程面板 =====
    /// 存储过程/函数面板是否显示
    pub show_routines: bool,
    /// 存储过程面板高度比例
    pub routines_ratio: f32,
    /// 存储过程/函数列表
    pub routines: Vec<RoutineInfo>,
    /// 存储过程/函数选中索引
    pub routine_selected_index: usize,
    /// 是否正在加载存储过程
    pub loading_routines: bool,

    // ===== 筛选面板 =====
    /// 筛选面板是否显示
    pub show_filters: bool,
    /// 筛选面板高度比例
    pub filters_ratio: f32,

    // ===== 其他状态 =====
    /// 各区域的选中状态
    pub selection: SidebarSelectionState,
    /// 当前正在拖动的分割条索引 (0=连接/筛选, 1=筛选/触发器, 2=触发器/存储过程)
    pub dragging_divider: Option<usize>,
    /// 命令缓冲区（用于多键命令如 gs）
    pub command_buffer: String,
    /// 筛选值输入框当前是否持有文本焦点
    pub filter_input_has_focus: bool,
    /// 侧边栏工作流状态
    pub workflow: SidebarWorkflowState,
}

impl Default for SidebarPanelState {
    fn default() -> Self {
        Self {
            // 连接面板 - 默认显示，占主要空间
            show_connections: true,
            connections_ratio: 0.65,

            // 触发器面板 - 默认关闭，按需展开
            show_triggers: false,
            triggers_ratio: 0.2,
            triggers: Vec::new(),
            trigger_selected_index: 0,
            loading_triggers: false,

            // 存储过程面板 - 默认关闭，按需展开
            show_routines: false,
            routines_ratio: 0.2,
            routines: Vec::new(),
            routine_selected_index: 0,
            loading_routines: false,

            // 筛选面板 - 默认显示，和连接面板形成新手默认布局
            show_filters: true,
            filters_ratio: 0.35,

            selection: SidebarSelectionState::default(),
            dragging_divider: None,
            command_buffer: String::new(),
            filter_input_has_focus: false,
            workflow: SidebarWorkflowState::default(),
        }
    }
}

impl SidebarPanelState {
    /// 开始一个新的筛选工作区渲染周期
    pub fn begin_filter_workspace_frame(&mut self) {
        self.filter_input_has_focus = false;
        self.workflow.filter_workspace = SidebarFilterWorkspaceMode::List;
    }

    /// 标记筛选输入框已获得焦点
    pub fn mark_filter_input_focus(&mut self) {
        self.filter_input_has_focus = true;
        self.workflow.filter_workspace = SidebarFilterWorkspaceMode::Input;
    }

    /// 主动退出筛选输入模式
    pub fn exit_filter_input(&mut self) {
        self.filter_input_has_focus = false;
        self.workflow.filter_workspace = SidebarFilterWorkspaceMode::List;
    }

    /// 当前筛选工作区是否处于输入模式
    pub fn filter_input_mode(&self) -> bool {
        self.workflow.filter_workspace == SidebarFilterWorkspaceMode::Input
    }

    /// 清空触发器列表
    pub fn clear_triggers(&mut self) {
        self.triggers.clear();
        self.trigger_selected_index = 0;
        self.selection.triggers = 0;
    }

    /// 设置触发器列表
    pub fn set_triggers(&mut self, triggers: Vec<TriggerInfo>) {
        self.triggers = triggers;
        self.trigger_selected_index = 0;
        self.selection.triggers = 0;
        self.loading_triggers = false;
    }

    /// 清空存储过程/函数列表
    pub fn clear_routines(&mut self) {
        self.routines.clear();
        self.routine_selected_index = 0;
        self.selection.routines = 0;
    }

    /// 设置存储过程/函数列表
    pub fn set_routines(&mut self, routines: Vec<RoutineInfo>) {
        self.routines = routines;
        self.routine_selected_index = 0;
        self.selection.routines = 0;
        self.loading_routines = false;
    }
}

#[cfg(test)]
mod tests {
    use super::{
        SidebarFilterWorkspaceMode, SidebarPanelState, SidebarWorkflowAction,
        SidebarWorkflowContext, SidebarWorkflowEffect, SidebarWorkflowState,
        reduce_sidebar_workflow,
    };
    use crate::ui::SidebarSection;

    fn flow(
        show_connections: bool,
        show_filters: bool,
        show_triggers: bool,
        show_routines: bool,
        has_databases: bool,
        has_tables: bool,
    ) -> SidebarWorkflowContext {
        SidebarWorkflowContext::new(
            show_connections,
            show_filters,
            show_triggers,
            show_routines,
            has_databases,
            has_tables,
        )
    }

    fn reduce(
        workflow: &mut SidebarWorkflowState,
        action: SidebarWorkflowAction,
        context: SidebarWorkflowContext,
    ) -> Option<SidebarWorkflowEffect> {
        reduce_sidebar_workflow(workflow, context, action).effect
    }

    #[test]
    fn default_sidebar_layout_prioritizes_connections_and_filters() {
        let state = SidebarPanelState::default();

        assert!(state.show_connections);
        assert!(state.show_filters);
        assert!(!state.show_triggers);
        assert!(!state.show_routines);
        assert!(state.connections_ratio > state.filters_ratio);
        assert!(state.workflow.edge_transfer);
        assert_eq!(
            state.workflow.filter_workspace,
            SidebarFilterWorkspaceMode::List
        );
    }

    #[test]
    fn tables_edge_down_enters_filters_when_filter_panel_is_open() {
        let mut workflow = SidebarWorkflowState::default();

        assert_eq!(
            reduce(
                &mut workflow,
                SidebarWorkflowAction::EdgeNext {
                    current: SidebarSection::Tables
                },
                flow(true, true, false, false, true, true),
            ),
            Some(SidebarWorkflowEffect::SectionChanged(
                SidebarSection::Filters
            ))
        );
    }

    #[test]
    fn tables_move_right_enters_data_grid_even_when_filters_are_visible() {
        let mut workflow = SidebarWorkflowState::default();

        assert_eq!(
            reduce(
                &mut workflow,
                SidebarWorkflowAction::MoveRight {
                    current: SidebarSection::Tables,
                    selected_filter_index: 0,
                    filter_needs_value: false,
                },
                flow(true, true, true, false, true, true),
            ),
            Some(SidebarWorkflowEffect::FocusTransferToDataGrid)
        );
    }

    #[test]
    fn tables_move_right_enters_data_grid_when_filter_workspace_is_hidden() {
        let mut workflow = SidebarWorkflowState::default();

        assert_eq!(
            reduce(
                &mut workflow,
                SidebarWorkflowAction::MoveRight {
                    current: SidebarSection::Tables,
                    selected_filter_index: 0,
                    filter_needs_value: false,
                },
                flow(true, false, false, false, true, true),
            ),
            Some(SidebarWorkflowEffect::FocusTransferToDataGrid)
        );
    }

    #[test]
    fn filters_move_right_prefers_value_input_when_rule_needs_text() {
        let mut workflow = SidebarWorkflowState::default();

        assert_eq!(
            reduce(
                &mut workflow,
                SidebarWorkflowAction::MoveRight {
                    current: SidebarSection::Filters,
                    selected_filter_index: 2,
                    filter_needs_value: true,
                },
                flow(true, true, true, true, true, true),
            ),
            Some(SidebarWorkflowEffect::FocusFilterInput(2))
        );
        assert_eq!(workflow.filter_workspace, SidebarFilterWorkspaceMode::Input);
    }

    #[test]
    fn filters_move_right_stays_local_when_no_value_is_needed() {
        let mut workflow = SidebarWorkflowState::default();

        assert_eq!(
            reduce(
                &mut workflow,
                SidebarWorkflowAction::MoveRight {
                    current: SidebarSection::Filters,
                    selected_filter_index: 0,
                    filter_needs_value: false,
                },
                flow(true, true, true, false, true, true),
            ),
            None
        );
    }

    #[test]
    fn connections_move_right_does_not_fall_through_to_filters_without_database_hierarchy() {
        let mut workflow = SidebarWorkflowState::default();

        assert_eq!(
            reduce(
                &mut workflow,
                SidebarWorkflowAction::MoveRight {
                    current: SidebarSection::Connections,
                    selected_filter_index: 0,
                    filter_needs_value: false,
                },
                flow(true, true, false, false, false, false),
            ),
            None
        );
    }

    #[test]
    fn databases_move_right_does_not_fall_through_to_filters_when_tables_are_unavailable() {
        let mut workflow = SidebarWorkflowState::default();

        assert_eq!(
            reduce(
                &mut workflow,
                SidebarWorkflowAction::MoveRight {
                    current: SidebarSection::Databases,
                    selected_filter_index: 0,
                    filter_needs_value: false,
                },
                flow(true, true, false, false, true, false),
            ),
            None
        );
    }

    #[test]
    fn connections_move_right_prefers_database_hierarchy_before_grid() {
        let mut workflow = SidebarWorkflowState::default();

        assert_eq!(
            reduce(
                &mut workflow,
                SidebarWorkflowAction::MoveRight {
                    current: SidebarSection::Connections,
                    selected_filter_index: 0,
                    filter_needs_value: false,
                },
                flow(true, true, false, false, true, true),
            ),
            Some(SidebarWorkflowEffect::SectionChanged(
                SidebarSection::Databases
            ))
        );
    }

    #[test]
    fn sidebar_focus_graph_uses_move_right_only_for_layer_depth() {
        let context = flow(true, true, true, true, true, true);
        let mut workflow = SidebarWorkflowState::default();

        assert_eq!(
            reduce(
                &mut workflow,
                SidebarWorkflowAction::MoveRight {
                    current: SidebarSection::Connections,
                    selected_filter_index: 0,
                    filter_needs_value: false,
                },
                context,
            ),
            Some(SidebarWorkflowEffect::SectionChanged(
                SidebarSection::Databases
            ))
        );
        assert_eq!(
            reduce(
                &mut workflow,
                SidebarWorkflowAction::MoveRight {
                    current: SidebarSection::Databases,
                    selected_filter_index: 0,
                    filter_needs_value: false,
                },
                context,
            ),
            Some(SidebarWorkflowEffect::SectionChanged(
                SidebarSection::Tables
            ))
        );
        assert_eq!(
            reduce(
                &mut workflow,
                SidebarWorkflowAction::MoveRight {
                    current: SidebarSection::Tables,
                    selected_filter_index: 0,
                    filter_needs_value: false,
                },
                context,
            ),
            Some(SidebarWorkflowEffect::FocusTransferToDataGrid)
        );
        assert_eq!(
            reduce(
                &mut workflow,
                SidebarWorkflowAction::MoveRight {
                    current: SidebarSection::Filters,
                    selected_filter_index: 0,
                    filter_needs_value: false,
                },
                context,
            ),
            None
        );
        assert_eq!(
            reduce(
                &mut workflow,
                SidebarWorkflowAction::MoveRight {
                    current: SidebarSection::Triggers,
                    selected_filter_index: 0,
                    filter_needs_value: false,
                },
                context,
            ),
            None
        );
        assert_eq!(
            reduce(
                &mut workflow,
                SidebarWorkflowAction::MoveLeft {
                    current: SidebarSection::Routines
                },
                context,
            ),
            Some(SidebarWorkflowEffect::SectionChanged(
                SidebarSection::Triggers
            ))
        );
        assert_eq!(
            reduce(
                &mut workflow,
                SidebarWorkflowAction::MoveLeft {
                    current: SidebarSection::Triggers
                },
                context,
            ),
            Some(SidebarWorkflowEffect::SectionChanged(
                SidebarSection::Filters
            ))
        );
    }

    #[test]
    fn filters_edge_up_back_to_tables_in_default_learning_flow() {
        let mut workflow = SidebarWorkflowState::default();

        assert_eq!(
            reduce(
                &mut workflow,
                SidebarWorkflowAction::EdgePrevious {
                    current: SidebarSection::Filters
                },
                flow(true, true, false, false, true, true),
            ),
            Some(SidebarWorkflowEffect::SectionChanged(
                SidebarSection::Tables
            ))
        );
    }

    #[test]
    fn filters_fall_through_to_triggers_when_enabled() {
        let mut workflow = SidebarWorkflowState::default();

        assert_eq!(
            reduce(
                &mut workflow,
                SidebarWorkflowAction::EdgeNext {
                    current: SidebarSection::Filters
                },
                flow(true, true, true, false, false, true),
            ),
            Some(SidebarWorkflowEffect::SectionChanged(
                SidebarSection::Triggers
            ))
        );
    }

    #[test]
    fn edge_transfer_can_be_disabled() {
        let mut workflow = SidebarWorkflowState {
            edge_transfer: false,
            filter_workspace: SidebarFilterWorkspaceMode::List,
        };

        assert_eq!(
            reduce(
                &mut workflow,
                SidebarWorkflowAction::EdgeNext {
                    current: SidebarSection::Tables
                },
                flow(true, true, false, false, true, true),
            ),
            None
        );
    }

    #[test]
    fn explicit_filter_input_actions_switch_modes() {
        let mut workflow = SidebarWorkflowState::default();

        assert_eq!(
            reduce(
                &mut workflow,
                SidebarWorkflowAction::EnterFilterInput {
                    index: 1,
                    filter_needs_value: true,
                },
                flow(true, true, false, false, true, true),
            ),
            Some(SidebarWorkflowEffect::FocusFilterInput(1))
        );
        assert_eq!(workflow.filter_workspace, SidebarFilterWorkspaceMode::Input);

        assert_eq!(
            reduce(
                &mut workflow,
                SidebarWorkflowAction::ExitFilterInput,
                flow(true, true, false, false, true, true),
            ),
            None
        );
        assert_eq!(workflow.filter_workspace, SidebarFilterWorkspaceMode::List);
    }
}
