//! 表格虚拟行视图
//!
//! 将查询结果中的已有行和未保存的新行统一成同一种可导航视图。

use crate::domain::result::ResultSet;

#[derive(Debug, Clone, Copy)]
pub(crate) enum GridVirtualRow<'a> {
    Existing {
        row_key: usize,
        row_data: &'a [crate::domain::value::DbValue],
    },
    PendingNew {
        row_key: usize,
        row_data: &'a [String],
    },
}

impl GridVirtualRow<'_> {
    pub(crate) fn row_key(self) -> usize {
        match self {
            Self::Existing { row_key, .. } | Self::PendingNew { row_key, .. } => row_key,
        }
    }

    pub(crate) fn display_row(self) -> Vec<String> {
        match self {
            Self::Existing { row_data, .. } => row_data.iter().map(|cell| cell.display()).collect(),
            Self::PendingNew { row_data, .. } => row_data.to_vec(),
        }
    }

    pub(crate) fn display_cell(self, column_index: usize) -> Option<String> {
        match self {
            Self::Existing { row_data, .. } => {
                row_data.get(column_index).map(|cell| cell.display())
            }
            Self::PendingNew { row_data, .. } => row_data.get(column_index).cloned(),
        }
    }
}

pub(crate) struct GridVirtualRows<'a> {
    result: &'a ResultSet,
    filtered_row_indices: &'a [usize],
    new_rows: &'a [Vec<String>],
}

impl<'a> GridVirtualRows<'a> {
    pub(crate) fn new(
        result: &'a ResultSet,
        filtered_row_indices: &'a [usize],
        new_rows: &'a [Vec<String>],
    ) -> Self {
        Self {
            result,
            filtered_row_indices,
            new_rows,
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.filtered_row_indices.len() + self.new_rows.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub(crate) fn row_at_display_index(&self, display_idx: usize) -> Option<GridVirtualRow<'a>> {
        if let Some(row_key) = self.filtered_row_indices.get(display_idx) {
            return Some(GridVirtualRow::Existing {
                row_key: *row_key,
                row_data: self.result.row(*row_key),
            });
        }

        let new_row_idx = display_idx.checked_sub(self.filtered_row_indices.len())?;
        self.new_rows
            .get(new_row_idx)
            .map(|row_data| GridVirtualRow::PendingNew {
                row_key: self.result.row_count + new_row_idx,
                row_data,
            })
    }

    pub(crate) fn row_at_row_key(&self, row_key: usize) -> Option<GridVirtualRow<'a>> {
        self.display_index_for_row_key(row_key)
            .and_then(|display_idx| self.row_at_display_index(display_idx))
    }

    pub(crate) fn row_key_at_display_index(&self, display_idx: usize) -> Option<usize> {
        self.row_at_display_index(display_idx)
            .map(GridVirtualRow::row_key)
    }

    pub(crate) fn display_index_for_row_key(&self, row_key: usize) -> Option<usize> {
        if row_key < self.result.row_count {
            return self
                .filtered_row_indices
                .iter()
                .position(|existing_row_key| *existing_row_key == row_key);
        }

        let new_row_idx = row_key.checked_sub(self.result.row_count)?;
        (new_row_idx < self.new_rows.len()).then_some(self.filtered_row_indices.len() + new_row_idx)
    }
}
