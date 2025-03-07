use std::collections::HashMap;
use std::mem;
use std::sync::atomic::{AtomicU32, Ordering::SeqCst};

use crate::model::{Task, TaskId, TaskState};

pub struct TaskStore {
    tasks: HashMap<TaskId, Task>,
    task_id_counter: AtomicU32,
}

impl Clone for TaskStore {
    fn clone(&self) -> Self {
        Self {
            tasks: self.tasks.clone(),
            task_id_counter: AtomicU32::new(self.task_id_counter.load(SeqCst)),
        }
    }
}

impl TaskStore {
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            task_id_counter: AtomicU32::new(1),
        }
    }

    pub fn insert_task(&mut self, task: Task) {
        self.tasks.insert(task.id, task);
    }

    pub fn remove_task(&mut self, task_id: &TaskId) -> Option<Task> {
        self.tasks.remove(task_id)
    }

    pub fn mut_task(&mut self, task_id: &TaskId) -> Option<&mut Task> {
        self.tasks.get_mut(task_id)
    }

    pub fn read_task(&self, task_id: &TaskId) -> Option<&Task> {
        self.tasks.get(task_id)
    }

    pub fn clear(&mut self) {
        let tasks = mem::take(&mut self.tasks);
        for mut task in tasks.into_values() {
            if let Some(ret) = task.ret.take() {
                task.set_state(TaskState::Cancel);
                let _ = ret.send(task.into());
            }
        }
    }

    pub fn next_task_id(&self) -> TaskId {
        self.task_id_counter.fetch_add(1, SeqCst)
    }
}