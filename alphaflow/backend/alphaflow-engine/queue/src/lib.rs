pub mod task_queue;
pub mod task_dispatcher;
pub mod task_runner;

#[cfg(test)]
mod tests {
    use super::task_queue::TaskQueue; // 显示引入 TaskQueue
    use store::model::{Task, TaskContent, QualityOfService}; // 引入 Task/TaskContent/QoS

    #[test]
    fn test_push_and_mut_head() {
        let mut queue = TaskQueue::new();

        // 构造一个任务
        let task = Task::new(
            "my_handler",
            1,
            TaskContent::Text("hello queue".into()),
            QualityOfService::Background,
        );
        queue.push(&task);

        // 测试能否从队列弹出
        let popped_pending = queue.mut_head(|list| list.pop());
        assert!(popped_pending.is_some(), "Should pop one PendingTask");
        let pending_task = popped_pending.unwrap();
        assert_eq!(pending_task.id, 1, "PendingTask id should match 1");

        // 你也可以对 pending_task.qos 作进一步断言:
        // assert_eq!(pending_task.qos, QualityOfService::Background);
    }

    #[test]
    fn test_clear_queue() {
        let mut queue = TaskQueue::new();

        let t1 = Task::new(
            "handler",
            10,
            TaskContent::Text("A".into()),
            QualityOfService::Background
        );
        let t2 = Task::new(
            "handler",
            11,
            TaskContent::Text("B".into()),
            QualityOfService::Background
        );

        queue.push(&t1);
        queue.push(&t2);

        queue.clear();

        // 之后 queue 里没有任何元素
        // mut_head 应该返回 None
        let popped = queue.mut_head(|list| list.pop());
        assert!(popped.is_none(), "After clear, queue is empty");
    }
}