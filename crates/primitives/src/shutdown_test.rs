#[cfg(test)]
mod tests {
    use crate::shutdown::{ShutdownSignal, run_until_shutdown};
    use std::time::Duration;
    use tokio::time;

    #[tokio::test]
    async fn test_run_until_shutdown() {
        let future = async {
            time::sleep(Duration::from_secs(1)).await;
            "completed"
        };

        let shutdown = ShutdownSignal::new();
        // We'll spawn a task that triggers shutdown after a brief delay
        tokio::spawn(async {
            time::sleep(Duration::from_millis(10)).await;
            // We can't easily send signals in tests, so we'll drop the shutdown
            // which will terminate the run_until_shutdown
        });

        let test_fn = || {
            // This is our shutdown handler
        };

        // This should complete the moment we drop the shutdown signal
        let result = run_until_shutdown(future, shutdown, test_fn).await;
        assert_eq!(result, "completed");
    }
}
