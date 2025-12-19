//! 核心模块测试
//!
//! 测试 SQL 格式化、自动补全、历史记录等功能。

use gridix::core::{format_sql, AutoComplete, CompletionKind};

#[cfg(test)]
mod sql_formatter_tests {
    use super::*;

    #[test]
    fn test_format_simple_select() {
        let sql = "select * from users where id=1";
        let formatted = format_sql(sql);
        
        // 应该大写关键字
        assert!(formatted.contains("SELECT"));
        assert!(formatted.contains("FROM"));
        assert!(formatted.contains("WHERE"));
    }

    #[test]
    fn test_format_preserves_strings() {
        let sql = "SELECT * FROM users WHERE name = 'select from'";
        let formatted = format_sql(sql);
        
        // 字符串内容不应被修改
        assert!(formatted.contains("'select from'"));
    }

    #[test]
    fn test_format_empty_sql() {
        let sql = "";
        let formatted = format_sql(sql);
        assert!(formatted.is_empty());
    }

    #[test]
    fn test_format_whitespace_only() {
        let sql = "   \n\t  ";
        let formatted = format_sql(sql);
        assert!(formatted.trim().is_empty());
    }
}

#[cfg(test)]
mod autocomplete_tests {
    use super::*;

    #[test]
    fn test_autocomplete_keywords() {
        let ac = AutoComplete::new();
        let completions = ac.get_completions("SEL", 3);
        
        // 应该包含 SELECT
        assert!(completions.iter().any(|c| c.label == "SELECT"));
    }

    #[test]
    fn test_autocomplete_functions() {
        let ac = AutoComplete::new();
        let completions = ac.get_completions("COU", 3);
        
        // 应该包含 COUNT
        assert!(completions.iter().any(|c| c.label == "COUNT"));
    }

    #[test]
    fn test_autocomplete_tables() {
        let mut ac = AutoComplete::new();
        ac.set_tables(vec!["users".to_string(), "orders".to_string()]);
        
        let completions = ac.get_completions("use", 3);
        
        // 应该包含 users 表
        assert!(completions.iter().any(|c| c.label == "users"));
    }

    #[test]
    fn test_autocomplete_empty_input() {
        let ac = AutoComplete::new();
        let completions = ac.get_completions("", 0);
        
        // 空输入不应返回补全
        assert!(completions.is_empty());
    }

    #[test]
    fn test_autocomplete_case_insensitive() {
        let ac = AutoComplete::new();
        
        let lower = ac.get_completions("sel", 3);
        let upper = ac.get_completions("SEL", 3);
        
        // 应该返回相同的结果
        assert_eq!(lower.len(), upper.len());
    }

    #[test]
    fn test_completion_kind_icon() {
        assert_eq!(CompletionKind::Keyword.icon(), "K");
        assert_eq!(CompletionKind::Function.icon(), "F");
        assert_eq!(CompletionKind::Table.icon(), "T");
        assert_eq!(CompletionKind::Column.icon(), "C");
    }
}

#[cfg(test)]
mod query_history_tests {
    use gridix::core::QueryHistory;

    #[test]
    fn test_empty_history() {
        let history = QueryHistory::new(10);
        assert!(history.is_empty());
    }

    #[test]
    fn test_add_entry() {
        let mut history = QueryHistory::new(10);
        history.add("SELECT 1".to_string(), "SQLite".to_string(), true, None);
        
        assert_eq!(history.len(), 1);
        assert_eq!(history.items()[0].sql, "SELECT 1");
    }

    #[test]
    fn test_max_entries() {
        let mut history = QueryHistory::new(3);
        
        history.add("SQL 1".to_string(), "SQLite".to_string(), true, None);
        history.add("SQL 2".to_string(), "SQLite".to_string(), true, None);
        history.add("SQL 3".to_string(), "SQLite".to_string(), true, None);
        history.add("SQL 4".to_string(), "SQLite".to_string(), true, None);
        
        // 应该只保留最新的 3 条
        assert_eq!(history.len(), 3);
        assert_eq!(history.items()[0].sql, "SQL 4");
    }

    #[test]
    fn test_clear_history() {
        let mut history = QueryHistory::new(10);
        history.add("SELECT 1".to_string(), "SQLite".to_string(), true, None);
        history.add("SELECT 2".to_string(), "SQLite".to_string(), true, None);
        
        history.clear();
        
        assert!(history.is_empty());
    }
}

#[cfg(test)]
mod theme_tests {
    use gridix::core::{ThemeManager, ThemePreset};

    #[test]
    fn test_theme_presets() {
        // 测试所有预设主题
        let presets = vec![
            ThemePreset::TokyoNight,
            ThemePreset::TokyoNightStorm,
            ThemePreset::TokyoNightLight,
            ThemePreset::CatppuccinMocha,
            ThemePreset::OneDark,
            ThemePreset::GruvboxDark,
            ThemePreset::Dracula,
            ThemePreset::Nord,
        ];

        for preset in presets {
            let manager = ThemeManager::new(preset);
            // 确保颜色有效
            assert!(manager.colors.accent.r() > 0 || manager.colors.accent.g() > 0 || manager.colors.accent.b() > 0);
        }
    }

    #[test]
    fn test_theme_manager_set_theme() {
        let mut manager = ThemeManager::new(ThemePreset::TokyoNight);
        let original_accent = manager.colors.accent;
        
        manager.set_theme(ThemePreset::Dracula);
        
        // 主题应该已更改
        assert_ne!(manager.colors.accent, original_accent);
    }
}
