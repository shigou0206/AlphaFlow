use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::interval;

use std::sync::Arc;

use crate::task_dispatcher::TaskDispatcher;

pub struct TaskRunner();

impl TaskRunner {
    pub async fn run(dispatcher: Arc<RwLock<TaskDispatcher>>) {
        // 让 dispatcher 先发一次通知
        dispatcher.read().await.notify();

        let debounce_duration = Duration::from_millis(300);
        let mut notifier = dispatcher
            .write()
            .await
            .notifier_rx
            .take()
            .expect("Only take once");

        loop {
            // stops the runner if the notifier was closed.
            if notifier.changed().await.is_err() {
                break;
            }

            // stops the runner if the value is `true`
            if *notifier.borrow() {
                break;
            }

            let mut itv = interval(debounce_duration);
            itv.tick().await;

            let _ = dispatcher.write().await.process_next_task().await;
        }
    }
}