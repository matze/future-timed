//! Future timing instrumentation.
//!
//! Provides instrumentation to record the time taken by a future or warn if polling exceeds a
//! threshold. The recorded time includes the busy time and the idle time. The busy time of a
//! future is the sum of all the time consumed during calls to [`Future::poll`] on that future. On
//! the other hand, The idle time of a future is the sum of all the time between calls to
//! [`Future::poll`]. The time before the first poll is not included.
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

mod timed;
mod warn;

pub use timed::{timed, Timed, Timing};
pub use warn::{warn_if, WarnIf};

/// An extension trait for `Future`s that adds the [`timed`] method.
pub trait TimedFutureExt: Future {
    /// Instrument a future to record its timing.
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

    /// Instrument a future call a closure if a certain threshold is exceeded. The closure is
    /// called for _each_ poll that exceeds the threshold.
    ///
    /// # Examples
    ///
    /// ```
    /// use future_timed::TimedFutureExt;
    /// # use std::time::Duration;
    /// # #[tokio::main]
    /// # async fn main() {
    ///
    /// let output = async {
    ///     // Block the executor
    ///     std::thread::sleep(Duration::from_micros(200));
    ///     42
    /// }
    /// .warn_if(Duration::from_micros(10), |duration| {
    ///     assert!(duration >= Duration::from_micros(200));
    /// })
    /// .await;
    ///
    /// assert_eq!(output, 42);
    /// # }
    fn warn_if<F>(self, threshold: std::time::Duration, f: F) -> WarnIf<Self, F>
    where
        Self: Sized,
        F: Fn(std::time::Duration),
    {
        WarnIf::new(self, threshold, f)
    }
}

impl<T: Future> TimedFutureExt for T {}
