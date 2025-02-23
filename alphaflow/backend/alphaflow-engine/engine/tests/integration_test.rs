// tests/integration_test.rs

use alphaflow_engine::task_store::TaskStore;
use alphaflow_nodes::NodeRegistry;
use alphaflow_nodes::register_all_nodes;
use alphaflow_engine::model::{Task, TaskContent, QualityOfService};
use alphaflow_engine::task_dispatcher::TaskDispatcher;
use std::time::Duration;
use dotenv::dotenv;
use std::env;

#[tokio::test]


async fn integration_test_engine() {
    // 尝试加载 .env 文件（如需）
    dotenv().ok();

    // 尝试检查 OPENAI_API_KEY，如果没有则仅测试 HTTP 节点，跳过 OpenAI 测试
    let openai_key = env::var("OPENAI_API_KEY").ok(); // Some(key) 或 None

    // 1. 初始化节点注册表，并注册所有节点
    let mut registry = NodeRegistry::new();
    register_all_nodes(&mut registry);
    println!("Registered nodes: {:?}", registry.list_nodes());

    // 2. 初始化引擎组件
    let store = TaskStore::new();
    let timeout = Duration::from_secs(10);
    let mut dispatcher = TaskDispatcher::new(timeout);

    // ===================
    // Part A: 测试 HTTP 节点
    // ===================
    {
        println!("--- Testing HTTP Node integration ---");
        let task_id_http = store.next_task_id();
        // 构造一个简单的 HTTP 任务
        let http_task = Task::new(
            "http", // handler_id = "http" 节点
            task_id_http,
            TaskContent::Text("https://httpbin.org/get?test=integration".into()),
            QualityOfService::Background,
        );

        dispatcher.add_task(http_task);
        let processed = dispatcher.process_next_task().await;
        assert!(processed.is_some(), "Expected at least one HTTP task processed");

        let processed_task = store.read_task(&task_id_http);
        assert!(processed_task.is_none(), "HTTP Task should be removed from store after processing");
        println!("HTTP node test completed!");
    }

    // ===================
    // Part B: 测试 OpenAI 节点 (若存在 OPENAI_API_KEY)
    // ===================
    if let Some(api_key) = openai_key {
        println!("--- Testing OpenAI Node integration ---");
        let task_id_openai = store.next_task_id();
        // 构造一个 OpenAI 任务
        //   假设 openai 节点需要在 TaskContent 里携带 prompt 等信息。
        //   这里为了简化，先只放文本（OpenAiNode 内部若只读取 Text 即可）。
        //   如果你的节点解析 JSON 参数，可把 JSON 串写进 Text，或扩展TaskContent。
        let openai_task = Task::new(
            "openai", // handler_id = "openai"
            task_id_openai,
            TaskContent::Text("Hello from integration test with OpenAI!".into()),
            QualityOfService::Background,
        );

        dispatcher.add_task(openai_task);
        let processed = dispatcher.process_next_task().await;
        assert!(processed.is_some(), "Expected at least one OpenAI task processed");

        // 检查是否被移除
        let processed_task = store.read_task(&task_id_openai);
        assert!(
            processed_task.is_none(),
            "OpenAI Task should be removed after processing"
        );

        println!("OpenAI node test completed!");
    } else {
        eprintln!("Skipping OpenAI test because OPENAI_API_KEY not set.");
    }

    println!("Integration test completed successfully.");
}