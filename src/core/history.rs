use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryHistoryItem {
    pub sql: String,
    pub timestamp: DateTime<Local>,
    pub database_type: String,
    pub success: bool,
    pub rows_affected: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryHistory {
    items: Vec<QueryHistoryItem>,
    max_size: usize,
}

impl QueryHistory {
    pub fn new(max_size: usize) -> Self {
        Self {
            items: Vec::new(),
            max_size,
        }
    }

    pub fn add(
        &mut self,
        sql: String,
        database_type: String,
        success: bool,
        rows_affected: Option<u64>,
    ) {
        let item = QueryHistoryItem {
            sql,
            timestamp: Local::now(),
            database_type,
            success,
            rows_affected,
        };

        self.items.insert(0, item);

        // 保持最大历史记录数量
        if self.items.len() > self.max_size {
            self.items.truncate(self.max_size);
        }
    }

    pub fn items(&self) -> &[QueryHistoryItem] {
        &self.items
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }
}
