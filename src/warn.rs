//! Timed future calling a closure if polling exceeds a given threshold.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use pin_project_lite::pin_project;

/// Instrument a future call a closure if a certain threshold is exceeded. The closure is called
/// for _each_ poll that exceeds the threshold.
///
/// In general, it is more straightforward to use the [`super::TimedFutureExt`] extension trait to
/// instrument a future directly.
///
/// # Examples
///
/// ```
/// use future_timed::warn_if;
/// use std::time::Duration;
/// # #[tokio::main]
/// # async fn main() {
/// let blocking = async {
///     std::thread::sleep(Duration::from_millis(10));
/// };
///
/// warn_if(blocking, Duration::from_millis(5), |duration| {
///     assert!(duration >= Duration::from_millis(5))
/// })
/// .await;
/// # }
pub fn warn_if<Fut, F>(fut: Fut, threshold: Duration, op: F) -> WarnIf<Fut, F>
where
    Fut: Future,
    F: Fn(Duration),
{
    WarnIf::new(fut, threshold, op)
}

pin_project! {
    /// Future for the [`warn_if`] function and [`warn_if`](TimedFutureExt::warn_if) method.
    pub struct WarnIf<Fut, F> where Fut: Future, F: Fn(Duration) {
        threshold: Duration,
        op: F,
        #[pin]
        inner: Fut,
    }
}

impl<Fut, F> WarnIf<Fut, F>
where
    Fut: Future,
    F: Fn(Duration),
{
    pub(crate) fn new(inner: Fut, threshold: Duration, op: F) -> Self {
        Self {
            threshold,
            op,
            inner,
        }
    }
}

impl<Fut, F> Future for WarnIf<Fut, F>
where
    Fut: Future,
    F: Fn(Duration),
{
    type Output = Fut::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let start = Instant::now();
        let mut this = self.project();
        let result = this.inner.as_mut().poll(cx);
        let end = Instant::now();

        let busy = end - start;

        if busy >= *this.threshold {
            (*this.op)(busy);
        }

        result
    }
}
