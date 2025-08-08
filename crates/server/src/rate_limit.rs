#![allow(unreachable_pub, clippy::redundant_pub_crate)]
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use api_types::ErrorResponse;
use axum::{
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
};
use tower::{Layer, Service};

use runtime::rate_limiter::RateLimiter;

#[derive(Clone, Debug)]
pub(super) struct RateLimitLayer {
    limiter: RateLimiter,
    period: Duration,
}

impl RateLimitLayer {
    pub fn new(max: u64, period: Duration) -> Self {
        Self { limiter: RateLimiter::new(max, period), period }
    }
}

impl<S> Layer<S> for RateLimitLayer {
    type Service = RateLimit<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimit { inner, limiter: self.limiter.clone(), period: self.period }
    }
}

#[derive(Clone, Debug)]
pub(super) struct RateLimit<S> {
    inner: S,
    limiter: RateLimiter,
    period: Duration,
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
            let error_body = ErrorResponse::new(
                "rate-limit",
                "Too Many Requests",
                StatusCode::TOO_MANY_REQUESTS,
                format!("Rate limit exceeded. Retry after {} seconds", self.period.as_secs()),
            );
            let mut resp = axum::Json(error_body).into_response();
            *resp.status_mut() = StatusCode::TOO_MANY_REQUESTS;
            resp.headers_mut().insert(
                axum::http::header::RETRY_AFTER,
                axum::http::HeaderValue::from_str(&self.period.as_secs().to_string()).unwrap(),
            );
            Box::pin(std::future::ready(Ok(resp)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RateLimitLayer;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        response::Response,
    };
    use std::{convert::Infallible, time::Duration};
    use tower::{Layer, Service, ServiceExt, service_fn};

    #[tokio::test]
    async fn sets_retry_after_header() {
        let layer = RateLimitLayer::new(1, Duration::from_secs(30));
        let inner = service_fn(|_req: Request<Body>| async move {
            Ok::<_, Infallible>(Response::new(Body::empty()))
        });
        let mut svc = layer.layer(inner);

        let _ = svc.ready().await.unwrap().call(Request::new(Body::empty())).await.unwrap();
        let resp = svc.ready().await.unwrap().call(Request::new(Body::empty())).await.unwrap();

        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
        let retry = resp.headers().get(axum::http::header::RETRY_AFTER).unwrap();
        assert_eq!(retry.to_str().unwrap(), "30");
    }
}
