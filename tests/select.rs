use std::future::{pending, ready};
use std::time::Duration;

use async_select::select;
use tokio::time::sleep;

#[tokio::test]
async fn expression_type() -> Result<(), std::io::Error> {
    select! {
        _ = ready(5) => Ok(()),
        default => Ok(()),
    }
}

#[tokio::test]
async fn biased_all_ready() {
    let r = select! {
        biased;
        v = ready(1) => v,
        v = ready(2) => v,
        v = ready(3) => v,
        v = ready(4) => v,
    };
    assert_eq!(r, 1);
}

#[tokio::test]
async fn biased_partial_ready() {
    let r = select! {
        biased;
        v = pending() => v,
        v = pending() => v,
        v = ready(3) => v,
        v = ready(4) => v,
    };
    assert_eq!(r, 3);
}

#[tokio::test]
async fn biased_no_ready_default() {
    let r = select! {
        biased;
        v = pending() => v,
        v = pending() => v,
        v = pending() => v,
        v = pending() => v,
        default => 3,
    };
    assert_eq!(r, 3);
}

#[tokio::test]
async fn biased_no_ready() {
    let r = select! {
        biased;
        v = pending() => v,
        v = pending() => v,
        v = pending() => v,
        v = pending() => v,
        _ = sleep(Duration::from_millis(5)) => 3,
        complete => 5,
    };
    assert_eq!(r, 3);
}

#[tokio::test]
async fn ready_default() {
    let r = select! {
        v = ready(5) => v,
        default => 6,
    };
    assert_eq!(r, 5);
}

#[tokio::test]
async fn ready_complete() {
    let r = select! {
        v = ready(5) => v,
        complete => 7,
    };
    assert_eq!(r, 5);
}

#[tokio::test]
async fn ready_complete_with_default() {
    let r = select! {
        v = ready(5) => v,
        default => 6,
        complete => 7,
    };
    assert_eq!(r, 5);
}

#[tokio::test]
async fn not_ready_default() {
    let r = select! {
        v = pending() => v,
        default => 6,
    };
    assert_eq!(r, 6);
}

#[tokio::test]
async fn not_ready_complete() {
    let r = select! {
        v = pending() => v,
        _ = sleep(Duration::from_millis(5)) => 6,
        complete => 7,
    };
    assert_eq!(r, 6);
}

#[tokio::test]
async fn not_ready_complete_with_default() {
    let r = select! {
        v = pending() => v,
        default => 6,
        complete => 7,
    };
    assert_eq!(r, 6);
}

fn none() -> Option<i32> {
    None
}

#[tokio::test]
#[should_panic(expected = "all branches are disabled or completed")]
async fn all_disabled_panic() {
    let opt: Option<i32> = none();
    select! {
        v = ready(opt.unwrap()), if opt.is_some() => v,
    };
}

#[tokio::test]
async fn all_disabled_default() {
    let opt: Option<i32> = none();
    let r = select! {
        v = ready(opt.unwrap()), if opt.is_some() => v,
        default => 6,
    };
    assert_eq!(r, 6);
}

#[tokio::test]
async fn all_disabled_complete() {
    let opt: Option<i32> = none();
    let r = select! {
        v = ready(opt.unwrap()), if opt.is_some() => v,
        complete => 7,
    };
    assert_eq!(r, 7);
}

#[tokio::test]
async fn all_disabled_complete_with_default() {
    let opt: Option<i32> = none();
    let r = select! {
        v = ready(opt.unwrap()), if opt.is_some() => v,
        default => 6,
        complete => 7,
    };
    assert_eq!(r, 7);
}

#[tokio::test]
#[should_panic(expected = "all branches are disabled or completed")]
async fn all_completed_panic() {
    select! {
        Some(5) = ready(None) => {},
    }
}

#[tokio::test]
async fn all_completed_default() {
    let r = select! {
        Some(v) = ready(None) => v,
        default => 7,
    };
    assert_eq!(r, 7);
}

#[tokio::test]
async fn all_completed_complete() {
    let r = select! {
        Some(v) = ready(None) => v,
        complete => 7,
    };
    assert_eq!(r, 7);
}

#[tokio::test]
async fn all_completed_complete_with_default() {
    let r = select! {
        Some(v) = ready(None) => v,
        default => 6,
        complete => 7,
    };
    assert_eq!(r, 7);
}
