use serde::{Deserialize, Serialize};

use crate::types::embedding::Embedding;
use crate::types::truncation::TruncationDirection;

fn default_normalize() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EmbedRequest {
    pub input: String,
    #[serde(default = "default_normalize")]
    pub normalize: bool,
    pub prompt_name: Option<String>,
    #[serde(default)]
    pub truncate: bool,
    #[serde(default)]
    pub truncation_direction: TruncationDirection,
}

impl EmbedRequest {
    pub fn new(input: impl Into<String>) -> Self {
        Self {
            input: input.into(),
            normalize: default_normalize(),
            prompt_name: None,
            truncate: false,
            truncation_direction: TruncationDirection::default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, derive_more::From)]
pub struct EmbedResponse(Embedding);
