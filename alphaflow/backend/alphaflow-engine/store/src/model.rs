use std::cmp::Ordering;
use tokio::sync::oneshot::{Receiver, Sender};

#[derive(Eq, Debug, Clone, Copy)]
pub enum QualityOfService {
    Background,
    UserInteractive,
}

impl PartialEq for QualityOfService {
    fn eq(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::Background, Self::Background) | (Self::UserInteractive, Self::UserInteractive)
        )
    }
}

/// Compare QoS: user-interactive > background
impl PartialOrd for QualityOfService {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QualityOfService {
    fn cmp(&self, other: &Self) -> Ordering {
        use QualityOfService::*;
        match (self, other) {
            (UserInteractive, UserInteractive) => Ordering::Equal,
            (UserInteractive, Background) => Ordering::Greater,
            (Background, UserInteractive) => Ordering::Less,
            (Background, Background) => Ordering::Equal,
        }
    }
}

pub type TaskId = u32;

/// A simplified struct representing a queued task
#[derive(Eq, Debug, Clone, Copy)]
pub struct PendingTask {
    pub qos: QualityOfService,
    pub id: TaskId,
}

// for BinaryHeap ordering
impl PartialEq for PendingTask {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl PartialOrd for PendingTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for PendingTask {
    fn cmp(&self, other: &Self) -> Ordering {
        // if QoS same, compare ID
        match self.qos.cmp(&other.qos) {
            Ordering::Equal => self.id.cmp(&other.id),
            x => x,
        }
    }
}

#[derive(Debug, Clone)]
pub enum TaskContent {
    Text(String),
    Blob(Vec<u8>),
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum TaskState {
    Pending,
    Processing,
    Done,
    Failure,
    Cancel,
    Timeout,
}

impl TaskState {
    pub fn is_pending(&self) -> bool {
        matches!(self, TaskState::Pending)
    }
    pub fn is_done(&self) -> bool {
        matches!(self, TaskState::Done)
    }
    pub fn is_cancel(&self) -> bool {
        matches!(self, TaskState::Cancel)
    }
    pub fn is_processing(&self) -> bool {
        matches!(self, TaskState::Processing)
    }
    pub fn is_failed(&self) -> bool {
        matches!(self, TaskState::Failure)
    }
}

#[derive(Debug)]
pub struct Task {
    pub id: TaskId,
    pub handler_id: String,
    pub content: Option<TaskContent>,
    pub qos: QualityOfService,
    state: TaskState,
    pub ret: Option<Sender<TaskResult>>,
    pub recv: Option<Receiver<TaskResult>>,
}

/// 自定义 Clone： 复制除 ret/recv 以外的字段，ret/recv 置为 None
impl Clone for Task {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            handler_id: self.handler_id.clone(),
            content: self.content.clone(),
            qos: self.qos,
            state: self.state.clone(),
            ret: None,   // oneshot::Sender 不可克隆
            recv: None,  // oneshot::Receiver 同理
        }
    }
}

impl Task {
    pub fn new(handler_id: &str, id: TaskId, content: TaskContent, qos: QualityOfService) -> Self {
        let handler_id = handler_id.to_owned();
        let (ret, recv) = tokio::sync::oneshot::channel();
        Self {
            handler_id,
            id,
            content: Some(content),
            qos,
            ret: Some(ret),
            recv: Some(recv),
            state: TaskState::Pending,
        }
    }

    pub fn state(&self) -> &TaskState {
        &self.state
    }

    pub fn set_state(&mut self, status: TaskState) {
        self.state = status;
    }

    pub fn is_done(&self) -> bool {
        self.state.is_done()
    }
}

#[derive(Debug)]
pub struct TaskResult {
    pub id: TaskId,
    pub state: TaskState,
}

impl From<Task> for TaskResult {
    fn from(task: Task) -> Self {
        TaskResult {
            id: task.id,
            state: task.state().clone(),
        }
    }
}