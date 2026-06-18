//! Shared dockable surface chrome.

use eframe::egui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceAction {
    pub id: &'static str,
    pub icon: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub command_id: Option<&'static str>,
    pub shortcut: Option<&'static str>,
}

impl SurfaceAction {
    pub const fn new(
        id: &'static str,
        icon: &'static str,
        label: &'static str,
        description: &'static str,
        command_id: Option<&'static str>,
        shortcut: Option<&'static str>,
    ) -> Self {
        Self {
            id,
            icon,
            label,
            description,
            command_id,
            shortcut,
        }
    }

    pub fn tooltip(self) -> String {
        surface_tooltip(self.label, self.description, self.command_id, self.shortcut)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkbenchSurfaceHeaderResponse {
    pub clicked_action: Option<&'static str>,
}

pub struct WorkbenchSurfaceHeader<'a> {
    icon: &'a str,
    title: &'a str,
    context: Option<&'a str>,
    actions: &'a [SurfaceAction],
    show_title: bool,
}

impl<'a> WorkbenchSurfaceHeader<'a> {
    pub fn new(icon: &'a str, title: &'a str) -> Self {
        Self {
            icon,
            title,
            context: None,
            actions: &[],
            show_title: true,
        }
    }

    pub fn context(mut self, context: Option<&'a str>) -> Self {
        self.context = context;
        self
    }

    pub fn actions(mut self, actions: &'a [SurfaceAction]) -> Self {
        self.actions = actions;
        self
    }

    pub fn show_title(mut self, show_title: bool) -> Self {
        self.show_title = show_title;
        self
    }

    pub fn show(self, ui: &mut egui::Ui) -> WorkbenchSurfaceHeaderResponse {
        let mut response = WorkbenchSurfaceHeaderResponse::default();

        ui.horizontal(|ui| {
            ui.set_min_height(32.0);
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 0.0);

            ui.label(egui::RichText::new(self.icon).monospace().strong());
            if self.show_title {
                ui.label(egui::RichText::new(self.title).strong());
            }
            if let Some(context) = self.context.filter(|value| !value.is_empty()) {
                ui.label(egui::RichText::new(context).color(ui.visuals().weak_text_color()));
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                for action in self.actions.iter().rev() {
                    if surface_icon_button(ui, *action) {
                        response.clicked_action = Some(action.id);
                    }
                }
            });
        });

        response
    }
}

pub fn surface_icon_button(ui: &mut egui::Ui, action: SurfaceAction) -> bool {
    ui.add(
        egui::Button::new(egui::RichText::new(action.icon).monospace())
            .frame(false)
            .min_size(egui::vec2(26.0, 24.0)),
    )
    .on_hover_text(action.tooltip())
    .clicked()
}

pub fn surface_icon_glyph(icon_key: &str) -> &'static str {
    match icon_key {
        "database-tree" => "▦",
        "filter" => "◇",
        "objects" => "{}",
        "history" => "↺",
        "gear" => "⚙",
        "help" => "?",
        "sql" => "SQL",
        "table" | "grid" => "▤",
        "explain" => "EX",
        "graph" => "◇",
        "schema" => "□",
        "log" => "!",
        "tasks" => "✓",
        "info" => "i",
        "welcome" => "·",
        _ => "•",
    }
}

pub fn surface_tooltip(
    label: &str,
    description: &str,
    command_id: Option<&str>,
    shortcut: Option<&str>,
) -> String {
    let mut tooltip = format!("{}\n{}", label, description);
    if let Some(shortcut) = shortcut.filter(|value| !value.is_empty()) {
        tooltip.push_str(&format!("\n{}", shortcut));
    }
    if let Some(command_id) = command_id.filter(|value| !value.is_empty()) {
        tooltip.push_str(&format!("\nCommand: {}", command_id));
    }
    tooltip
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_action_tooltip_includes_label_shortcut_and_command() {
        let action = SurfaceAction::new(
            "run",
            "run",
            "Run Query",
            "执行当前 SQL 文档",
            Some("query.run"),
            Some("Ctrl+Enter"),
        );

        let tooltip = action.tooltip();

        assert!(tooltip.contains("Run Query"));
        assert!(tooltip.contains("执行当前 SQL 文档"));
        assert!(tooltip.contains("Ctrl+Enter"));
        assert!(tooltip.contains("Command: query.run"));
    }

    #[test]
    fn surface_tooltip_omits_missing_optional_metadata() {
        assert_eq!(
            surface_tooltip("Explorer", "显示连接、数据库和表", None, None),
            "Explorer\n显示连接、数据库和表"
        );
    }

    #[test]
    fn surface_icon_glyph_maps_descriptor_keys_to_compact_symbols() {
        assert_eq!(surface_icon_glyph("database-tree"), "▦");
        assert_eq!(surface_icon_glyph("filter"), "◇");
        assert_eq!(surface_icon_glyph("objects"), "{}");
        assert_eq!(surface_icon_glyph("unknown"), "•");
    }
}
