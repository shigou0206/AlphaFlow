// src/global_state.rs

use once_cell::sync::Lazy;
use std::sync::RwLock;

/// 定义全局状态结构体，包含所有需要全局维护的配置
#[derive(Debug, Clone)]
pub struct GlobalState {
    /// 默认时区，示例中设置为字符串形式
    pub default_timezone: String,
}

/// 全局状态变量，使用 RwLock 来保证线程安全，Lazy 进行懒加载初始化
pub static GLOBAL_STATE: Lazy<RwLock<GlobalState>> = Lazy::new(|| {
    RwLock::new(GlobalState {
        default_timezone: "America/New_York".to_string(),
    })
});

/// 设置全局状态。此操作会覆盖现有状态，必须传入完整的 GlobalState 对象。
///
/// # 示例
/// ```rust
/// use crate::global_state::{set_global_state, GlobalState};
///
/// let new_state = GlobalState { default_timezone: "Europe/London".to_string() };
/// set_global_state(new_state);
/// ```
pub fn set_global_state(state: GlobalState) {
    let mut gs = GLOBAL_STATE.write().expect("GLOBAL_STATE lock poisoned");
    *gs = state;
}

/// 获取全局状态的克隆副本。
/// 返回的是一个新的 GlobalState 对象，确保外部操作不会直接影响内部全局状态。
///
/// # 示例
/// ```rust
/// use crate::global_state::get_global_state;
///
/// let state = get_global_state();
/// println!("当前默认时区: {}", state.default_timezone);
/// ```
pub fn get_global_state() -> GlobalState {
    let gs = GLOBAL_STATE.read().expect("GLOBAL_STATE lock poisoned");
    gs.clone()
}