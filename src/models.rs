use serde::{Deserialize};
use serde_json::Value;

#[derive(Deserialize)]
pub struct MjmlInput {
    pub mjml: Option<String>,
    pub payload: Value,
    pub template: Option<String>,
}
