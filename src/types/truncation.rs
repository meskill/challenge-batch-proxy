use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Hash, Default)]
pub enum TruncationDirection {
    Left,
    #[default]
    Right,
}
