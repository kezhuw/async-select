#![no_std]

/// # Select multiplex asynchronous futures simultaneously
///
/// `select!` supports three different clauses:
///
/// * pattern = future [, if condition] => code,
/// * default => code,
/// * complete => code,
///
/// ## Evaluation order
/// * All conditions and futures are evaluated before selection.
/// * Future expression is not evaluated if corresponding condition evaluated to false.
/// * Whenever a branch is ready, its clause is executed. And the whole select returns.
/// * Fail to match a refutable pattern will disable that branch.
/// * `default` clause is executed if no futures are ready. That is non blocking mode.
/// * If all branches are disabled by conditions or refutable pattern match, it resort to
///   `complete` or `default` in case of no `complete`.
///
/// ## Panics
/// * Panic when all futures are disabled or completed and there is no `default` or `complete`.
///
/// ## Comparing with `tokio::select!`
/// * Future expression is only evaluated if condition meets.
///   ```
///   use std::future::ready;
///   use async_select::select;
///
///   async fn guard_future_by_condition() {
///       let opt: Option<i32> = None;
///       let r = select! {
///           v = ready(opt.unwrap()), if opt.is_some() => v,
///           v = ready(6) => v,
///       };
///       assert_eq!(r, 6);
///   }
///   ```
///   This will panic in `tokio::select!` as it evaluates `ready(opt.unwrap())` irrespective of
///   corresponding condition. See <https://github.com/tokio-rs/tokio/pull/6555>.
/// * There is no `default` counterpart in `tokio::select!`. But it could be emulated with `biased`
///   and `ready(())`.`complete` is same as `else` in `tokio::select!`.
/// * `async_select::select!` depends only on `proc_macro` macros and hence the generated code is
///   `no_std` compatible.
///
/// ## Polling order
/// By default, the polling order of each branch is indeterminate. Use `biased;` to poll
/// sequentially if desired.
/// ```
/// use async_select::select;
/// use core::future::{pending, ready};
///
/// async fn poll_sequentially() {
///     let r = select! {
///         biased;
///         default => unreachable!(),
///         _ = pending() => unreachable!(),
///         _ = pending() => unreachable!(),
///         v = ready(5) => v,
///         v = ready(6) => v,
///         v = ready(7) => v,
///     };
///     assert_eq!(r, 5);
/// }
/// ```
///
/// ## Efficiency
/// `select!` blindly `Future:poll` all enabled futures without checking for waking branch.
///
/// ## Examples
/// ```rust
/// use async_select::select;
/// use core::future::{pending, ready};
///
/// async fn on_ready() {
///     let r = select! {
///         _ = pending() => unreachable!(),
///         v = ready(5) => v,
///         default => unreachable!(),
///     };
///     assert_eq!(r, 5);
/// }
/// ```
#[macro_export]
macro_rules! select {
    (biased; $($token:tt)*) => {
        $crate::select_biased! { $($token)* }
    };
    ($($token:tt)*) => {
        $crate::select_default! { $($token)* }
    };
}

// By importing them into this crate and using `$crate::select_xyz`, caller crates
// are free from depending on `async-select-proc-macros` directly.
#[doc(hidden)]
pub use async_select_proc_macros::select_biased;
#[doc(hidden)]
pub use async_select_proc_macros::select_default;
