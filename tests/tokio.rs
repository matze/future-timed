//! Integration tests running on the tokio runtime.

use future_timed::{timed, warn_if, TimedFutureExt, Timing};
use std::time::Duration;

#[tokio::test]
async fn never_yield() {
    let output = timed(async { 42 }, |Timing { idle, busy }| {
        assert!(idle.is_zero());
        assert!(!busy.is_zero());
    })
    .await;

    assert_eq!(output, 42);
}

#[tokio::test]
async fn short_async_sleep() {
    let output = async {
        tokio::time::sleep(Duration::from_micros(10)).await;
        42
    }
    .timed(|Timing { idle, busy }| {
        assert!(idle > Duration::from_micros(10));
        assert!(!busy.is_zero());
    })
    .await;

    assert_eq!(output, 42);
}

#[tokio::test]
async fn more_busy_time() {
    let output = timed(
        async {
            std::thread::sleep(Duration::from_micros(200));
            tokio::time::sleep(Duration::from_micros(10)).await;
            42
        },
        |Timing { idle, busy }| {
            assert!(idle > Duration::from_micros(10));
            assert!(busy > Duration::from_micros(200));
        },
    )
    .await;

    assert_eq!(output, 42);
}

#[tokio::test]
async fn warn_if_exceeds_threshold() {
    let blocking = async {
        std::thread::sleep(Duration::from_millis(10));
    };

    warn_if(blocking, Duration::from_millis(5), |duration| {
        assert!(duration >= Duration::from_millis(5));
    })
    .await;
}
