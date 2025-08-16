use serde::{Deserialize, Serialize};

// TODO: encode static sizes for embedding based on model if needed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding(Vec<f32>);
