// src/constants.rs

use chrono::{DateTime, Utc, TimeZone};
use once_cell::sync::Lazy;
use std::collections::HashSet;

/// 数字字符集
pub const DIGITS: &str = "0123456789";

/// 大写字母字符集
pub const UPPERCASE_LETTERS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";

/// 小写字母字符集
pub const LOWERCASE_LETTERS: &str = "abcdefghijklmnopqrstuvwxyz";

/// 全部字符集，包含数字、大写字母和小写字母
pub const ALPHABET: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// 默认二进制编码方式
pub const BINARY_ENCODING: &str = "base64";

/// 表示无限等待的时间，使用未来非常远的日期表示（例如 3000 年）
pub static WAIT_INDEFINITELY: Lazy<DateTime<Utc>> = Lazy::new(|| {
    Utc.with_ymd_and_hms(3000, 1, 1, 0, 0, 0).unwrap()
});

/// 日志级别，按照从不输出到详细调试的顺序排列
pub const LOG_LEVELS: [&str; 5] = ["silent", "error", "warn", "info", "debug"];

/// 支持的代码语言
pub const CODE_LANGUAGES: [&str; 2] = ["javaScript", "python"];

/// 代码执行模式：一次对所有数据执行 或 每个数据项分别执行
pub const CODE_EXECUTION_MODES: [&str; 2] = ["runOnceForAllItems", "runOnceForEachItem"];

/// 空凭据的标识，防止误认为空值为有效凭据
pub const CREDENTIAL_EMPTY_VALUE: &str = "__af_EMPTY_VALUE_7b1af746-3729-4c60-9b9b-e08eb29e58da";

/// 表单触发器标识
pub const FORM_TRIGGER_PATH_IDENTIFIER: &str = "af-form";

/// 未知错误提示信息
pub const UNKNOWN_ERROR_MESSAGE: &str = "There was an unknown issue while executing the node";

/// 未知错误描述，包含文档链接帮助排查问题
pub const UNKNOWN_ERROR_DESCRIPTION: &str = "Double-check the node configuration and the service it connects to. Check the error details below and refer to the <a href=\"https://docs.af.io\" target=\"_blank\">af documentation</a> to troubleshoot the issue.";

/// 未知错误凭据提示信息
pub const UNKNOWN_ERROR_MESSAGE_CRED: &str = "UNKNOWN ERROR";

/// af-nodes-base 节点类型常量定义
pub const STICKY_NODE_TYPE: &str = "af-nodes-base.stickyNote";
pub const NO_OP_NODE_TYPE: &str = "af-nodes-base.noOp";
pub const HTTP_REQUEST_NODE_TYPE: &str = "af-nodes-base.httpRequest";
pub const WEBHOOK_NODE_TYPE: &str = "af-nodes-base.webhook";
pub const MANUAL_TRIGGER_NODE_TYPE: &str = "af-nodes-base.manualTrigger";
pub const ERROR_TRIGGER_NODE_TYPE: &str = "af-nodes-base.errorTrigger";
pub const START_NODE_TYPE: &str = "af-nodes-base.start";
pub const EXECUTE_WORKFLOW_NODE_TYPE: &str = "af-nodes-base.executeWorkflow";
pub const EXECUTE_WORKFLOW_TRIGGER_NODE_TYPE: &str = "af-nodes-base.executeWorkflowTrigger";
pub const CODE_NODE_TYPE: &str = "af-nodes-base.code";
pub const FUNCTION_NODE_TYPE: &str = "af-nodes-base.function";
pub const FUNCTION_ITEM_NODE_TYPE: &str = "af-nodes-base.functionItem";
pub const MERGE_NODE_TYPE: &str = "af-nodes-base.merge";
pub const AI_TRANSFORM_NODE_TYPE: &str = "af-nodes-base.aiTransform";
pub const FORM_NODE_TYPE: &str = "af-nodes-base.form";
pub const FORM_TRIGGER_NODE_TYPE: &str = "af-nodes-base.formTrigger";
pub const CHAT_TRIGGER_NODE_TYPE: &str = "@af/af-nodes-langchain.chatTrigger";
pub const WAIT_NODE_TYPE: &str = "af-nodes-base.wait";

/// 工作流启动时的起始节点类型，确定哪个节点优先作为工作流入口
pub const STARTING_NODE_TYPES: [&str; 4] = [
    MANUAL_TRIGGER_NODE_TYPE,
    EXECUTE_WORKFLOW_TRIGGER_NODE_TYPE,
    ERROR_TRIGGER_NODE_TYPE,
    START_NODE_TYPE,
];

/// 脚本节点类型，用于代码执行和数据处理
pub const SCRIPTING_NODE_TYPES: [&str; 4] = [
    FUNCTION_NODE_TYPE,
    FUNCTION_ITEM_NODE_TYPE,
    CODE_NODE_TYPE,
    AI_TRANSFORM_NODE_TYPE,
];

/// 表单通知标识
pub const ADD_FORM_NOTICE: &str = "addFormPage";

/// 指定那些节点其参数中可能引用其他节点，重命名时需要更新
pub static NODES_WITH_RENAMABLE_CONTENT: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    let mut set = HashSet::new();
    set.insert(CODE_NODE_TYPE);
    set.insert(FUNCTION_NODE_TYPE);
    set.insert(FUNCTION_ITEM_NODE_TYPE);
    set.insert(AI_TRANSFORM_NODE_TYPE);
    set
});

/// af-nodes-langchain 节点类型常量
pub const MANUAL_CHAT_TRIGGER_LANGCHAIN_NODE_TYPE: &str = "@af/af-nodes-langchain.manualChatTrigger";
pub const AGENT_LANGCHAIN_NODE_TYPE: &str = "@af/af-nodes-langchain.agent";
pub const CHAIN_LLM_LANGCHAIN_NODE_TYPE: &str = "@af/af-nodes-langchain.chainLlm";
pub const OPENAI_LANGCHAIN_NODE_TYPE: &str = "@af/af-nodes-langchain.openAi";
pub const CHAIN_SUMMARIZATION_LANGCHAIN_NODE_TYPE: &str = "@af/af-nodes-langchain.chainSummarization";
pub const CODE_TOOL_LANGCHAIN_NODE_TYPE: &str = "@af/af-nodes-langchain.toolCode";
pub const WORKFLOW_TOOL_LANGCHAIN_NODE_TYPE: &str = "@af/af-nodes-langchain.toolWorkflow";
pub const HTTP_REQUEST_TOOL_LANGCHAIN_NODE_TYPE: &str = "@af/af-nodes-langchain.toolHttpRequest";

/// 自定义 langchain 工具列表
pub const LANGCHAIN_CUSTOM_TOOLS: [&str; 3] = [
    CODE_TOOL_LANGCHAIN_NODE_TYPE,
    WORKFLOW_TOOL_LANGCHAIN_NODE_TYPE,
    HTTP_REQUEST_TOOL_LANGCHAIN_NODE_TYPE,
];

/// 操作标识
pub const SEND_AND_WAIT_OPERATION: &str = "sendAndWait";
pub const AI_TRANSFORM_CODE_GENERATED_FOR_PROMPT: &str = "codeGeneratedForPrompt";
pub const AI_TRANSFORM_JS_CODE: &str = "jsCode";

/// 数据传输时用于标记被截断的数据项的键
pub const TRIMMED_TASK_DATA_CONNECTIONS_KEY: &str = "__isTrimmedManualExecutionDataItem";

/// OpenAI API 凭据类型
pub const OPEN_AI_API_CREDENTIAL_TYPE: &str = "openAiApi";

/// AI 免费额度错误类型及错误代码
pub const FREE_AI_CREDITS_ERROR_TYPE: &str = "free_ai_credits_request_error";
pub const FREE_AI_CREDITS_USED_ALL_CREDITS_ERROR_CODE: u16 = 400;

/// 标识由 AI 自动生成代码的特殊标记
pub const FROM_AI_AUTO_GENERATED_MARKER: &str = "/*af-auto-generated-fromAI-override*/";