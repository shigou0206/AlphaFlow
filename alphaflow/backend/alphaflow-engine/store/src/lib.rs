pub mod model;
pub mod task_store;

#[cfg(test)]
mod tests {
    use super::model::{Task, TaskContent, TaskState, QualityOfService};
    use super::task_store::TaskStore;

    #[test]
    fn test_insert_and_read() {
        let mut store = TaskStore::new();

        // 假设你的 Task 构造函数是这样:
        // pub fn new(handler_id: &str, id: u32, content: TaskContent, qos: QualityOfService) -> Self
        let task = Task::new("dummy_handler", 1, TaskContent::Text("Hello".to_owned()), QualityOfService::Background);
        store.insert_task(task);

        let retrieved = store.read_task(&1);
        assert!(retrieved.is_some(), "Should retrieve a task we just inserted");

        let t = retrieved.unwrap();
        assert_eq!(t.id, 1, "Task id should match");
        assert_eq!(t.state(), &TaskState::Pending, "New task default state is Pending");
        assert!(matches!(t.content, Some(TaskContent::Text(ref txt)) if txt == "Hello"));
    }

    #[test]
    fn test_remove_task() {
        let mut store = TaskStore::new();
        let task = Task::new("dummy_handler", 2, TaskContent::Text("RemoveTest".to_owned()), QualityOfService::Background);
        store.insert_task(task);

        let removed = store.remove_task(&2);
        assert!(removed.is_some(), "Remove an existing task should return Some");
        assert_eq!(removed.unwrap().id, 2);

        assert!(store.read_task(&2).is_none(), "Should no longer find the task after removal");
    }

    #[test]
    fn test_clear_store() {
        let mut store = TaskStore::new();
        let t1 = Task::new("dummy_handler", 10, TaskContent::Text("T1".to_owned()), QualityOfService::Background);
        let t2 = Task::new("dummy_handler", 11, TaskContent::Text("T2".to_owned()), QualityOfService::Background);
        store.insert_task(t1);
        store.insert_task(t2);

        store.clear();
        assert!(store.read_task(&10).is_none(), "After clear, can't read t1");
        assert!(store.read_task(&11).is_none(), "After clear, can't read t2");
    }

    #[test]
    fn test_next_task_id() {
        let store = TaskStore::new();
        let id1 = store.next_task_id();
        let id2 = store.next_task_id();
        assert!(id2 > id1, "Subsequent calls to next_task_id should give ascending ids");
    }
}