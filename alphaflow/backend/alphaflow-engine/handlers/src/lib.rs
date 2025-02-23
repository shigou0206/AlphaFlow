use anyhow::Error;
use async_trait::async_trait;
use store::model::TaskContent;

/// 用 string 表示 handler_id
pub type TaskHandlerId = String;

#[async_trait]
pub trait TaskHandler: Send + Sync + 'static {
    fn handler_id(&self) -> &str;

    fn handler_name(&self) -> &str {
        ""
    }

    async fn run(&self, content: TaskContent) -> Result<(), Error>;
}

// 给 Box<T> & Arc<T> 实现自动转发
#[async_trait]
impl<T> TaskHandler for Box<T>
where
    T: TaskHandler,
{
    fn handler_id(&self) -> &str {
        (**self).handler_id()
    }

    fn handler_name(&self) -> &str {
        (**self).handler_name()
    }

    async fn run(&self, content: TaskContent) -> Result<(), Error> {
        (**self).run(content).await
    }
}

#[async_trait]
impl<T> TaskHandler for std::sync::Arc<T>
where
    T: TaskHandler,
{
    fn handler_id(&self) -> &str {
        (**self).handler_id()
    }

    fn handler_name(&self) -> &str {
        (**self).handler_name()
    }

    async fn run(&self, content: TaskContent) -> Result<(), Error> {
        (**self).run(content).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;  // 只导入 Result，避免和标准 Ok(...) 冲突
    use store::model::TaskContent;

    /// 一个最简单的 `TaskHandler` 实现，用于测试
    struct MockHandler;

    #[async_trait]
    impl TaskHandler for MockHandler {
        fn handler_id(&self) -> &str {
            "mock_handler"
        }

        fn handler_name(&self) -> &str {
            "MockHandler"
        }

        async fn run(&self, content: TaskContent) -> Result<(), Error> {
            println!("MockHandler running, content = {:?}", content);
            Ok(())
        }
    }

    /// 测试 `Box<dyn TaskHandler>` 的自动转发
    #[tokio::test]
    async fn test_box_task_handler() -> Result<()> {
        let handler: Box<dyn TaskHandler> = Box::new(MockHandler);
        assert_eq!(handler.handler_id(), "mock_handler");
        assert_eq!(handler.handler_name(), "MockHandler");

        let content = TaskContent::Text("Box Test".into());
        let result = handler.run(content).await;
        assert!(result.is_ok(), "Handler run should be Ok");
        Ok(())
    }

    /// 测试 `Arc<dyn TaskHandler>` 的自动转发
    #[tokio::test]
    async fn test_arc_task_handler() -> Result<()> {
        let handler: std::sync::Arc<dyn TaskHandler> = std::sync::Arc::new(MockHandler);
        assert_eq!(handler.handler_id(), "mock_handler");
        assert_eq!(handler.handler_name(), "MockHandler");

        let content = TaskContent::Text("Arc Test".into());
        let result = handler.run(content).await;
        assert!(result.is_ok(), "Handler run should be Ok");
        Ok(())
    }
}