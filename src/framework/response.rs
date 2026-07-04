use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
pub struct PatchEntry {
    pub id: String,
    pub html: String,
}

#[derive(Serialize)]
pub struct PageResponse {
    pub patches: Vec<PatchEntry>,
    pub model: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect: Option<String>,
}
