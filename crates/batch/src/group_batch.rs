use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;

use crate::{Batcher, GroupBatchable, Message, Spawned};

#[derive(Clone)]
pub struct GroupBatcher<T: GroupBatchable> {
    max_wait_time: Duration,
    max_batch_size: usize,
    batched: T,
    #[allow(clippy::type_complexity)]
    groups: Arc<DashMap<T::GroupKey, Batcher<T::Batch, Spawned<T::Batch>>>>,
}

impl<T: GroupBatchable> GroupBatcher<T> {
    pub fn new(max_wait_time: Duration, max_batch_size: usize, batched: T) -> Self {
        Self {
            max_wait_time,
            max_batch_size,
            batched,
            groups: Arc::new(DashMap::new()),
        }
    }

    pub async fn run(&self, input: T::Input) -> Message<T::Batch> {
        let group_key = self.batched.group_key(&input);

        // NOTE: the clone here is not quite necessarily to implement this kind of logic
        // but interfaces of concurrent maps are quite tricky to bypass this.
        // clone for simplicity
        let group = self
            .groups
            .entry(group_key.clone())
            .or_insert_with(|| {
                let batcher = Batcher::new(
                    self.max_wait_time,
                    self.max_batch_size,
                    self.batched.group_batcher(&group_key),
                );

                batcher.spawn()
            })
            .clone();

        let input = self.batched.input_to_batch_input(input);

        group.run(input).await
    }
}

#[cfg(test)]
mod tests {
    use crate::Batchable;

    use super::*;
    use futures::future::try_join_all;
    use std::ops::DerefMut;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use test_log::test;
    use tokio::time::timeout;

    macro_rules! batch_spawn_task {
        ($batcher:expr, $input:expr) => {
            tokio::spawn({
                let batcher = $batcher.clone();
                async move { batcher.run($input).await }
            })
        };
    }

    #[derive(Clone)]
    struct MockGroupProcessor {
        #[allow(clippy::type_complexity)]
        calls: Arc<Mutex<Vec<(String, Vec<String>)>>>, // (group_key, batch)
        should_error: bool,
    }

    impl MockGroupProcessor {
        fn new() -> Self {
            Self {
                calls: Arc::new(Mutex::new(Vec::new())),
                should_error: false,
            }
        }

        fn new_with_error() -> Self {
            Self {
                calls: Arc::new(Mutex::new(Vec::new())),
                should_error: true,
            }
        }

        fn get_calls(&self) -> Vec<(String, Vec<String>)> {
            std::mem::take(self.calls.lock().unwrap().deref_mut())
        }

        async fn process_batch(
            &self,
            group_key: String,
            inputs: Vec<String>,
        ) -> Result<Vec<String>, String> {
            tracing::debug!(
                "Processing batch for group '{}' with inputs: {:?}",
                group_key,
                inputs
            );

            // Record this call
            self.calls
                .lock()
                .unwrap()
                .push((group_key.clone(), inputs.clone()));

            if self.should_error {
                tracing::error!("Mock group batch processor encountered an error");
                return Err("Mock group batch error".to_string());
            }

            tracing::info!(
                "Mock group batch processed {} inputs for group '{}'",
                inputs.len(),
                group_key
            );

            // Transform inputs (add group prefix and "processed_" suffix)
            Ok(inputs
                .into_iter()
                .map(|s| format!("{}:processed_{}", group_key, s))
                .collect())
        }
    }

    #[derive(Clone)]
    struct MockBatchProcessorForGroup {
        group_key: String,
        processor: MockGroupProcessor,
    }

    impl Batchable for MockBatchProcessorForGroup {
        type Input = String;
        type Output = String;
        type Error = String;

        async fn batch(&self, inputs: Vec<Self::Input>) -> Result<Vec<Self::Output>, Self::Error> {
            self.processor
                .process_batch(self.group_key.clone(), inputs)
                .await
        }
    }

    // Test input that contains both group information and data
    #[derive(Debug, Clone)]
    struct GroupedInput {
        group: String,
        data: String,
    }

    impl GroupBatchable for MockGroupProcessor {
        type GroupKey = String;
        type Input = GroupedInput;
        type Batch = MockBatchProcessorForGroup;

        fn group_key(&self, input: &Self::Input) -> Self::GroupKey {
            input.group.clone()
        }

        fn group_batcher(&self, group_key: &Self::GroupKey) -> Self::Batch {
            MockBatchProcessorForGroup {
                group_key: group_key.clone(),
                processor: self.clone(),
            }
        }

        fn input_to_batch_input(&self, input: Self::Input) -> <Self::Batch as Batchable>::Input {
            input.data
        }
    }

    #[test(tokio::test)]
    async fn test_different_groups_are_batched_separately() {
        let processor = MockGroupProcessor::new();
        let batcher = GroupBatcher::new(Duration::from_millis(10), 5, processor.clone());

        // Send inputs to different groups
        let results = try_join_all([
            batch_spawn_task!(
                batcher,
                GroupedInput {
                    group: "group_a".to_string(),
                    data: "input_1".to_string(),
                }
            ),
            batch_spawn_task!(
                batcher,
                GroupedInput {
                    group: "group_b".to_string(),
                    data: "input_2".to_string(),
                }
            ),
            batch_spawn_task!(
                batcher,
                GroupedInput {
                    group: "group_a".to_string(),
                    data: "input_3".to_string(),
                }
            ),
            batch_spawn_task!(
                batcher,
                GroupedInput {
                    group: "group_b".to_string(),
                    data: "input_4".to_string(),
                }
            ),
        ])
        .await
        .unwrap();

        // All should succeed
        assert!(results.iter().all(|r| r.is_ok()));

        // Should have exactly 2 batch calls (one for each group)
        let mut calls = processor.get_calls();

        calls.sort_by(|a, b| a.0.cmp(&b.0));

        assert_eq!(
            calls.len(),
            2,
            "Expected exactly 2 batch calls for 2 groups"
        );

        let mut calls_iter = calls.into_iter();
        // Verify group A calls
        let mut group_a_inputs = calls_iter.next().unwrap().1;
        group_a_inputs.sort();
        assert_eq!(group_a_inputs, vec!["input_1", "input_3"]);

        // Verify group B calls
        let mut group_b_inputs = calls_iter.next().unwrap().1;
        group_b_inputs.sort();
        assert_eq!(group_b_inputs, vec!["input_2", "input_4"]);
    }

    #[test(tokio::test)]
    async fn test_same_group_inputs_are_batched_together() {
        let processor = MockGroupProcessor::new();
        let batcher = GroupBatcher::new(Duration::from_millis(10), 5, processor.clone());

        // Send multiple inputs to the same group
        let results = try_join_all([
            batch_spawn_task!(
                batcher,
                GroupedInput {
                    group: "same_group".to_string(),
                    data: "input_1".to_string(),
                }
            ),
            batch_spawn_task!(
                batcher,
                GroupedInput {
                    group: "same_group".to_string(),
                    data: "input_2".to_string(),
                }
            ),
            batch_spawn_task!(
                batcher,
                GroupedInput {
                    group: "same_group".to_string(),
                    data: "input_3".to_string(),
                }
            ),
        ])
        .await
        .unwrap();

        // All should succeed
        assert!(results.iter().all(|r| r.is_ok()));

        // Should have exactly 1 batch call
        let calls = processor.get_calls();
        assert_eq!(
            calls.len(),
            1,
            "Expected exactly 1 batch call for same group"
        );

        // Verify the batch contains all inputs
        let mut group_inputs = calls.into_iter().next().unwrap().1;
        assert_eq!(group_inputs.len(), 3);
        group_inputs.sort();
        assert_eq!(group_inputs, vec!["input_1", "input_2", "input_3"]);
    }

    #[test(tokio::test)]
    async fn test_group_batch_triggered_by_max_size() {
        let processor = MockGroupProcessor::new();
        let batcher = GroupBatcher::new(
            Duration::from_secs(10), // Very long wait time
            2,                       // Small max batch size
            processor.clone(),
        );

        // Send exactly 2 requests to the same group
        let handle1 = batch_spawn_task!(
            batcher,
            GroupedInput {
                group: "test_group".to_string(),
                data: "input_1".to_string()
            }
        );

        let handle2 = batch_spawn_task!(
            batcher,
            GroupedInput {
                group: "test_group".to_string(),
                data: "input_2".to_string()
            }
        );

        // Both should complete quickly (triggered by size, not time)
        let results = timeout(Duration::from_millis(100), async {
            let r1 = handle1.await.unwrap();
            let r2 = handle2.await.unwrap();
            (r1, r2)
        })
        .await
        .expect("Requests should complete quickly when batch size is reached");

        assert!(results.0.is_ok());
        assert!(results.1.is_ok());

        // Verify exactly one batch was called with 2 items
        let calls = processor.get_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].1.len(), 2);
    }

    #[test(tokio::test)]
    async fn test_group_batch_triggered_by_max_wait_time() {
        let processor = MockGroupProcessor::new();
        let batcher = GroupBatcher::new(
            Duration::from_millis(10), // Short wait time
            10,                        // Large batch size that won't be reached
            processor.clone(),
        );

        // Send just one request
        let start = tokio::time::Instant::now();
        let result = batcher
            .run(GroupedInput {
                group: "wait_group".to_string(),
                data: "input_1".to_string(),
            })
            .await;
        let elapsed = start.elapsed();

        assert!(result.is_ok());
        // Should take approximately 100ms (the wait time)
        assert!(elapsed >= Duration::from_millis(9));
        assert!(elapsed <= Duration::from_millis(15));

        // Verify exactly one batch was called with 1 item
        let calls = processor.get_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].1.len(), 1);
        assert_eq!(calls[0].1[0], "input_1");
    }

    #[test(tokio::test)]
    async fn test_concurrent_groups_are_processed_independently() {
        let processor = MockGroupProcessor::new();
        let batcher = GroupBatcher::new(
            Duration::from_millis(10),
            2, // Small batch size
            processor.clone(),
        );

        // Send concurrent requests to different groups
        let handles: Vec<_> = (0..6)
            .map(|i| {
                let group = format!("group_{}", i % 3); // 3 different groups
                batch_spawn_task!(
                    batcher,
                    GroupedInput {
                        group,
                        data: format!("input_{}", i)
                    }
                )
            })
            .collect();

        let results = try_join_all(handles).await.unwrap();

        // All should succeed
        assert!(results.iter().all(|r| r.is_ok()));

        // Should have multiple batch calls for different groups
        let calls = processor.get_calls();
        assert!(
            calls.len() >= 3,
            "Should have at least 3 batch calls for 3 groups"
        );

        // Verify each group processed its inputs
        for group_calls in calls {
            assert_eq!(
                group_calls.1.len(),
                2,
                "Each group should process exactly 2 inputs"
            );
        }
    }

    #[test(tokio::test)]
    async fn test_group_error_propagation() {
        let processor = MockGroupProcessor::new_with_error();
        let batcher = GroupBatcher::new(Duration::from_millis(10), 5, processor.clone());

        // Send requests to different groups
        let results = try_join_all([
            batch_spawn_task!(
                batcher,
                GroupedInput {
                    group: "error_group_a".to_string(),
                    data: "input_1".to_string(),
                }
            ),
            batch_spawn_task!(
                batcher,
                GroupedInput {
                    group: "error_group_b".to_string(),
                    data: "input_2".to_string(),
                }
            ),
        ])
        .await
        .unwrap();

        // All should fail with the error
        for result in results {
            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), "Mock group batch error");
        }

        // Verify batches were called
        assert_eq!(processor.get_calls().len(), 2);
    }
}
