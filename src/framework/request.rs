use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
pub struct PageRequest {
    pub event: String,
    #[serde(default)]
    pub model: Value,
    #[serde(default)]
    pub params: Value,
}
