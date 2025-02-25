use derivative::Derivative;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Derivative)]
#[serde(untagged)]
#[derivative(Hash)]
pub enum InputMapping {
    Single(String),
    Multi {
        fields: BTreeMap<String, String>,
        #[serde(default)]
        #[derivative(Hash = "ignore")]
        defaultValue: Option<Value>,
    },
}