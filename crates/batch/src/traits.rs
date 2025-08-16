use std::hash::Hash;

/// Wrapper for batchable entities that could resolve multiple tasks in single batch
pub trait Batchable: Send + 'static {
    /// Represents a single task input
    type Input: Send + 'static;
    /// Represents a single task output
    type Output: Send + 'static;
    /// Represents possible errors when processing a batch
    type Error: Clone + Send + 'static;

    /// Execute multiple tasks as single batch
    fn batch(
        &self,
        input: Vec<Self::Input>,
    ) -> impl Future<Output = Result<Vec<Self::Output>, Self::Error>> + Send;
}

/// Wrapper for batchable entities that should be grouped to be batched properly.
/// Built around [`Batchable`] for single group.
pub trait GroupBatchable {
    /// Key to group batchable tasks together
    type GroupKey: Send + 'static + Hash + Eq + Clone;
    /// Represents a single task input before grouping
    type Input: Send + 'static;
    /// Represents [`Batchable`] entity for the group
    type Batch: Batchable;

    /// Get the group key from the whole input.
    /// The batches will be grouped together based on this key.
    /// The inputs that could be batched together should return the same value for group_key,
    /// and which shouldn't be batched together should have different keys
    fn group_key(&self, input: &Self::Input) -> Self::GroupKey;

    /// Create a new batcher for the given group
    fn group_batcher(&self, group_key: &Self::GroupKey) -> Self::Batch;

    /// Converts the usual input to input that is used in grouping batching.
    /// Usually, helpful to optimize storage of input data since all of the input
    /// shares the same group key that contains parts of the input itself
    fn input_to_batch_input(&self, input: Self::Input) -> <Self::Batch as Batchable>::Input;
}
