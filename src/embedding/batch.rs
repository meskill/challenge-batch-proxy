use batch::{Batchable, GroupBatchable};

use crate::embedding::upstream::Upstream;
use crate::embedding::{EmbedError, EmbedRequest, EmbedUpstreamRequest};
use crate::types::embedding::Embedding;
use crate::types::truncation::TruncationDirection;

#[derive(Clone)]
pub(super) struct EmbeddingBatch {
    upstream: Upstream,
}

#[derive(Clone)]
pub(super) struct EmbeddingGroupBatch {
    upstream: Upstream,
    group_key: EmbeddingGroupKey,
}

// Request options to group by without the input string
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct EmbeddingGroupKey {
    normalize: bool,
    prompt_name: Option<String>,
    truncate: bool,
    truncation_direction: TruncationDirection,
}

// this represent a single input
pub(super) struct BatchInput(String);

impl From<&EmbedRequest> for EmbeddingGroupKey {
    fn from(value: &EmbedRequest) -> Self {
        Self {
            normalize: value.normalize,
            prompt_name: value.prompt_name.clone(),
            truncate: value.truncate,
            truncation_direction: value.truncation_direction,
        }
    }
}

impl EmbeddingBatch {
    pub fn new(upstream: Upstream) -> Self {
        Self { upstream }
    }
}

impl Batchable for EmbeddingGroupBatch {
    type Input = BatchInput;

    type Output = Embedding;

    type Error = EmbedError;

    async fn batch(&self, input: Vec<Self::Input>) -> Result<Vec<Self::Output>, Self::Error> {
        let request = EmbedUpstreamRequest {
            inputs: input.into_iter().map(|x| x.0).collect(),
            normalize: self.group_key.normalize,
            prompt_name: self.group_key.prompt_name.clone(),
            truncate: self.group_key.truncate,
            truncation_direction: self.group_key.truncation_direction,
        };

        let response = self.upstream.embed(&request).await?;

        Ok(response.0)
    }
}

impl GroupBatchable for EmbeddingBatch {
    type GroupKey = EmbeddingGroupKey;

    type Input = EmbedRequest;

    type Batch = EmbeddingGroupBatch;

    fn group_key(&self, input: &Self::Input) -> Self::GroupKey {
        EmbeddingGroupKey::from(input)
    }

    fn group_batcher(&self, group_key: &Self::GroupKey) -> Self::Batch {
        EmbeddingGroupBatch {
            upstream: self.upstream.clone(),
            group_key: group_key.clone(),
        }
    }

    fn input_to_batch_input(&self, input: Self::Input) -> <Self::Batch as batch::Batchable>::Input {
        BatchInput(input.input)
    }
}
