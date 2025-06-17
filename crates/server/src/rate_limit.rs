#![allow(unreachable_pub, clippy::redundant_pub_crate)]
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::Response,
};
use tower::{Layer, Service};

use runtime::rate_limiter::RateLimiter;

#[derive(Clone, Debug)]
pub(super) struct RateLimitLayer {
    limiter: RateLimiter,
}

impl RateLimitLayer {
    pub fn new(max: u64, period: Duration) -> Self {
        Self { limiter: RateLimiter::new(max, period) }
    }
}

impl<S> Layer<S> for RateLimitLayer {
    type Service = RateLimit<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimit { inner, limiter: self.limiter.clone() }
    }
}

#[derive(Clone, Debug)]
pub(super) struct RateLimit<S> {
    inner: S,
    limiter: RateLimiter,
}

impl<S, ReqBody> Service<Request<ReqBody>> for RateLimit<S>
where
    S: Service<Request<ReqBody>, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Send + 'static,
{
    type Response = Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        if self.limiter.try_acquire() {
            Box::pin(self.inner.call(req))
        } else {
            let resp = Response::builder()
                .status(StatusCode::TOO_MANY_REQUESTS)
                .body(Body::empty())
                .unwrap();
            Box::pin(std::future::ready(Ok(resp)))
        }
    }
}
