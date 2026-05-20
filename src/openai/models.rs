use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelList {
    pub object: String,
    pub data: Vec<Model>,
}

impl ModelList {
    pub fn default_models() -> Self {
        Self {
            object: "list".to_string(),
            data: vec![
                Model {
                    id: "gpt-4o".to_string(),
                    object: "model".to_string(),
                    created: 1714067200,
                    owned_by: "kagi".to_string(),
                    permission: vec![],
                    root: "gpt-4o".to_string(),
                    parent: None,
                },
                Model {
                    id: "gpt-4o-mini".to_string(),
                    object: "model".to_string(),
                    created: 1714067200,
                    owned_by: "kagi".to_string(),
                    permission: vec![],
                    root: "gpt-4o-mini".to_string(),
                    parent: None,
                },
                Model {
                    id: "claude-sonnet".to_string(),
                    object: "model".to_string(),
                    created: 1714067200,
                    owned_by: "kagi".to_string(),
                    permission: vec![],
                    root: "claude-sonnet".to_string(),
                    parent: None,
                },
                Model {
                    id: "claude-opus".to_string(),
                    object: "model".to_string(),
                    created: 1714067200,
                    owned_by: "kagi".to_string(),
                    permission: vec![],
                    root: "claude-opus".to_string(),
                    parent: None,
                },
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub id: String,
    pub object: String,
    pub created: u32,
    pub owned_by: String,
    pub permission: Vec<serde_json::Value>,
    pub root: String,
    pub parent: Option<String>,
}