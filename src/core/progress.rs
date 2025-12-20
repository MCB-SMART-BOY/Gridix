//! 进度反馈系统
//!
//! 提供长时间操作的进度跟踪和显示功能。

#![allow(dead_code)] // 公开 API，供外部使用

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// 进度任务
#[derive(Debug, Clone)]
pub struct ProgressTask {
    /// 任务 ID
    pub id: u64,
    /// 任务描述
    pub description: String,
    /// 进度值 (0.0 - 1.0)，None 表示不确定进度
    pub progress: Option<f32>,
    /// 是否可取消
    pub cancellable: bool,
    /// 取消标志
    cancelled: Arc<AtomicBool>,
    /// 开始时间
    pub started_at: Instant,
}

impl ProgressTask {
    /// 创建新任务
    fn new(id: u64, description: impl Into<String>, cancellable: bool) -> Self {
        Self {
            id,
            description: description.into(),
            progress: None,
            cancellable,
            cancelled: Arc::new(AtomicBool::new(false)),
            started_at: Instant::now(),
        }
    }

    /// 检查任务是否已被取消
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    /// 获取取消标志的克隆（用于异步任务）
    pub fn cancel_token(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.cancelled)
    }

    /// 取消任务
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    /// 获取已运行时间（毫秒）
    pub fn elapsed_ms(&self) -> u64 {
        self.started_at.elapsed().as_millis() as u64
    }
}

/// 进度管理器
///
/// 管理所有活跃的进度任务，支持：
/// - 创建新任务
/// - 更新任务进度
/// - 取消任务
/// - 查询活跃任务
#[derive(Debug, Default)]
pub struct ProgressManager {
    /// 任务映射
    tasks: HashMap<u64, ProgressTask>,
    /// 下一个任务 ID
    next_id: u64,
}

impl ProgressManager {
    /// 创建新的进度管理器
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            next_id: 1,
        }
    }

    /// 开始一个新任务
    ///
    /// # 参数
    /// - `description`: 任务描述
    /// - `cancellable`: 是否可取消
    ///
    /// # 返回
    /// 任务 ID
    pub fn start(&mut self, description: impl Into<String>, cancellable: bool) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        let task = ProgressTask::new(id, description, cancellable);
        self.tasks.insert(id, task);

        id
    }

    /// 更新任务进度
    ///
    /// # 参数
    /// - `id`: 任务 ID
    /// - `progress`: 进度值 (0.0 - 1.0)
    pub fn update(&mut self, id: u64, progress: f32) {
        if let Some(task) = self.tasks.get_mut(&id) {
            task.progress = Some(progress.clamp(0.0, 1.0));
        }
    }

    /// 完成任务
    pub fn finish(&mut self, id: u64) {
        self.tasks.remove(&id);
    }

    /// 取消任务
    pub fn cancel(&mut self, id: u64) {
        if let Some(task) = self.tasks.get(&id) {
            if task.cancellable {
                task.cancel();
            }
        }
        self.tasks.remove(&id);
    }

    /// 获取指定任务
    pub fn get(&self, id: u64) -> Option<&ProgressTask> {
        self.tasks.get(&id)
    }

    /// 获取所有活跃任务
    pub fn active_tasks(&self) -> Vec<&ProgressTask> {
        self.tasks.values().collect()
    }

    /// 检查是否有活跃任务
    pub fn has_active_tasks(&self) -> bool {
        !self.tasks.is_empty()
    }

    /// 获取活跃任务数量
    pub fn active_count(&self) -> usize {
        self.tasks.len()
    }

    /// 清理所有任务
    pub fn clear(&mut self) {
        // 取消所有可取消的任务
        for task in self.tasks.values() {
            if task.cancellable {
                task.cancel();
            }
        }
        self.tasks.clear();
    }
}
