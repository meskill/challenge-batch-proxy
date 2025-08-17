use serde::{Deserialize, Serialize};

use crate::types::embedding::Embedding;
use crate::types::truncation::TruncationDirection;

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct EmbedRequest {
    pub input: String,
    #[serde(default = "default_true")]
    pub normalize: bool,
    pub prompt_name: Option<String>,
    #[serde(default)]
    pub truncate: bool,
    #[serde(default)]
    pub truncation_direction: TruncationDirection,
}

#[derive(Debug, Serialize, derive_more::From)]
pub struct EmbedResponse(Embedding);
