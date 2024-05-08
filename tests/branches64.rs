use core::future::{pending, ready};

use async_select::select;

#[tokio::test]
async fn ready64() {
    let v = select! {
        complete => unreachable!(),
        default => unreachable!(),

        // 64 branches
        r = ready(5) => r,
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),

        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),

        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),

        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),

        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),

        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),

        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
    };
    assert_eq!(v, 5);
}

#[tokio::test]
async fn ready64_complete() {
    let v = select! {
        complete => 5,
        default => 6,

        // 64 branches
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,

        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,

        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,

        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,

        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,

        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,

        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
        Some(i) = ready(None) => i,
    };
    assert_eq!(v, 5);
}

#[tokio::test]
async fn non_blocking64() {
    let v = select! {
        complete => unreachable!(),
        default => 5,

        // 64 branches
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),

        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),

        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),

        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),

        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),

        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),

        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
    };
    assert_eq!(v, 5);
}

#[tokio::test]
async fn nested_blocking64() {
    let v = select! {
        _ = blocking64() => unreachable!(),
        v = ready(5) => v,
    };
    assert_eq!(v, 5);
}

async fn blocking64<T>() -> T {
    select! {
        complete => unreachable!(),

        // 64 branches
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),

        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),

        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),

        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),

        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),

        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),

        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
        _i = pending() => unreachable!(),
    }
}
