use serde::{Deserialize, Serialize};

use crate::kagi::KagiModel;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelList {
    pub object: String,
    pub data: Vec<Model>,
}

impl ModelList {
    pub fn from_kagi_models(kagi_models: &[KagiModel]) -> Self {
        Self {
            object: "list".to_string(),
            data: kagi_models
                .iter()
                .map(|m| Model {
                    id: m.id.clone(),
                    object: "model".to_string(),
                    created: 1714067200,
                    owned_by: m
                        .provider_label
                        .clone()
                        .unwrap_or_else(|| "kagi".to_string()),
                    permission: vec![],
                    root: m.id.clone(),
                    parent: None,
                })
                .collect(),
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
