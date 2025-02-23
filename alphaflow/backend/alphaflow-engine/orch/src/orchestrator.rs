use queue::task_queue::TaskQueue;
use store::task_store::TaskStore;
use store::model::{Task, TaskContent, QualityOfService};

// 一个简单的“占位”编排器，用来示例如何把队列 & 存储组合起来
pub struct Orchestrator {
    pub queue: TaskQueue,
    pub store: TaskStore,
}

impl Orchestrator {
    // 需要从外部传入一个已经初始化的 queue & store
    pub fn new(queue: TaskQueue, store: TaskStore) -> Self {
        Self { queue, store }
    }

    /// 一个示例方法：把一个新任务同时插入 store、放入 queue
    /// 并返回该任务 ID
    pub fn add_task(
        &mut self,
        handler_id: &str,
        content: TaskContent,
        qos: QualityOfService,
    ) -> u32 {
        let new_id = self.store.next_task_id();
        let task = Task::new(handler_id, new_id, content, qos);
        self.store.insert_task(task.clone());
        self.queue.push(&task);
        new_id
    }

    /// 占位方法：演示如何“编排”队列中的任务
    /// 这里只是简单弹出 pending_task ID & 打印显示
    pub fn process_one(&mut self) -> Option<u32> {
        // 从队列里拿到最优先 pending_task
        let popped = self.queue.mut_head(|list| list.pop())?;
        let task_id = popped.id;

        // 这里可以在 store 里查到对应 Task，执行一些逻辑
        if let Some(_task) = self.store.remove_task(&task_id) {
            println!("Orchestrator: processing task ID = {task_id}");
            // 在更复杂场景可调用 Handler 或其他模块
        }
        Some(task_id)
    }

    // 一个简单的占位: 你想要做更高级编排时，可写更多逻辑
    pub fn run_all(&mut self) {
        // 不断 process_one() 直到没有任务
        while self.process_one().is_some() {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use store::model::TaskContent;

    #[test]
    fn test_orchestrator_add_and_process() {
        let queue = TaskQueue::new();
        let store = TaskStore::new();
        let mut orch = Orchestrator::new(queue, store);

        // add 一个任务
        let tid = orch.add_task("dummy_handler", TaskContent::Text("hello".into()), QualityOfService::Background);
        assert_eq!(tid, 1);

        // process_one() 会弹出任务并从 store 里移除
        let processed_id = orch.process_one();
        assert_eq!(processed_id, Some(1));

        // 再次 process 时 queue 已空 -> None
        let processed_id2 = orch.process_one();
        assert!(processed_id2.is_none(), "No more tasks left");
    }

    #[test]
    fn test_orchestrator_run_all() {
        let queue = TaskQueue::new();
        let store = TaskStore::new();
        let mut orch = Orchestrator::new(queue, store);

        orch.add_task("handler1", TaskContent::Text("task1".into()), QualityOfService::Background);
        orch.add_task("handler2", TaskContent::Text("task2".into()), QualityOfService::Background);

        // 一次性跑完
        orch.run_all();

        // 这里再 process_one() 已经没有任务了
        let p = orch.process_one();
        assert!(p.is_none());
    }
}