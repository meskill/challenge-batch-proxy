use serde::{Deserialize, Serialize};

use crate::types::embedding::Embedding;
use crate::types::truncation::TruncationDirection;

fn default_normalize() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize, derive_more::From)]
#[serde(untagged)]
pub enum OneOrMany<T> {
    #[from]
    One(T),
    Many(Vec<T>),
}

impl<T> From<Vec<T>> for OneOrMany<T> {
    fn from(value: Vec<T>) -> Self {
        if value.len() == 1 {
            OneOrMany::One(value.into_iter().next().unwrap())
        } else {
            OneOrMany::Many(value)
        }
    }
}

impl<T> OneOrMany<T> {
    pub fn len(&self) -> usize {
        match self {
            OneOrMany::One(_) => 1,
            OneOrMany::Many(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn into_vec(self) -> Vec<T> {
        match self {
            OneOrMany::One(s) => vec![s],
            OneOrMany::Many(v) => v,
        }
    }
}

pub type EmbedInput = OneOrMany<String>;

#[derive(Debug, Serialize, Deserialize)]
pub struct EmbedRequest {
    pub input: EmbedInput,
    #[serde(default = "default_normalize")]
    pub normalize: bool,
    pub prompt_name: Option<String>,
    #[serde(default)]
    pub truncate: bool,
    #[serde(default)]
    pub truncation_direction: TruncationDirection,
}

impl From<&str> for EmbedRequest {
    fn from(input: &str) -> Self {
        Self {
            input: EmbedInput::from(input.to_owned()),
            normalize: default_normalize(),
            prompt_name: None,
            truncate: false,
            truncation_direction: TruncationDirection::default(),
        }
    }
}

impl From<Vec<&str>> for EmbedRequest {
    fn from(input: Vec<&str>) -> Self {
        Self {
            input: EmbedInput::from(input.into_iter().map(|s| s.to_owned()).collect::<Vec<_>>()),
            normalize: default_normalize(),
            prompt_name: None,
            truncate: false,
            truncation_direction: TruncationDirection::default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, derive_more::From)]
pub struct EmbedResponse(OneOrMany<Embedding>);
