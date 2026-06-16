//! 会话层（Layer 2）
//!
//! 管理数据库连接生命周期、查询执行、异步消息分发和 Tab 状态。
//! 依赖 data/ layer，被 state/ 和 ui/ 层使用。

pub mod message;
pub mod tab;

use crate::data::ConnectionManager;
use crate::core::{NotificationManager, ProgressManager};
use crate::data::QueryResult;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;

// ============================================================================
// Session — 数据库会话管理
// ============================================================================

/// 会话状态，聚合所有 DB 连接、查询生命周期和 Tab 状态。
///
/// 这是 Layer 2 的核心结构体，从 `DbManagerApp` 中提取。
/// 未来将由独立的 `poll_messages()` 方法驱动，返回 `FrameEffects`。
pub struct Session {
    // ── 连接管理 ──
    pub manager: ConnectionManager,

    // ── Tab 管理 ──
    pub tab_manager: tab::QueryTabManager,

    // ── 异步基础设施 ──
    pub tx: Sender<message::Message>,
    pub runtime: tokio::runtime::Runtime,

    // ── 执行状态 ──
    pub connecting: bool,
    pub executing: bool,
    pub import_executing: bool,

    // ── 请求 ID 序列 ──
    pub next_connect_request_id: u64,
    pub next_query_request_id: u64,
    pub next_metadata_request_id: u64,

    // ── 请求追踪 ──
    pub pending_connect_requests: HashMap<String, u64>,
    pub pending_database_requests: HashMap<String, (String, u64)>,
    pub pending_triggers_request: Option<(String, Option<String>, u64)>,
    pub pending_routines_request: Option<(String, Option<String>, u64)>,
    pub pending_query_tasks: HashMap<u64, tokio::task::JoinHandle<()>>,
    pub pending_query_connections: HashMap<u64, String>,
    pub pending_query_cancellers: HashMap<u64, Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>>,
    pub user_cancelled_query_requests: HashSet<u64>,

    // ── 通知 ──
    pub notifications: NotificationManager,
    pub progress: ProgressManager,
}

impl Session {
    /// 使用给定的运行时和通道创建新的 Session
    pub fn new(
        runtime: tokio::runtime::Runtime,
        tx: Sender<message::Message>,
    ) -> Self {
        Self {
            manager: ConnectionManager::default(),
            tab_manager: tab::QueryTabManager::new(),
            tx,
            runtime,
            connecting: false,
            executing: false,
            import_executing: false,
            next_connect_request_id: 0,
            next_query_request_id: 0,
            next_metadata_request_id: 0,
            pending_connect_requests: HashMap::new(),
            pending_database_requests: HashMap::new(),
            pending_triggers_request: None,
            pending_routines_request: None,
            pending_query_tasks: HashMap::new(),
            pending_query_connections: HashMap::new(),
            pending_query_cancellers: HashMap::new(),
            user_cancelled_query_requests: HashSet::new(),
            notifications: NotificationManager::default(),
            progress: ProgressManager::default(),
        }
    }

    /// 获取当前活动 Tab 的 SQL
    pub fn active_sql(&self) -> &str {
        self.tab_manager
            .get_active()
            .map(|t| t.sql.as_str())
            .unwrap_or("")
    }

    /// 确保存在一个活动标签，返回可变引用
    pub fn ensure_active_tab(&mut self) -> &mut tab::QueryTab {
        if self.tab_manager.tabs.is_empty() {
            self.tab_manager.new_tab();
        }
        self.tab_manager.get_active_mut().unwrap()
    }

    /// 设置编辑器 SQL（如无 tab 则自动创建）
    pub fn set_active_sql(&mut self, sql: String) {
        self.ensure_active_tab().sql = sql;
    }

    /// 检查活动 Tab 的查询结果
    pub fn active_result(&self) -> Option<&QueryResult> {
        self.tab_manager.get_active().and_then(|t| t.result.as_ref())
    }

    // ── 请求 ID 生成 ──

    fn next_nonzero_request_id(counter: &mut u64) -> u64 {
        *counter = counter.wrapping_add(1);
        if *counter == 0 {
            *counter = 1;
        }
        *counter
    }

    pub fn next_connect_request_id(&mut self) -> u64 {
        Self::next_nonzero_request_id(&mut self.next_connect_request_id)
    }

    pub fn next_metadata_request_id(&mut self) -> u64 {
        Self::next_nonzero_request_id(&mut self.next_metadata_request_id)
    }

    // ── 执行状态刷新 ──

    pub fn refresh_connecting_flag(&mut self) {
        let has_pending = self.manager.active.as_ref().is_some_and(|active_name| {
            self.pending_connect_requests.contains_key(active_name)
                || self.pending_database_requests.contains_key(active_name)
        });
        self.connecting = has_pending;
    }

    pub fn refresh_executing_flag(&mut self) {
        let query_executing = self.tab_manager.tabs.iter().any(|t| t.executing);
        self.executing = self.import_executing || query_executing;
    }

    // ── 查询任务追踪 ──

    pub fn track_query_task(
        &mut self,
        request_id: u64,
        conn_name: String,
        handle: tokio::task::JoinHandle<()>,
        cancel_sender: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
    ) {
        if let Some(prev_handle) = self.pending_query_tasks.insert(request_id, handle) {
            prev_handle.abort();
        }
        self.pending_query_connections.insert(request_id, conn_name);
        self.pending_query_cancellers
            .insert(request_id, cancel_sender);
    }
}
