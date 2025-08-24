//! Future timing instrumentation.
//!
//! Provides instrumentation to record the time taken by a future. This includes the busy time and
//! the idle time. The busy time of a future is the sum of all the time consumed during calls to
//! [`Future::poll`] on that future. On the other hand, The idle time of a future is the sum of all
//! the time between calls to [`Future::poll`]. The time before the first poll is not included.
//!
//! # Usage
//!
//! Add `future-timed` to your Cargo.toml `dependencies`:
//!
//! ```toml
//! future-timed = "0.1"
//! ```
//!
//! Use the [`TimedFutureExt`] extension trait to add an instrumentation closure
//! on a future:
//!
//! ```
//! # use future_timed::{TimedFutureExt, Timing};
//! # use std::time::Duration;
//! # async fn some_async_fn() -> u64 {
//! #   tokio::time::sleep(Duration::from_micros(10)).await;
//! #   42
//! # }
//! # fn do_something_with_output(_: u64) {}
//! # #[tokio::main]
//! # async fn main() {
//!     let output = some_async_fn()
//!         .timed(|Timing { idle, busy }| {
//!             assert!(!idle.is_zero());
//!             assert!(!busy.is_zero());
//!         })
//!         .await;
//!
//!     do_something_with_output(output);
//! # }
//! ```
//!
//! # Comparison with similar crates
//!
//! This work is based almost entirely on the [future-timing] crate but sports a different API.
//! While future-timing requires destructuring the future's output into the timing data and the
//! futures output itself, future-timed allows to report inline and compose the output with
//! subsequent future combinators, for example from the [futures] crate:
//!
//! ```
//! # use future_timed::{TimedFutureExt, Timing};
//! # use futures::future::FutureExt;
//! # use std::time::Duration;
//! # #[tokio::main]
//! # async fn main() {
//! let output = async {
//!         tokio::time::sleep(Duration::from_micros(10)).await;
//!         21
//!     }
//!     .timed(|Timing { busy, .. }| {
//!         println!("busy for {busy:?}");
//!     })
//!     .map(|n| 2 * n)
//!     .await;
//!
//! assert_eq!(output, 42);
//! # }
//! ```
//!
//! Note that in that case you measure the combined time for all wrapped futures.
//!
//! # License
//!
//! This project is licensed under the [MIT license].
//!
//! [MIT license]: https://github.com/matze/future-timed/blob/main/LICENSE
//! [future-timing]: https://docs.rs/future-timing/latest/future_timing/
//! [futures]: https://docs.rs/futures/latest/futures/index.html

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
    fn new(inner: Fut, op: F) -> Self {
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

/// An extension trait for `Future`s that adds the [`timed`] method.
pub trait TimedFutureExt: Future {
    /// Instrument a future to record its timing
    ///
    /// The busy and idle time for the future will be passed as an argument to the provided
    /// closure. See the documentation for [`Timing`] for more details.
    ///
    /// # Examples
    ///
    /// ```
    /// use future_timed::{TimedFutureExt, Timing};
    /// # use std::time::Duration;
    /// # #[tokio::main]
    /// # async fn main() {
    ///
    /// let output = async {
    ///         // Block the executor
    ///         std::thread::sleep(Duration::from_micros(200));
    ///         tokio::time::sleep(Duration::from_micros(10)).await;
    ///     42
    ///     }.timed(|Timing { idle, busy }| {
    ///         assert!(idle > Duration::from_micros(10));
    ///         assert!(busy > Duration::from_micros(200));
    ///     })
    ///     .await;
    ///
    /// assert_eq!(output, 42);
    /// # }
    fn timed<F>(self, f: F) -> Timed<Self, F>
    where
        Self: Sized,
        F: FnOnce(Timing),
    {
        Timed::new(self, f)
    }
}

impl<T: Future> TimedFutureExt for T {}

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
