// src/waiting_queue.rs
use std::collections::HashMap;
use serde_json::Value;

/// 等待队列：用于收集某个子节点来自多个父节点的数据
#[derive(Debug, Default)]
pub struct WaitingQueue {
    /// 子节点名称 -> (父节点名称 -> 父节点输出)
    pub data: HashMap<String, HashMap<String, Value>>,
}

impl WaitingQueue {
    /// 添加某个父节点输出到指定子节点的等待数据中
    pub fn add(&mut self, child: &str, parent: &str, output: Value) {
        self.data
            .entry(child.to_string())
            .or_default()
            .insert(parent.to_string(), output);
    }

    /// 检查是否已经收集到足够父节点数据
    /// 这里以简单规则：如果至少有2个父节点数据，则认为数据齐全
    pub fn is_ready(&self, child: &str, required: usize) -> bool {
        self.data.get(child).map(|m| m.len() >= required).unwrap_or(false)
    }

    /// 合并多个父节点数据，生成一个 JSON 对象：
    /// { "A": output_from_A, "B": output_from_B, ... }
    pub fn merge(&mut self, child: &str) -> Option<Value> {
        self.data.remove(child).map(|m| serde_json::json!(m))
    }
}