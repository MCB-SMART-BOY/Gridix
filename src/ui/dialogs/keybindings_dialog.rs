//! 快捷键编辑对话框
//!
//! 允许用户自定义快捷键绑定。

use crate::core::{Action, KeyBinding, KeyBindings, KeyCode, KeyModifiers};
use eframe::egui::{self, Key, RichText};

/// 快捷键编辑对话框状态
#[derive(Default)]
pub struct KeyBindingsDialogState {
    /// 是否显示对话框
    pub show: bool,
    /// 当前快捷键绑定（编辑中的副本）
    bindings: KeyBindings,
    /// 当前选中的操作
    selected_action: Option<Action>,
    /// 是否正在录制快捷键
    recording: bool,
    /// 录制的按键
    recorded_key: Option<KeyCode>,
    /// 录制的修饰键
    recorded_modifiers: KeyModifiers,
    /// 搜索过滤
    filter: String,
    /// 当前显示的分类
    current_category: Option<&'static str>,
    /// 是否有未保存的更改
    has_changes: bool,
    /// 冲突提示
    conflict_message: Option<String>,
}

impl KeyBindingsDialogState {
    /// 打开对话框
    pub fn open(&mut self, bindings: &KeyBindings) {
        self.show = true;
        self.bindings = bindings.clone();
        self.selected_action = None;
        self.recording = false;
        self.recorded_key = None;
        self.recorded_modifiers = KeyModifiers::NONE;
        self.filter.clear();
        self.current_category = None;
        self.has_changes = false;
        self.conflict_message = None;
    }

    /// 关闭对话框
    pub fn close(&mut self) {
        self.show = false;
        self.recording = false;
    }

    /// 重置为默认快捷键
    pub fn reset_to_defaults(&mut self) {
        self.bindings = KeyBindings::default();
        self.has_changes = true;
        self.conflict_message = None;
    }

    /// 获取编辑后的快捷键绑定
    #[allow(dead_code)] // 公开 API，供未来使用
    pub fn get_bindings(&self) -> &KeyBindings {
        &self.bindings
    }

    /// 检查快捷键冲突
    fn check_conflict(&self, action: Action, binding: &KeyBinding) -> Option<Action> {
        for a in Action::all() {
            if *a != action {
                if let Some(existing) = self.bindings.get(*a) {
                    if existing == binding {
                        return Some(*a);
                    }
                }
            }
        }
        None
    }
}

/// 快捷键编辑对话框
pub struct KeyBindingsDialog;

impl KeyBindingsDialog {
    /// 显示对话框
    ///
    /// 返回 Some(KeyBindings) 表示用户保存了更改
    pub fn show(ctx: &egui::Context, state: &mut KeyBindingsDialogState) -> Option<KeyBindings> {
        if !state.show {
            return None;
        }

        let mut result = None;
        let mut should_close = false;

        egui::Window::new("快捷键设置")
            .collapsible(false)
            .resizable(true)
            .default_width(600.0)
            .default_height(500.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                // 顶部工具栏
                ui.horizontal(|ui| {
                    // 搜索框
                    ui.label("搜索:");
                    ui.add(
                        egui::TextEdit::singleline(&mut state.filter)
                            .desired_width(150.0)
                            .hint_text("输入操作名称...")
                    );

                    ui.separator();

                    // 分类筛选
                    ui.label("分类:");
                    egui::ComboBox::from_id_salt("category_filter")
                        .selected_text(state.current_category.unwrap_or("全部"))
                        .show_ui(ui, |ui| {
                            if ui.selectable_label(state.current_category.is_none(), "全部").clicked() {
                                state.current_category = None;
                            }
                            for category in &["全局", "创建", "Tab", "编辑", "缩放"] {
                                if ui.selectable_label(
                                    state.current_category == Some(*category),
                                    *category
                                ).clicked() {
                                    state.current_category = Some(*category);
                                }
                            }
                        });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("重置为默认").clicked() {
                            state.reset_to_defaults();
                        }
                    });
                });

                ui.add_space(8.0);

                // 冲突提示
                if let Some(msg) = &state.conflict_message {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("⚠").color(egui::Color32::YELLOW));
                        ui.label(RichText::new(msg).color(egui::Color32::YELLOW));
                    });
                    ui.add_space(4.0);
                }

                // 快捷键列表
                egui::ScrollArea::vertical()
                    .max_height(350.0)
                    .show(ui, |ui| {
                        egui::Grid::new("keybindings_grid")
                            .num_columns(3)
                            .spacing([20.0, 8.0])
                            .striped(true)
                            .show(ui, |ui| {
                                // 表头
                                ui.label(RichText::new("操作").strong());
                                ui.label(RichText::new("快捷键").strong());
                                ui.label(RichText::new("分类").strong());
                                ui.end_row();

                                let filter_lower = state.filter.to_lowercase();

                                for action in Action::all() {
                                    let desc = action.description();
                                    let category = action.category();

                                    // 应用过滤
                                    if !state.filter.is_empty()
                                        && !desc.to_lowercase().contains(&filter_lower) {
                                        continue;
                                    }

                                    if let Some(cat) = state.current_category {
                                        if category != cat {
                                            continue;
                                        }
                                    }

                                    // 操作名称
                                    let is_selected = state.selected_action == Some(*action);
                                    if ui.selectable_label(is_selected, desc).clicked() {
                                        state.selected_action = Some(*action);
                                        state.recording = false;
                                        state.conflict_message = None;
                                    }

                                    // 快捷键显示/编辑
                                    let binding_text = state.bindings.get(*action)
                                        .map(|b| b.display())
                                        .unwrap_or_else(|| "未设置".to_string());

                                    let is_recording = state.recording && state.selected_action == Some(*action);

                                    if is_recording {
                                        // 显示录制中状态
                                        let recording_text = if state.recorded_key.is_some() {
                                            let binding = KeyBinding::new(
                                                state.recorded_key.unwrap(),
                                                state.recorded_modifiers,
                                            );
                                            binding.display()
                                        } else if state.recorded_modifiers != KeyModifiers::NONE {
                                            format!("{}+...", state.recorded_modifiers)
                                        } else {
                                            "按下快捷键...".to_string()
                                        };

                                        ui.label(
                                            RichText::new(recording_text)
                                                .color(egui::Color32::LIGHT_BLUE)
                                                .italics()
                                        );
                                    } else if ui.button(&binding_text).clicked()
                                        && state.selected_action == Some(*action) {
                                        // 开始录制
                                        state.recording = true;
                                        state.recorded_key = None;
                                        state.recorded_modifiers = KeyModifiers::NONE;
                                        state.conflict_message = None;
                                    }

                                    // 分类
                                    ui.label(category);
                                    ui.end_row();
                                }
                            });
                    });

                ui.add_space(8.0);

                // 录制快捷键时的键盘输入处理
                if state.recording
                    && let Some(action) = state.selected_action {
                    ctx.input(|i| {
                        // 获取修饰键状态
                        let mods = i.modifiers;
                        state.recorded_modifiers = KeyModifiers::from_egui(mods);

                        // 检测按键
                        for event in &i.events {
                            if let egui::Event::Key { key, pressed: true, .. } = event {
                                // 尝试转换为 KeyCode（修饰键会返回 None）
                                if let Some(key_code) = KeyCode::from_egui_key(*key) {
                                    state.recorded_key = Some(key_code);

                                    // 创建新绑定
                                    let new_binding = KeyBinding::new(
                                        key_code,
                                        state.recorded_modifiers,
                                    );

                                    // 检查冲突
                                    if let Some(conflict_action) = state.check_conflict(action, &new_binding) {
                                        state.conflict_message = Some(format!(
                                            "快捷键 {} 已被 \"{}\" 使用",
                                            new_binding.display(),
                                            conflict_action.description()
                                        ));
                                    } else {
                                        // 应用新绑定
                                        state.bindings.set(action, new_binding);
                                        state.has_changes = true;
                                        state.conflict_message = None;
                                    }

                                    state.recording = false;
                                }
                            }
                        }

                        // ESC 取消录制
                        if i.key_pressed(Key::Escape) {
                            state.recording = false;
                            state.recorded_key = None;
                            state.recorded_modifiers = KeyModifiers::NONE;
                        }
                    });
                }

                // 底部按钮
                ui.separator();
                ui.horizontal(|ui| {
                    // 清除选中操作的快捷键
                    if let Some(action) = state.selected_action {
                        if ui.button("清除快捷键").clicked() {
                            state.bindings.remove(action);
                            state.has_changes = true;
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("取消").clicked() {
                            should_close = true;
                        }

                        let save_text = if state.has_changes { "保存 *" } else { "保存" };
                        if ui.button(save_text).clicked() {
                            result = Some(state.bindings.clone());
                            should_close = true;
                        }
                    });
                });

                // 帮助提示
                ui.add_space(4.0);
                ui.label(
                    RichText::new("提示: 选中操作后点击快捷键按钮开始录制，按 ESC 取消")
                        .small()
                        .weak()
                );
            });

        if should_close {
            state.close();
        }

        result
    }
}
