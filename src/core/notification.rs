//! 消息通知系统
//!
//! 提供统一的通知管理，支持多种级别的消息和自动过期。

#![allow(dead_code)] // 公开 API，供外部使用

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// 通知级别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationLevel {
    /// 信息提示（蓝色）
    Info,
    /// 成功消息（绿色）
    Success,
    /// 警告消息（黄色）
    Warning,
    /// 错误消息（红色）
    Error,
}

impl NotificationLevel {
    /// 获取默认显示时长
    pub fn default_duration(&self) -> Duration {
        match self {
            NotificationLevel::Info => Duration::from_secs(3),
            NotificationLevel::Success => Duration::from_secs(3),
            NotificationLevel::Warning => Duration::from_secs(5),
            NotificationLevel::Error => Duration::from_secs(8),
        }
    }

    /// 获取对应的颜色
    pub fn color(&self) -> egui::Color32 {
        match self {
            NotificationLevel::Info => egui::Color32::from_rgb(100, 149, 237),    // 蓝色
            NotificationLevel::Success => egui::Color32::from_rgb(46, 204, 113),  // 绿色
            NotificationLevel::Warning => egui::Color32::from_rgb(241, 196, 15),  // 黄色
            NotificationLevel::Error => egui::Color32::from_rgb(231, 76, 60),     // 红色
        }
    }

    /// 获取图标
    pub fn icon(&self) -> &'static str {
        match self {
            NotificationLevel::Info => "i",
            NotificationLevel::Success => "v",
            NotificationLevel::Warning => "!",
            NotificationLevel::Error => "x",
        }
    }
}

/// 单条通知
#[derive(Debug, Clone)]
pub struct Notification {
    /// 唯一标识符
    pub id: u64,
    /// 通知级别
    pub level: NotificationLevel,
    /// 消息内容
    pub message: String,
    /// 创建时间
    pub created_at: Instant,
    /// 显示时长（超时后自动消失）
    pub duration: Duration,
}

impl Notification {
    /// 创建新通知
    pub fn new(id: u64, level: NotificationLevel, message: impl Into<String>) -> Self {
        let duration = level.default_duration();
        Self {
            id,
            level,
            message: message.into(),
            created_at: Instant::now(),
            duration,
        }
    }

    /// 创建带自定义时长的通知
    pub fn with_duration(
        id: u64,
        level: NotificationLevel,
        message: impl Into<String>,
        duration: Duration,
    ) -> Self {
        Self {
            id,
            level,
            message: message.into(),
            created_at: Instant::now(),
            duration,
        }
    }

    /// 检查通知是否已过期
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() >= self.duration
    }

    /// 获取剩余显示时间比例 (0.0 - 1.0)
    pub fn remaining_ratio(&self) -> f32 {
        let elapsed = self.created_at.elapsed().as_secs_f32();
        let total = self.duration.as_secs_f32();
        (1.0 - elapsed / total).max(0.0)
    }
}

/// 通知管理器
///
/// 管理所有通知的生命周期，支持：
/// - 添加新通知
/// - 手动关闭通知
/// - 自动清理过期通知
/// - 限制最大通知数量
#[derive(Debug)]
pub struct NotificationManager {
    /// 通知队列（新通知在前）
    notifications: VecDeque<Notification>,
    /// 下一个通知 ID
    next_id: u64,
    /// 最大同时显示的通知数量
    max_notifications: usize,
}

impl Default for NotificationManager {
    fn default() -> Self {
        Self::new()
    }
}

impl NotificationManager {
    /// 创建新的通知管理器
    pub fn new() -> Self {
        Self {
            notifications: VecDeque::new(),
            next_id: 1,
            max_notifications: 5,
        }
    }

    /// 设置最大通知数量
    pub fn with_max_notifications(mut self, max: usize) -> Self {
        self.max_notifications = max;
        self
    }

    /// 添加信息通知
    pub fn info(&mut self, message: impl Into<String>) -> u64 {
        self.push(NotificationLevel::Info, message)
    }

    /// 添加成功通知
    pub fn success(&mut self, message: impl Into<String>) -> u64 {
        self.push(NotificationLevel::Success, message)
    }

    /// 添加警告通知
    pub fn warning(&mut self, message: impl Into<String>) -> u64 {
        self.push(NotificationLevel::Warning, message)
    }

    /// 添加错误通知
    pub fn error(&mut self, message: impl Into<String>) -> u64 {
        self.push(NotificationLevel::Error, message)
    }

    /// 添加通知
    pub fn push(&mut self, level: NotificationLevel, message: impl Into<String>) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        let notification = Notification::new(id, level, message);
        self.notifications.push_front(notification);

        // 限制最大数量
        while self.notifications.len() > self.max_notifications {
            self.notifications.pop_back();
        }

        id
    }

    /// 添加带自定义时长的通知
    pub fn push_with_duration(
        &mut self,
        level: NotificationLevel,
        message: impl Into<String>,
        duration: Duration,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        let notification = Notification::with_duration(id, level, message, duration);
        self.notifications.push_front(notification);

        // 限制最大数量
        while self.notifications.len() > self.max_notifications {
            self.notifications.pop_back();
        }

        id
    }

    /// 手动关闭指定通知
    pub fn dismiss(&mut self, id: u64) {
        self.notifications.retain(|n| n.id != id);
    }

    /// 关闭所有通知
    pub fn dismiss_all(&mut self) {
        self.notifications.clear();
    }

    /// 清理过期通知，返回是否有通知被清理
    pub fn tick(&mut self) -> bool {
        let before = self.notifications.len();
        self.notifications.retain(|n| !n.is_expired());
        self.notifications.len() != before
    }

    /// 获取所有活跃通知的迭代器
    pub fn iter(&self) -> impl Iterator<Item = &Notification> {
        self.notifications.iter()
    }

    /// 获取活跃通知数量
    pub fn len(&self) -> usize {
        self.notifications.len()
    }

    /// 检查是否没有通知
    pub fn is_empty(&self) -> bool {
        self.notifications.is_empty()
    }

    /// 获取最新的一条通知（用于状态栏显示）
    pub fn latest(&self) -> Option<&Notification> {
        self.notifications.front()
    }

    /// 获取最新消息文本（兼容旧的 last_message 用法）
    pub fn latest_message(&self) -> Option<&str> {
        self.notifications.front().map(|n| n.message.as_str())
    }
}
