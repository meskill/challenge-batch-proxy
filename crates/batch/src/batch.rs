use std::marker::PhantomData;
use std::time::Duration;

use tokio::select;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

use crate::traits::Batchable;

#[allow(type_alias_bounds)] // we don't need to enforce and check types here, it's just a type alias
pub(crate) type Message<T: Batchable> = Result<T::Output, T::Error>;

/// Represents uninitialized batcher state
pub struct Uninitialized<T: Batchable> {
    batched: T,
}

/// Represents spawned batcher state that can accept inputs
pub struct Spawned<T: Batchable> {
    tx: mpsc::Sender<(T::Input, oneshot::Sender<Message<T>>)>,
}

/// A batcher that will collect multiple tasks together into batches,
/// send them for execution to [`Batchable`] and place the results
/// back to the callers
pub struct Batcher<T: Batchable, Status = Uninitialized<T>> {
    max_wait_time: Duration,
    max_batch_size: usize,
    status: Status,
    t: PhantomData<T>,
}

// Cloning of the spawned batcher only.
// derive(Clone) forces the [Batched] to be clonable also and manual implementation doesn't require it.
// The cloning should be cheap since we clone primitive and sender only
impl<T: Batchable> Clone for Batcher<T, Spawned<T>> {
    fn clone(&self) -> Self {
        Self {
            max_wait_time: self.max_wait_time,
            max_batch_size: self.max_batch_size,
            status: Spawned {
                tx: self.status.tx.clone(),
            },
            t: PhantomData,
        }
    }
}

impl<T: Batchable> Batcher<T, Uninitialized<T>> {
    pub fn new(max_wait_time: Duration, max_batch_size: usize, batched: T) -> Self {
        Self {
            max_wait_time,
            max_batch_size,
            status: Uninitialized { batched },
            t: PhantomData,
        }
    }
}

impl<T: Batchable> Batcher<T, Uninitialized<T>> {
    /// Spawn the Batcher so it can run tasks
    pub fn spawn(self) -> Batcher<T, Spawned<T>> {
        let (tx, mut rx) = mpsc::channel(self.max_batch_size);

        let max_wait_time = self.max_wait_time;
        let max_batch_size = self.max_batch_size;
        let batched = self.status.batched;

        tokio::spawn(async move {
            let mut inputs = Vec::new();
            let mut notifiers: Vec<oneshot::Sender<Message<T>>> = Vec::new();

            let mut interval = tokio::time::interval(max_wait_time);
            let mut accept_new = true;

            loop {
                select! {
                    _ = interval.tick() => {
                        accept_new = true;

                        if inputs.is_empty() {
                            tracing::trace!("No inputs received, resetting sleep timer");
                            continue;
                        }

                        tracing::debug!("Processing batch of {} inputs", inputs.len());

                        let result = batched.batch(std::mem::take(&mut inputs)).await;

                        match result {
                            Ok(outputs) => {
                                for (output, notifier) in outputs.into_iter().zip(std::mem::take(&mut notifiers)) {
                                    if let Err(_output) = notifier.send(Ok(output)) {
                                        tracing::warn!("Failed to send response, receiver dropped");
                                    }
                                }
                            },
                            Err(error) => {
                                for notifier in std::mem::take(&mut notifiers) {
                                    if let Err(_err) = notifier.send(Err(error.clone())) {
                                        tracing::warn!("Failed to send error response, receiver dropped");
                                    }
                                }
                            },
                        }
                    }
                    job = rx.recv(), if accept_new => {
                        let Some((input, send_response)) = job else {
                            tracing::debug!("Channel closed, exiting the loop");
                            break;
                        };

                        if inputs.is_empty() {
                            // reset interval only if we got input for first time
                            interval.reset();
                        }

                        inputs.push(input);
                        notifiers.push(send_response);

                        if inputs.len() >= max_batch_size {
                            tracing::debug!("Batch size reached, processing batch of {} inputs", inputs.len());
                            // force interval to tick immediately
                            interval.reset_immediately();
                            accept_new = false;
                        }
                    }
                }
            }
        });

        Batcher {
            max_wait_time,
            max_batch_size,
            status: Spawned { tx },
            t: PhantomData,
        }
    }
}

impl<T: Batchable> Batcher<T, Spawned<T>> {
    pub async fn run(&self, input: T::Input) -> Message<T> {
        let (tx, rx) = oneshot::channel();

        self.status
            .tx
            .send((input, tx))
            .await
            .expect("Batcher dropped before finishing the job");

        rx.await.expect("Batcher dropped while waiting for result")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::future::try_join_all;
    use std::ops::DerefMut;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use test_log::test;
    use tokio::time::{sleep, timeout};

    macro_rules! batch_spawn_task {
        ($batcher:expr, $input:expr) => {
            tokio::spawn({
                let batcher = $batcher.clone();
                async move { batcher.run($input).await }
            })
        };
    }

    #[derive(Clone)]
    struct MockBatchProcessor {
        calls: Arc<Mutex<Vec<Vec<String>>>>,
        should_error: bool,
    }

    impl MockBatchProcessor {
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

        fn get_calls(&self) -> Vec<Vec<String>> {
            std::mem::take(self.calls.lock().unwrap().deref_mut())
        }

        async fn process_batch(&self, inputs: Vec<String>) -> Result<Vec<String>, String> {
            tracing::debug!("Processing batch with inputs: {:?}", inputs);
            // Record this call
            self.calls.lock().unwrap().push(inputs.clone());

            if self.should_error {
                tracing::error!("Mock batch processor encountered an error");
                return Err("Mock batch error".to_string());
            }

            tracing::info!("Mock batch processed {} inputs", inputs.len());

            // Transform inputs (just add "processed_" prefix)
            Ok(inputs
                .into_iter()
                .map(|s| format!("processed_{}", s))
                .collect())
        }
    }

    impl Batchable for MockBatchProcessor {
        type Input = String;
        type Output = String;
        type Error = String;

        async fn batch(&self, inputs: Vec<Self::Input>) -> Result<Vec<Self::Output>, Self::Error> {
            self.process_batch(inputs).await
        }
    }

    #[test(tokio::test)]
    async fn test_multiple_parallel_requests_batched_together() {
        let processor = MockBatchProcessor::new();
        let batcher = Batcher::new(Duration::from_millis(10), 3, processor.clone());

        let batcher = batcher.spawn();

        // Start multiple requests in parallel
        let results = try_join_all([
            batch_spawn_task!(batcher, "input_0".to_string()),
            batch_spawn_task!(batcher, "input_1".to_string()),
            batch_spawn_task!(batcher, "input_2".to_string()),
        ])
        .await
        .unwrap();

        // All should succeed
        assert!(results.iter().all(|r| r.is_ok()));

        // Check that exactly one batch was processed with all 3 inputs
        let calls = processor.get_calls();
        assert_eq!(calls.len(), 1, "Expected exactly one batch call");

        let mut batch_inputs = calls.into_iter().next().unwrap();
        assert_eq!(batch_inputs.len(), 3, "Expected batch size of 3");

        // Verify all inputs were processed
        batch_inputs.sort();
        assert_eq!(batch_inputs, vec!["input_0", "input_1", "input_2"]);
    }

    #[test(tokio::test)]
    async fn test_batch_triggered_by_max_size() {
        let processor = MockBatchProcessor::new();
        let batcher = Batcher::new(Duration::from_secs(10), 2, processor.clone());

        let batcher = batcher.spawn();

        // Send exactly 2 requests
        let handle1 = batch_spawn_task!(batcher, "input_1".to_string());
        let handle2 = batch_spawn_task!(batcher, "input_2".to_string());

        // Both should complete quickly (triggered by size, not time)
        let results = timeout(Duration::from_millis(50), async {
            let r1 = handle1.await.unwrap();
            let r2 = handle2.await.unwrap();
            vec![r1, r2]
        })
        .await
        .expect("Requests should complete quickly when batch size is reached");

        assert!(results.iter().all(|r| r.is_ok()));

        // Verify exactly one batch was called with 2 items
        let calls = processor.get_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].len(), 2);
    }

    #[test(tokio::test)]
    async fn test_batch_triggered_by_max_wait_time() {
        let processor = MockBatchProcessor::new();
        let batcher = Batcher::new(Duration::from_millis(10), 10, processor.clone());

        let batcher = batcher.spawn();

        // Send just one request
        let start = tokio::time::Instant::now();
        let result = batch_spawn_task!(batcher, "input_1".to_string()).await;
        let elapsed = start.elapsed();

        assert!(result.is_ok());
        // Should take approximately 10ms (the wait time)
        assert!(elapsed >= Duration::from_millis(9));
        assert!(elapsed <= Duration::from_millis(15));

        // Verify exactly one batch was called with 1 item
        let calls = processor.get_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].len(), 1);
        assert_eq!(calls[0][0], "input_1");
    }

    #[test(tokio::test)]
    async fn test_multiple_batches_when_size_exceeded() {
        let processor = MockBatchProcessor::new();
        let batcher = Batcher::new(
            Duration::from_millis(100),
            2, // Small batch size
            processor.clone(),
        );

        let batcher = batcher.spawn();

        // Send 5 requests - should trigger multiple batches
        let handles: Vec<_> = (0..5)
            .map(|i| batch_spawn_task!(batcher, format!("input_{}", i)))
            .collect();

        let results: Result<Vec<_>, _> = try_join_all(handles).await;
        let results = results.unwrap();

        // All should succeed
        assert!(results.iter().all(|r| r.is_ok()));

        // Should have multiple batch calls
        let calls = processor.get_calls();
        assert!(
            calls.len() >= 2,
            "Should have at least 2 batch calls for 5 inputs with batch size 2"
        );

        // Verify total number of processed items
        let total_processed: usize = calls.iter().map(|batch| batch.len()).sum();
        assert_eq!(total_processed, 5);
    }

    #[test(tokio::test)]
    async fn test_error_propagation() {
        let processor = MockBatchProcessor::new_with_error();
        let batcher = Batcher::new(Duration::from_millis(50), 5, processor.clone());

        let batcher = batcher.spawn();

        // Start multiple requests in parallel
        let results = try_join_all([
            batch_spawn_task!(batcher, "input_0".to_string()),
            batch_spawn_task!(batcher, "input_1".to_string()),
            batch_spawn_task!(batcher, "input_2".to_string()),
        ])
        .await
        .unwrap();

        for result in results {
            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), "Mock batch error");
        }

        // Verify batch was called once
        assert_eq!(processor.get_calls().len(), 1);
    }

    #[test(tokio::test)]
    async fn test_concurrent_batches_are_independent() {
        let processor = MockBatchProcessor::new();
        let batcher = Batcher::new(
            Duration::from_millis(5), // Short timeout to ensure batches complete separately
            2,                        // Small batch size
            processor.clone(),
        );

        let batcher = batcher.spawn();

        // Send first batch of 2 requests
        let first_batch_handles: Vec<_> = (0..2)
            .map(|i| batch_spawn_task!(batcher, format!("first_{}", i)))
            .collect();

        // Wait a bit to ensure first batch is processed
        sleep(Duration::from_millis(10)).await;

        // Send second batch of 2 requests
        let second_batch_handles: Vec<_> = (0..2)
            .map(|i| batch_spawn_task!(batcher, format!("second_{}", i)))
            .collect();

        for handle in first_batch_handles.into_iter().chain(second_batch_handles) {
            assert!(handle.await.unwrap().is_ok());
        }

        // Should have exactly 2 batch calls
        let calls = processor.get_calls();
        assert_eq!(calls.len(), 2, "Should have exactly 2 separate batches");

        // Each batch should have 2 items
        assert_eq!(calls[0].len(), 2);
        assert_eq!(calls[1].len(), 2);

        // Verify content of batches
        let first_batch = &calls[0];
        let second_batch = &calls[1];

        // First batch should contain "first_" prefixed items
        assert!(first_batch.iter().all(|s| s.starts_with("first_")));
        // Second batch should contain "second_" prefixed items
        assert!(second_batch.iter().all(|s| s.starts_with("second_")));
    }

    #[test(tokio::test)]
    async fn test_wait_between_calls() {
        let processor = MockBatchProcessor::new();
        let batcher = Batcher::new(
            Duration::from_millis(5), // Short timeout to ensure batches complete separately
            2,                        // Small batch size
            processor.clone(),
        );

        let batcher = batcher.spawn();

        let mut tasks = Vec::new();

        tasks.push(batch_spawn_task!(batcher, "first_0".to_string()));

        sleep(Duration::from_millis(2)).await;

        tasks.push(batch_spawn_task!(batcher, "first_1".to_string()));

        sleep(Duration::from_millis(5)).await;

        tasks.push(batch_spawn_task!(batcher, "second_0".to_string()));

        sleep(Duration::from_millis(2)).await;

        tasks.push(batch_spawn_task!(batcher, "second_1".to_string()));

        let results = try_join_all(tasks).await.unwrap();

        // All should succeed
        assert!(results.iter().all(|r| r.is_ok()));

        // Should have exactly 2 batch calls
        let calls = processor.get_calls();
        assert_eq!(calls.len(), 2, "Should have exactly 2 separate batches");

        let mut calls_iter = calls.into_iter();

        let calls_0 = calls_iter.next().unwrap();
        let calls_1 = calls_iter.next().unwrap();

        // Each batch should have 2 items
        assert_eq!(calls_0.len(), 2);
        assert_eq!(calls_1.len(), 2);

        assert_eq!(calls_0, vec!["first_0", "first_1"]);
        assert_eq!(calls_1, vec!["second_0", "second_1"]);
    }
}
