//! Timed future calling a closure on completion.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use pin_project_lite::pin_project;

/// Instrument a future to record its timing.
///
/// The busy and idle time for the future will be passed as an argument to the provided closure.
/// See the documentation for [`Timing`] for more details. In general, it is more straightforward
/// to use the [`TimedFutureExt`] extension trait to attach instrument a future directly.
///
/// # Examples
///
/// ```
/// use future_timed::{timed, Timing};
/// # async fn some_async_fn() -> u64 {
/// #   tokio::time::sleep(std::time::Duration::from_micros(10)).await;
/// #   42
/// # }
/// # fn do_something_with_output(_: u64) {}
/// # #[tokio::main]
/// # async fn main() {
///
/// let output = timed(some_async_fn(), |Timing { idle, busy }| {
///     assert!(!idle.is_zero());
///     assert!(!busy.is_zero());
/// })
/// .await;
///
/// do_something_with_output(output);
/// # }
pub fn timed<Fut, F>(fut: Fut, f: F) -> Timed<Fut, F>
where
    Fut: Future,
    F: FnOnce(Timing),
{
    Timed::new(fut, f)
}

pin_project! {
    /// Future for the [`timed`] function and [`timed`](TimedFutureExt::timed) method.
    pub struct Timed<Fut, F> where Fut: Future, F: FnOnce(Timing) {
        last_poll_end: Option<Instant>,
        timing: Timing,
        op: Option<F>,
        #[pin]
        inner: Fut,
    }
}

impl<Fut, F> Timed<Fut, F>
where
    Fut: Future,
    F: FnOnce(Timing),
{
    pub(crate) fn new(inner: Fut, op: F) -> Self {
        let timing = Timing {
            idle: Duration::ZERO,
            busy: Duration::ZERO,
        };

        Self {
            last_poll_end: None,
            timing,
            op: Some(op),
            inner,
        }
    }
}

impl<Fut, F> Future for Timed<Fut, F>
where
    Fut: Future,
    F: FnOnce(Timing),
{
    type Output = Fut::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let start = Instant::now();
        let mut this = self.project();
        let result = this.inner.as_mut().poll(cx);
        let end = Instant::now();

        if let Some(last_poll_end) = this.last_poll_end.take() {
            this.timing.idle += start - last_poll_end;
        }

        this.timing.busy += end - start;
        *this.last_poll_end = Some(end);

        match result {
            Poll::Pending => Poll::Pending,
            Poll::Ready(output) => {
                if let Some(op) = this.op.take() {
                    op(*this.timing);
                }
                Poll::Ready(output)
            }
        }
    }
}

/// Timing information for an instrumented future.
#[derive(Clone, Copy, Debug, Hash, PartialEq)]
pub struct Timing {
    /// The idle time of a future is the sum of all the time between calls to [`Future::poll`]. The
    /// time before the first poll is not included.
    pub idle: Duration,
    /// The busy time of a future is the sum of all the time consumed during calls to [`Future::poll`]
    /// on that future.
    pub busy: Duration,
}
