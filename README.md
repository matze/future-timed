# future-timed

[![Crates.io](https://img.shields.io/crates/v/future-timed.svg)](https://crates.io/crates/future-timed)
[![Documentation](https://docs.rs/future-timed/badge.svg)](https://docs.rs/future-timed)

Future timing instrumentation for Rust async code.

## Overview

`future-timed` provides instrumentation to record the time taken by a future. It
tracks the **busy time** which is the sum of all time consumed during calls to
`Future::poll` on the future and the **idle time** which is The sum of all time
between calls to `Future::poll` (excluding time before first poll)

## Installation

Add `future-timed` to your `Cargo.toml`:

```toml
future-timed = "0.1"
```

## Usage

Use the `TimedFutureExt` extension trait to add timing instrumentation to any future:

```rust
use future_timed::{TimedFutureExt, Timing};

async fn main() {
    let output = some_async_fn()
        .timed(|Timing { idle, busy }| {
            println!("Future was idle for {:?} and busy for {:?}", idle, busy);
        })
        .await;

    // Use the output as normal
}
```

You can also use the standalone `timed` function:

```rust
use future_timed::{timed, Timing};

async fn main() {
    let output = timed(some_async_fn(), |Timing { idle, busy }| {
        println!("Future was idle for {:?} and busy for {:?}", idle, busy);
    }).await;
}
```

## Composability

Unlike similar crates, `future-timed` allows you to report timing data inline
and compose with subsequent future combinators:

```rust
use future_timed::{TimedFutureExt, Timing};
use futures::future::FutureExt;

async fn main() {
    let output = async {
        // Some async operation
        21
    }
    .timed(|Timing { busy, .. }| {
        println!("busy for {busy:?}");
    })
    .map(|n| 2 * n)
    .await;

    assert_eq!(output, 42);
}
```

Note that in this case, timing data is for all wrapped futures.

## License

This project is licensed under the [MIT
license](https://github.com/matze/future-timed/blob/main/LICENSE).

## Credits

Based on the
[future-timing](https://docs.rs/future-timing/latest/future_timing/) crate but
with a different API.
