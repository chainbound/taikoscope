use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

/// A simple fixed window rate limiter.
#[derive(Clone, Debug)]
pub struct RateLimiter {
    state: Arc<Mutex<LimiterState>>,
    capacity: u64,
    period: Duration,
}

#[derive(Debug)]
struct LimiterState {
    count: u64,
    reset_at: Instant,
}

impl RateLimiter {
    /// Create a new [`RateLimiter`] with the given `capacity` and `period`.
    pub fn new(capacity: u64, period: Duration) -> Self {
        Self {
            state: Arc::new(Mutex::new(LimiterState {
                count: 0,
                reset_at: Instant::now() + period,
            })),
            capacity,
            period,
        }
    }

    /// Attempt to acquire a permit.
    pub fn try_acquire(&self) -> bool {
        let mut state = self.state.lock().expect("lock poisoned");
        let now = Instant::now();
        if now >= state.reset_at {
            state.reset_at = now + self.period;
            state.count = 1;
            true
        } else if state.count < self.capacity {
            state.count += 1;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RateLimiter;
    use std::{
        sync::{
            Arc,
            atomic::{AtomicU64, Ordering},
        },
        time::Duration,
    };
    use tokio::time::sleep;

    #[tokio::test]
    async fn denies_when_over_capacity() {
        let limiter = RateLimiter::new(2, Duration::from_millis(50));
        assert!(limiter.try_acquire());
        assert!(limiter.try_acquire());
        assert!(!limiter.try_acquire());
    }

    #[tokio::test]
    async fn resets_after_period() {
        let limiter = RateLimiter::new(1, Duration::from_millis(10));
        assert!(limiter.try_acquire());
        assert!(!limiter.try_acquire());
        sleep(Duration::from_millis(15)).await;
        assert!(limiter.try_acquire());
    }

    #[tokio::test]
    async fn concurrency_respects_capacity() {
        let limiter = Arc::new(RateLimiter::new(5, Duration::from_secs(1)));
        let success = Arc::new(AtomicU64::new(0));
        let mut handles = Vec::new();
        for _ in 0..10 {
            let l = Arc::clone(&limiter);
            let s = Arc::clone(&success);
            handles.push(tokio::spawn(async move {
                if l.try_acquire() {
                    s.fetch_add(1, Ordering::SeqCst);
                }
            }));
        }
        for handle in handles {
            handle.await.unwrap();
        }
        assert_eq!(success.load(Ordering::SeqCst), 5);
    }
}
