use std::cmp::Ordering;
use std::collections::hash_map::Entry;
use std::collections::{BinaryHeap, HashMap};
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use atomic_refcell::AtomicRefCell;
use tracing::warn;

// 引入 store 或 handlers 仅在需要时
// 但本文件只管理 Queue, 只需要知道 Task, PendingTask, QoS
use handlers::TaskHandlerId;
use store::model::{PendingTask, Task};

#[derive(Default)]
pub struct TaskQueue {
    // index_tasks for quick access
    index_tasks: HashMap<TaskHandlerId, Arc<AtomicRefCell<TaskList>>>,
    queue: BinaryHeap<Arc<AtomicRefCell<TaskList>>>,
}

impl TaskQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, task: &Task) {
        if task.content.is_none() {
            warn!("The task:{} with empty content will be not executed", task.id);
            return;
        }

        let pending_task = PendingTask {
            qos: task.qos,
            id: task.id,
        };

        match self.index_tasks.entry(task.handler_id.clone()) {
            Entry::Occupied(entry) => {
                let mut list = entry.get().borrow_mut();
                // debug_assert 确认 ID 递增
                debug_assert!(
                    list.peek()
                        .map(|old_id| pending_task.id >= old_id.id)
                        .unwrap_or(true)
                );
                list.push(pending_task);
            }
            Entry::Vacant(entry) => {
                let mut task_list = TaskList::new(entry.key());
                task_list.push(pending_task);
                let task_list = Arc::new(AtomicRefCell::new(task_list));
                entry.insert(task_list.clone());
                self.queue.push(task_list);
            }
        }
    }

    pub fn clear(&mut self) {
        self.queue.clear();
        self.index_tasks.clear();
    }

    /// 弹出堆顶，然后对其进行可变操作 f，操作完如果还不空再push回堆
    pub fn mut_head<T, F>(&mut self, mut f: F) -> Option<T>
    where
        F: FnMut(&mut TaskList) -> Option<T>,
    {
        let head = self.queue.pop()?;
        let result = {
            let mut ref_head = head.borrow_mut();
            f(&mut ref_head)
        };
        if !head.borrow().tasks.is_empty() {
            self.queue.push(head);
        } else {
            self.index_tasks.remove(&head.borrow().id);
        }
        result
    }
}

#[derive(Debug)]
pub struct TaskList {
    pub id: TaskHandlerId,
    tasks: BinaryHeap<PendingTask>,
}

impl TaskList {
    fn new(id: &str) -> Self {
        Self {
            id: id.to_owned(),
            tasks: BinaryHeap::new(),
        }
    }
}

impl Deref for TaskList {
    type Target = BinaryHeap<PendingTask>;

    fn deref(&self) -> &Self::Target {
        &self.tasks
    }
}

impl DerefMut for TaskList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.tasks
    }
}

// 实现比较，让 BinaryHeap 知道如何排序
impl PartialEq for TaskList {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for TaskList {}

impl Ord for TaskList {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.peek(), other.peek()) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Less,
            (Some(_), None) => Ordering::Greater,
            (Some(lhs), Some(rhs)) => lhs.cmp(rhs),
        }
    }
}

impl PartialOrd for TaskList {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}