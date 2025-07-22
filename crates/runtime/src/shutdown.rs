use std::{
    future::Future,
    io,
    pin::Pin,
    task::{Context, Poll},
};

use futures::FutureExt;
use tokio::signal::unix::{Signal, SignalKind};
use tracing::debug;

/// A `ShutdownSignal` is an helper struct that listens for various shutdown signals sources.
pub struct ShutdownSignal {
    /// A future that resolves when a SIGINT signal is received.
    ctrl_c: Pin<Box<dyn Future<Output = io::Result<()>> + Send>>,
    /// A future that resolves when a SIGTERM signal is received.
    term_signal: Signal,
}

impl std::fmt::Debug for ShutdownSignal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShutdownSignal").finish_non_exhaustive()
    }
}

impl Default for ShutdownSignal {
    fn default() -> Self {
        Self::new()
    }
}

impl ShutdownSignal {
    /// Creates a new `ShutdownSignal` instance.
    pub fn new() -> Self {
        let ctrl_c = Box::pin(tokio::signal::ctrl_c());
        let term_signal = tokio::signal::unix::signal(SignalKind::terminate())
            .expect("failed to install SIGTERM handler");

        Self { ctrl_c, term_signal }
    }
}

impl Future for ShutdownSignal {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        if this.ctrl_c.poll_unpin(cx).is_ready() {
            debug!("Received SIGINT signal");
            return Poll::Ready(());
        }

        if this.term_signal.poll_recv(cx).is_ready() {
            debug!("Received SIGTERM signal");
            return Poll::Ready(());
        }

        Poll::Pending
    }
}

/// Run a future until shutdown signal is received
pub async fn run_until_shutdown<F, O, C>(fut: F, shutdown: ShutdownSignal, on_shutdown: C) -> O
where
    F: Future<Output = O>,
    C: FnOnce(),
{
    tokio::select! {
        // NOTE: wrap with a `Box` so we don't allocate a
        // huge future state machine on the stack.
        result = Box::pin(fut) => result,
        _ = shutdown => {
            on_shutdown();
            std::process::exit(0);
        }
    }
}

/// Run a future until shutdown signal is received with graceful shutdown
pub async fn run_until_shutdown_graceful<F, O, C>(
    fut: F,
    shutdown: ShutdownSignal,
    shutdown_timeout: std::time::Duration,
    on_shutdown: C,
) -> O
where
    F: Future<Output = O>,
    C: FnOnce(),
{
    let mut fut = Box::pin(fut);

    tokio::select! {
        result = &mut fut => result,
        _ = shutdown => {
            on_shutdown();
            debug!("Shutdown signal received, waiting for graceful completion");

            // Give the future a chance to complete gracefully
            tokio::select! {
                result = &mut fut => {
                    debug!("Graceful shutdown completed successfully");
                    result
                },
                _ = tokio::time::sleep(shutdown_timeout) => {
                    tracing::warn!("Graceful shutdown timeout exceeded, forcing exit");
                    std::process::exit(1);
                }
            }
        }
    }
}
