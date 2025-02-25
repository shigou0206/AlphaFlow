use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use serde_json;
use tokio::sync::watch;
use tracing::{error, trace, warn};

use crate::task_queue::TaskQueue;
use store::task_store::TaskStore;
use store::model::{Task, TaskContent, TaskId, TaskState};

use alphaflow_nodes::{
    NodeType, // Trait for node
    NodeExecutionContext,
}; 

/// 假设 handler_id 用 String 表示, e.g. "openai", "http"
/// NodeType trait 里: fn name(&self) -> &str;
pub type NodeTypeId = String;

pub struct TaskDispatcher {
    pub queue: TaskQueue,
    pub store: TaskStore,
    pub timeout: Duration,

    // 改成 NodeType
    handlers: HashMap<NodeTypeId, Arc<dyn NodeType>>,

    notifier: watch::Sender<bool>,
    pub(crate) notifier_rx: Option<watch::Receiver<bool>>,
}

impl TaskDispatcher {
    pub fn new(timeout: Duration) -> Self {
        let (notifier, notifier_rx) = watch::channel(false);
        Self {
            queue: TaskQueue::new(),
            store: TaskStore::new(),
            timeout,
            handlers: HashMap::new(),
            notifier,
            notifier_rx: Some(notifier_rx),
        }
    }

    /// 注册节点（实现了 NodeType）
    ///  typical usage: dispatcher.register_node(OpenAiChatHandler::new())
    pub fn register_node<T>(&mut self, node: T)
    where
        T: NodeType + 'static,
    {
        let node_name = node.name().to_owned();  // e.g. "openai"
        self.handlers.insert(node_name, Arc::new(node));
    }

    pub async fn unregister_node<T: AsRef<str>>(&mut self, node_name: T) {
        if let Some(node) = self.handlers.remove(node_name.as_ref()) {
            trace!("Node {} is unregistered", node.name());
        }
    }

    /// 停止调度器并清理
    pub fn stop(&mut self) {
        let _ = self.notifier.send(true);
        self.queue.clear();
        self.store.clear();
    }

    /// 处理一个待执行任务
    ///  - 从队列pop最优先的pending_task
    ///  - 从 store拿到Task
    ///  - 若Task尚未被cancel/timeout, 则调用 node.execute(context)
    ///  - 根据执行结果更新task state
    ///  - 通过 oneshot ret 发送 task result
    pub async fn process_next_task(&mut self) -> Option<()> {
        // pop from queue
        let pending_task = self.queue.mut_head(|list| list.pop())?;
        let mut task = self.store.remove_task(&pending_task.id)?;
        let ret = task.ret.take()?;

        // 若此task被cancel
        if task.state().is_cancel() {
            let _ = ret.send(task.into());
            self.notify();
            return None;
        }

        // 取出 content
        let content = task.content.take()?;

        // 查找对应 node
        if let Some(node) = self.handlers.get(&task.handler_id) {
            task.set_state(TaskState::Processing);
            trace!("{} task is running, id={}", node.name(), task.id);

            // 构造 NodeExecutionContext
            let ctx = self.build_context(&content);

            // 执行节点
            match tokio::time::timeout(self.timeout, node.execute(&ctx)).await {
                Ok(Ok(_output)) => {
                    // 视情况存储output => store/DB
                    // ...
                    trace!("{} task is done, id={}", node.name(), task.id);
                    task.set_state(TaskState::Done)
                }
                Ok(Err(e)) => {
                    error!("{} task is failed: {:?}", node.name(), e);
                    task.set_state(TaskState::Failure);
                }
                Err(e) => {
                    error!("{} task is timeout: {:?}", node.name(), e);
                    task.set_state(TaskState::Timeout);
                }
            }
        } else {
            // 未找到对应node => Cancel
            trace!("Unknown handler_id: {} => cancel task id={}", task.handler_id, task.id);
            task.set_state(TaskState::Cancel);
        }

        let _ = ret.send(task.into());
        self.notify();
        Some(())
    }

    /// 把 TaskContent 转成 NodeExecutionContext
    /// 例如 if TaskContent::Text(s) => NodeExecutionContext { parameters: from_str(s)? , input_data: Null }
    fn build_context(&self, content: &TaskContent) -> NodeExecutionContext {
        match content {
            TaskContent::Text(s) => NodeExecutionContext {
                parameters: serde_json::json!({ "text": s }),
                input_data: serde_json::Value::Null,
                globals: serde_json::Value::Null,
                env: serde_json::Value::Null,
                pin_data: None,
            },
            TaskContent::Blob(bytes) => {
                // maybe parse as JSON or keep as raw?
                // for example if the node expects raw data
                NodeExecutionContext {
                    parameters: serde_json::json!({ "blob_size": bytes.len() }),
                    input_data: serde_json::Value::Null,
                    globals: serde_json::Value::Null,
                    env: serde_json::Value::Null,
                    pin_data: None,
                }
            },
        }
    }

    pub fn add_task(&mut self, task: Task) {
        debug_assert!(!task.state().is_done());
        if task.state().is_done() {
            warn!("Should not add a task which state is done");
            return;
        }
        trace!("Add task: handler:{}, task:{:?}", task.handler_id, task.content);

        self.queue.push(&task);
        self.store.insert_task(task);
        self.notify();
    }

    pub fn read_task(&self, task_id: &TaskId) -> Option<&Task> {
        self.store.read_task(task_id)
    }

    pub fn cancel_task(&mut self, task_id: TaskId) {
        if let Some(task) = self.store.mut_task(&task_id) {
            task.set_state(TaskState::Cancel);
        }
    }

    pub fn clear_task(&mut self) {
        self.store.clear();
    }

    pub fn next_task_id(&self) -> TaskId {
        self.store.next_task_id()
    }

    pub(crate) fn notify(&self) {
        let _ = self.notifier.send(false);
    }
}