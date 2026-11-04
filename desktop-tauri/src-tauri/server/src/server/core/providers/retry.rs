//! 可选的失败重试策略；旧插件返回未知方法时沿用原来的退避建议。

use serde::{Deserialize, Serialize};

pub const RETRY_DECISION_METHOD: &str = "retry.decision";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum RetryDecision {
    #[default]
    Default,
    Retry { delay_ms: u64, reason: String },
    Stop { reason: String },
}
