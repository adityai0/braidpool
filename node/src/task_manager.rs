use std::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time::{timeout, Duration};
use tracing::{info, span, warn, Level};

/// Graceful shutdown timeout before forcing abort
pub const GRACEFUL_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

pub struct TaskManager {
    tasks: Mutex<Vec<JoinHandle<()>>>,
}

impl Default for TaskManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            tasks: Mutex::new(Vec::new()),
        }
    }

    pub fn spawn<F>(&self, fut: F)
    where
        F: std::future::Future<Output = ()> + Send + 'static,
        F::Output: Send + 'static,
    {
        use tracing::Instrument;
        let location = std::panic::Location::caller();
        let span = span!(
            Level::DEBUG,
            "task",
            file = location.file(),
            line = location.line(),
            column = location.column(),
        );
        let handle = tokio::spawn(fut.instrument(span));
        self.tasks.lock().unwrap().push(handle);
    }

    /// Wait for all tasks to complete gracefully within timeout.
    /// Returns true if all tasks completed gracefully, false if timeout occurred.
    pub async fn shutdown_graceful(&self, timeout_duration: Duration) -> bool {
        let handles = {
            let mut tasks = self.tasks.lock().unwrap();
            std::mem::take(&mut *tasks)
        };

        let join_future = async {
            for handle in handles {
                let _ = handle.await;
            }
        };

        match timeout(timeout_duration, join_future).await {
            Ok(_) => {
                info!(component = "task_manager", "All tasks completed gracefully");
                true
            }
            Err(_) => {
                warn!(
                    component = "task_manager",
                    "Graceful shutdown timeout exceeded"
                );
                false
            }
        }
    }

    pub async fn join_all(&self) {
        let handles = {
            let mut tasks = self.tasks.lock().unwrap();
            std::mem::take(&mut *tasks)
        };

        for handle in handles {
            let _ = handle.await;
        }
    }

    pub async fn abort_all(&self) {
        let mut tasks = self.tasks.lock().unwrap();
        for handle in tasks.drain(..) {
            handle.abort();
        }
    }
}
