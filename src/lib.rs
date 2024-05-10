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
/// ## Limitations
/// * Support up to 64 branches.
/// * Refutability check may cause false negative compilation error. As results are matched against
///   `&mut output` but not value which will be fed to matching clause.
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
///   corresponding condition.
/// * There is no `default` counterpart in `tokio::select!`. But it could be emulated with `biased`
///   and `ready(())`.`complete` is same as `else` in `tokio::select!`.
/// * `tokio::select!` strips the pattern using in refutability through `proc_marco`. This avoid
///   false negative compilation error.
/// * `async_select::select!` is dependency free and hence `no_std` compatible.
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
    ($($tokens:tt)*) => {
        $crate::select_internal!($($tokens)*)
    };

    // For documentation purpose.
    (default => $body:expr) => {};
    (complete => $body:expr) => {};
    ($pattern:pat = $future:expr $(, if $condition:expr)? => $body:expr) => {};
}

#[doc(hidden)]
#[macro_export]
macro_rules! select_internal {
    // @list($tokens:tt $count:tt $branches:tt $default:tt $complete:tt) list branches
    // @poll all futures
    //
    // @count($index:tt) count index
    // @assign($index, $futures:ident, $future:expr) assign optional future to indexed tuple field
    // @access($index, $futures:ident) access future from indexed tuple field
    // @unwrap($index, $pattern:pat) pattern match output base on index
    // @wrap($index, $output) wrap output based on index for pattern match
    //
    //
    // As you may saw this macro is pretty long, most complexity comes from:
    // * Macros are not permitted to use everywhere. Say, you can't iterate repetition inside "<_>",
    //   so I have to write generic output types for all combination up to 64 branches. This takes 2500
    //   lines.
    // * Support up to 64 branches and avoid macro expansion `recursion_limit` so I have to exhaust
    //   possible match in one round.
    // * The match is not greedy, so I can't use `$(,)? $($token:tt)*` but exhaustion as `,` is also a
    //   valid token.
    // * There is no way to expand at most once with else clause. So I have to match and normalize
    //   separately. This is `$(, if $condition:expr)?`.
    // * Macro output must be itself a valid syntex tree. Say, I can't generate `if $condition {` and
    //   `}` separately based on at most once repetion.
    // * There is no way to capture matched literals but handwritten. I can't use `$(ref)? $(mut)?` to
    //   capture them and use it later. So, I have to exhauste them.
    // * Given above, I have to exhuaste combinations of `$(ref)?`, `$(mut)?`, `$(, if $condition:expr)?`
    //   and `$(,)?`.
    //
    // I might be totally wrong in above. Please point them out in issue/pr.

    (@list
        ()
        $count:tt
        ()
        $default:tt
        $complete:tt
    ) => {
        $crate::select_internal!(@no-branch {
            default: $default,
            complete: $complete,
        })
    };
    (@list
        ()
        $count:tt
        $branches:tt
        ()
        ()
    ) => {
        $crate::select_internal!(
            @poll
            non_blocking = false,
            when_completed = false,
            count = $count,
            branches = $branches,
            default = unreachable!("not in unblocking mode"),
            complete = panic!("all branches are disabled or completed and there is no `default` nor `complete`"))
    };
    (@list
        ()
        $count:tt
        $branches:tt
        ()
        ($complete:tt)
    ) => {
        $crate::select_internal!(
            @poll
            non_blocking = false,
            when_completed = true,
            count = $count,
            branches = $branches,
            default = unreachable!("not in unblocking mode"),
            complete = $complete)
    };
    (@list
        ()
        $count:tt
        $branches:tt
        ($default:tt)
        ()
    ) => {
        $crate::select_internal!(
            @poll
            non_blocking = true,
            when_completed = false,
            count = $count,
            branches = $branches,
            default = $default,
            complete = panic!("all branches are disabled or completed and there is no `default` nor `complete`"))
    };
    (@list
        ()
        $count:tt
        $branches:tt
        ($default:tt)
        ($complete:tt)
    ) => {
        $crate::select_internal!(
            @poll
            non_blocking = true,
            when_completed = true,
            count = $count,
            branches = $branches,
            default = $default,
            complete = $complete)
    };

    // `complete` in last case.
    (@list
        (complete => $body:expr $(,)?)
        $count:tt
        $branches:tt
        $default:tt
        ($complete:tt)
    ) => {
        compile_error!("`select!`: more than one `complete` clauses")
    };
    (@list
        (complete => $body:expr, $($token:tt)*)
        $count:tt
        $branches:tt
        $default:tt
        ($complete:tt)
    ) => {
        compile_error!("`select!`: more than one `complete` clauses")
    };
    (@list
        (complete => $body:expr)
        $count:tt
        $branches:tt
        $default:tt
        ()
    ) => {
        $crate::select_internal!(
            @list
            ()
            $count
            $branches:tt
            $default
            ($body)
         )
    };
    // `complete` in no last case.
    (@list
        (complete => $body:block, $($tokens:tt)*)
        $count:tt
        $branches:tt
        $default:tt
        ()
    ) => {
        $crate::select_internal!(
            @list
            ($($tokens)*)
            $count
            $branches
            $default
            ($body)
        )
    };
    (@list
        (complete => $body:block $($tokens:tt)*)
        $count:tt
        $branches:tt
        $default:tt
        ()
    ) => {
        $crate::select_internal!(
            @list
            ($($tokens)*)
            $count
            $branches
            $default
            ($body)
        )
    };
    (@list
        (complete => $body:expr, $($tokens:tt)*)
        $count:tt
        $branches:tt
        $default:tt
        ()
    ) => {
        $crate::select_internal!(
            @list
            ($($tokens)*)
            $count
            $branches
            $default
            ($body)
        )
    };
    (@list
        (complete => $($tokens:tt)*)
        $count:tt
        $branches:tt
        $default:tt
        $complete:tt
    ) => {
        $crate::select_internal!(@missing-comma "complete clause");
    };

    // multiple `default` clauses
    (@list
        (default => $body:expr $(,)?)
        $count:tt
        $branches:tt
        ($default:tt)
        $complete:tt
    ) => {
        compile_error!("`select!`: more than one `default` clauses")
    };
    (@list
        (default => $body:expr, $($token:tt)*)
        $count:tt
        $branches:tt
        ($default:tt)
        $complete:tt
    ) => {
        compile_error!("`select!`: more than one `default` clauses")
    };
    // `default` in last case
    (@list
        (default => $body:expr)
        $count:tt
        $branches:tt
        ()
        $complete:tt
    ) => {
        $crate::select_internal!(
            @list
            ()
            $count
            $branches
            ($body)
            $complete
        )
    };
    // `default` in no last case
    (@list
        (default => $body:block, $($tokens:tt)*)
        $count:tt
        $branches:tt
        ()
        $complete:tt
    ) => {
        $crate::select_internal!(
            @list
            ($($tokens)*)
            $count
            $branches
            ($body)
            $complete
        )
    };
    (@list
        (default => $body:block $($tokens:tt)*)
        $count:tt
        $branches:tt
        ()
        $complete:tt
    ) => {
        $crate::select_internal!(
            @list
            ($($tokens)*)
            $count
            $branches
            ($body)
            $complete
        )
    };
    (@list
        (default => $body:expr, $($tokens:tt)*)
        $count:tt
        $branches:tt
        ()
        $complete:tt
    ) => {
        $crate::select_internal!(
            @list
            ($($tokens)*)
            $count
            $branches
            ($body)
            $complete
        )
    };
    (@list
        (default => $($tokens:tt)*)
        $count:tt
        $branches:tt
        $default:tt
        $complete:tt
    ) => {
        $crate::select_internal!(@missing-comma "default clause");
    };

    (@list
        ()
        ($pattern:pat = $($token:tt)*)
        $count:tt
        $branches:tt
        $default:tt
        $complete:tt
    ) => {
        $crate::select_internal!(
            @list
            ($pattern)
            ($pattern = $($token)*)
            $count
            $branches
            $default
            $complete
        )
    };

    // block with trailing comma
    (@list
        (ref mut $ident:ident@$subpattern:pat = $future:expr, if $condition:expr => $body:block, $($tokens:tt)*)
        ($($count:tt)*)
        ($($branch:tt)*)
        $default:tt
        $complete:tt
    ) => {
        $crate::select_internal!(
            @list
            ($($tokens)*)
            ($($count)* _)
            ($($branch)* [($($count)*), $subpattern] ref mut $ident@$subpattern = $future, if $condition => $body,)
            $default
            $complete
        )
    };
    (@list
        (ref $ident:ident@$subpattern:pat = $future:expr, if $condition:expr => $body:block, $($tokens:tt)*)
        ($($count:tt)*)
        ($($branch:tt)*)
        $default:tt
        $complete:tt
    ) => {
        $crate::select_internal!(
            @list
            ($($tokens)*)
            ($($count)* _)
            ($($branch)* [($($count)*), $subpattern] ref $ident@$subpattern = $future, if $condition => $body,)
            $default
            $complete
        )
    };
    (@list
        (mut $ident:ident@$subpattern:pat = $future:expr, if $condition:expr => $body:block, $($tokens:tt)*)
        ($($count:tt)*)
        ($($branch:tt)*)
        $default:tt
        $complete:tt
    ) => {
        $crate::select_internal!(
            @list
            ($($tokens)*)
            ($($count)* _)
            ($($branch)* [($($count)*), $subpattern] mut $ident@$subpattern = $future, if $condition => $body,)
            $default
            $complete
        )
    };
    (@list
        ($pattern:pat = $future:expr, if $condition:expr => $body:block, $($tokens:tt)*)
        ($($count:tt)*)
        ($($branch:tt)*)
        $default:tt
        $complete:tt
    ) => {
        $crate::select_internal!(
            @list
            ($($tokens)*)
            ($($count)* _)
            ($($branch)* [($($count)*), $pattern] $pattern = $future, if $condition => $body,)
            $default
            $complete
        )
    };
    (@list
        (ref mut $ident:ident@$subpattern:pat = $future:expr => $body:block, $($tokens:tt)*)
        ($($count:tt)*)
        ($($branch:tt)*)
        $default:tt
        $complete:tt
    ) => {
        $crate::select_internal!(
            @list
            ($($tokens)*)
            ($($count)* _)
            ($($branch)* [($($count)*), $subpattern] ref mut $ident@$subpattern = $future, if true => $body,)
            $default
            $complete
        )
    };
    (@list
        (ref $ident:ident@$subpattern:pat = $future:expr => $body:block, $($tokens:tt)*)
        ($($count:tt)*)
        ($($branch:tt)*)
        $default:tt
        $complete:tt
    ) => {
        $crate::select_internal!(
            @list
            ($($tokens)*)
            ($($count)* _)
            ($($branch)* [($($count)*), $subpattern] ref $ident@$subpattern = $future, if true => $body,)
            $default
            $complete
        )
    };
    (@list
        (mut $ident:ident@$subpattern:pat = $future:expr => $body:block, $($tokens:tt)*)
        ($($count:tt)*)
        ($($branch:tt)*)
        $default:tt
        $complete:tt
    ) => {
        $crate::select_internal!(
            @list
            ($($tokens)*)
            ($($count)* _)
            ($($branch)* [($($count)*), $subpattern] mut $ident@$subpattern = $future, if true => $body,)
            $default
            $complete
        )
    };
    (@list
        ($pattern:pat = $future:expr => $body:block, $($tokens:tt)*)
        ($($count:tt)*)
        ($($branch:tt)*)
        $default:tt
        $complete:tt
        ) => {
        $crate::select_internal!(
            @list
            ($($tokens)*)
            ($($count)* _)
            ($($branch)* [($($count)*), $pattern] $pattern = $future, if true => $body,)
            $default
            $complete
        )
    };

    // block without trailing comma
    (@list
        (ref mut $ident:ident@$subpattern:pat = $future:expr, if $condition:expr => $body:block $($tokens:tt)*)
        ($($count:tt)*)
        ($($branch:tt)*)
        $default:tt
        $complete:tt
    ) => {
        $crate::select_internal!(
            @list
            ($($tokens)*)
            ($($count)* _)
            ($($branch)* [($($count)*), $subpattern] ref mut $ident@$subpattern = $future, if $condition => $body,)
            $default
            $complete
        )
    };
    (@list
        (ref $ident:ident@$subpattern:pat = $future:expr, if $condition:expr => $body:block $($tokens:tt)*)
        ($($count:tt)*)
        ($($branch:tt)*)
        $default:tt
        $complete:tt
    ) => {
        $crate::select_internal!(
            @list
            ($($tokens)*)
            ($($count)* _)
            ($($branch)* [($($count)*), $subpattern] ref $ident@$subpattern = $future, if $condition => $body,)
            $default
            $complete
        )
    };
    (@list
        (mut $ident:ident@$subpattern:pat = $future:expr, if $condition:expr => $body:block $($tokens:tt)*)
        ($($count:tt)*)
        ($($branch:tt)*)
        $default:tt
        $complete:tt
    ) => {
        $crate::select_internal!(
            @list
            ($($tokens)*)
            ($($count)* _)
            ($($branch)* [($($count)*), $subpattern] mut $ident@$subpattern = $future, if $condition => $body,)
            $default
            $complete
        )
    };
    (@list
        ($pattern:pat = $future:expr, if $condition:expr => $body:block $($tokens:tt)*)
        ($($count:tt)*)
        ($($branch:tt)*)
        $default:tt
        $complete:tt
    ) => {
        $crate::select_internal!(
            @list
            ($($tokens)*)
            ($($count)* _)
            ($($branch)* [($($count)*), $pattern] $pattern = $future, if $condition => $body,)
            $default
            $complete
        )
    };
    (@list
        (ref mut $ident:ident@$subpattern:pat = $future:expr => $body:block $($tokens:tt)*)
        ($($count:tt)*)
        ($($branch:tt)*)
        $default:tt
        $complete:tt
    ) => {
        $crate::select_internal!(
            @list
            ($($tokens)*)
            ($($count)* _)
            ($($branch)* [($($count)*), $subpattern] ref mut $ident@$subpattern = $future, if true => $body,)
            $default
            $complete
        )
    };
    (@list
        (ref $ident:ident@$subpattern:pat = $future:expr => $body:block $($tokens:tt)*)
        ($($count:tt)*)
        ($($branch:tt)*)
        $default:tt
        $complete:tt
    ) => {
        $crate::select_internal!(
            @list
            ($($tokens)*)
            ($($count)* _)
            ($($branch)* [($($count)*), $subpattern] ref $ident@$subpattern = $future, if true => $body,)
            $default
            $complete
        )
    };
    (@list
        (mut $ident:ident@$subpattern:pat = $future:expr => $body:block $($tokens:tt)*)
        ($($count:tt)*)
        ($($branch:tt)*)
        $default:tt
        $complete:tt
    ) => {
        $crate::select_internal!(
            @list
            ($($tokens)*)
            ($($count)* _)
            ($($branch)* [($($count)*), $subpattern] mut $ident@$subpattern = $future, if true => $body,)
            $default
            $complete
        )
    };
    (@list
        ($pattern:pat = $future:expr => $body:block $($tokens:tt)*)
        ($($count:tt)*)
        ($($branch:tt)*)
        $default:tt
        $complete:tt
        ) => {
        $crate::select_internal!(
            @list
            ($($tokens)*)
            ($($count)* _)
            ($($branch)* [($($count)*), $pattern] $pattern = $future, if true => $body,)
            $default
            $complete
        )
    };

    // expression without a trailing comma in last clause
    (@list
        (ref mut $ident:ident@$subpattern:pat = $future:expr, if $condition:expr => $body:expr)
        ($($count:tt)*)
        ($($branch:tt)*)
        $default:tt
        $complete:tt
    ) => {
        $crate::select_internal!(
            @list
            ()
            ($($count)* _)
            ($($branch)* [($($count)*), $subpattern] ref mut $ident@$subpattern = $future, if $condition => $body,)
            $default
            $complete
        )
    };
    (@list
        (ref $ident:ident@$subpattern:pat = $future:expr, if $condition:expr => $body:expr)
        ($($count:tt)*)
        ($($branch:tt)*)
        $default:tt
        $complete:tt
    ) => {
        $crate::select_internal!(
            @list
            ()
            ($($count)* _)
            ($($branch)* [($($count)*), $subpattern] ref $ident@$subpattern = $future, if $condition => $body,)
            $default
            $complete
        )
    };
    (@list
        (mut $ident:ident@$subpattern:pat = $future:expr, if $condition:expr => $body:expr)
        ($($count:tt)*)
        ($($branch:tt)*)
        $default:tt
        $complete:tt
    ) => {
        $crate::select_internal!(
            @list
            ()
            ($($count)* _)
            ($($branch)* [($($count)*), $subpattern] mut $ident@$subpattern = $future, if $condition => $body,)
            $default
            $complete
        )
    };
    (@list
        ($pattern:pat = $future:expr, if $condition:expr => $body:expr)
        ($($count:tt)*)
        ($($branch:tt)*)
        $default:tt
        $complete:tt
        ) => {
        $crate::select_internal!(
            @list
            ()
            ($($count)* _)
            ($($branch)* [($($count)*), $pattern] $pattern = $future, if $condition => $body,)
            $default
            $complete
        )
    };
    (@list
        (ref mut $ident:ident@$subpattern:pat = $future:expr => $body:expr)
        ($($count:tt)*)
        ($($branch:tt)*)
        $default:tt
        $complete:tt
    ) => {
        $crate::select_internal!(
            @list
            ()
            ($($count)* _)
            ($($branch)* [($($count)*), $subpattern] ref mut $ident@$subpattern = $future, if true => $body,)
            $default
            $complete
        )
    };
    (@list
        (ref $ident:ident@$subpattern:pat = $future:expr => $body:expr)
        ($($count:tt)*)
        ($($branch:tt)*)
        $default:tt
        $complete:tt
    ) => {
        $crate::select_internal!(
            @list
            ()
            ($($count)* _)
            ($($branch)* [($($count)*), $subpattern] ref $ident@$subpattern = $future, if true => $body,)
            $default
            $complete
        )
    };
    (@list
        (mut $ident:ident@$subpattern:pat = $future:expr => $body:expr)
        ($($count:tt)*)
        ($($branch:tt)*)
        $default:tt
        $complete:tt
    ) => {
        $crate::select_internal!(
            @list
            ()
            ($($count)* _)
            ($($branch)* [($($count)*), $subpattern] mut $ident@$subpattern = $future, if true => $body,)
            $default
            $complete
        )
    };
    (@list
        ($pattern:pat = $future:expr => $body:expr)
        ($($count:tt)*)
        ($($branch:tt)*)
        $default:tt
        $complete:tt
    ) => {
        $crate::select_internal!(
            @list
            ()
            ($($count)* _)
            ($($branch)* [($($count)*), $pattern] $pattern = $future, if true => $body,)
            $default
            $complete
        )
    };


    // expression with a trailing comma
    (@list
        (ref mut $ident:ident@$subpattern:pat = $future:expr, if $condition:expr => $body:expr, $($tokens:tt)*)
        ($($count:tt)*)
        ($($branch:tt)*)
        $default:tt
        $complete:tt
        ) => {
        $crate::select_internal!(
            @list
            ($($tokens)*)
            ($($count)* _)
            ($($branch)* [($($count)*), $subpattern] ref mut $ident@$subpattern = $future, if $condition => $body,)
            $default
            $complete
        )
    };
    (@list
        (ref $ident:ident@$subpattern:pat = $future:expr, if $condition:expr => $body:expr, $($tokens:tt)*)
        ($($count:tt)*)
        ($($branch:tt)*)
        $default:tt
        $complete:tt
        ) => {
        $crate::select_internal!(
            @list
            ($($tokens)*)
            ($($count)* _)
            ($($branch)* [($($count)*), $subpattern] ref $ident@$subpattern = $future, if $condition => $body,)
            $default
            $complete
        )
    };
    (@list
        (mut $ident:ident@$subpattern:pat = $future:expr, if $condition:expr => $body:expr, $($tokens:tt)*)
        ($($count:tt)*)
        ($($branch:tt)*)
        $default:tt
        $complete:tt
        ) => {
        $crate::select_internal!(
            @list
            ($($tokens)*)
            ($($count)* _)
            ($($branch)* [($($count)*), $subpattern] mut $ident@$subpattern = $future, if $condition => $body,)
            $default
            $complete
        )
    };
    (@list
        ($pattern:pat = $future:expr, if $condition:expr => $body:expr, $($tokens:tt)*)
        ($($count:tt)*)
        ($($branch:tt)*)
        $default:tt
        $complete:tt
        ) => {
        $crate::select_internal!(
            @list
            ($($tokens)*)
            ($($count)* _)
            ($($branch)* [($($count)*), $pattern] $pattern = $future, if $condition => $body,)
            $default
            $complete
        )
    };
    (@list
        (ref mut $ident:ident@$subpattern:pat = $future:expr => $body:expr, $($tokens:tt)*)
        ($($count:tt)*)
        ($($branch:tt)*)
        $default:tt
        $complete:tt
        ) => {
        $crate::select_internal!(
            @list
            ($($tokens)*)
            ($($count)* _)
            ($($branch)* [($($count)*), $subpattern] ref mut $ident@$subpattern = $future, if true => $body,)
            $default
            $complete
        )
    };
    (@list
        (ref $ident:ident@$subpattern:pat = $future:expr => $body:expr, $($tokens:tt)*)
        ($($count:tt)*)
        ($($branch:tt)*)
        $default:tt
        $complete:tt
        ) => {
        $crate::select_internal!(
            @list
            ($($tokens)*)
            ($($count)* _)
            ($($branch)* [($($count)*), $subpattern] ref $ident@$subpattern = $future, if true => $body,)
            $default
            $complete
        )
    };
    (@list
        (mut $ident:ident@$subpattern:pat = $future:expr => $body:expr, $($tokens:tt)*)
        ($($count:tt)*)
        ($($branch:tt)*)
        $default:tt
        $complete:tt
        ) => {
        $crate::select_internal!(
            @list
            ($($tokens)*)
            ($($count)* _)
            ($($branch)* [($($count)*), $subpattern] mut $ident@$subpattern = $future, if true => $body,)
            $default
            $complete
        )
    };
    (@list
        ($pattern:pat = $future:expr => $body:expr, $($tokens:tt)*)
        ($($count:tt)*)
        ($($branch:tt)*)
        $default:tt
        $complete:tt
    ) => {
        $crate::select_internal!(
            @list
            ($($tokens)*)
            ($($count)* _)
            ($($branch)* [($($count)*), $pattern] $pattern = $future, if true => $body,)
            $default
            $complete
        )
    };

    // complain missing comma.
    (@list
        ($pattern:pat = $future:expr $(, if condition:expr)? => $($tokens:tt)*)
        $count:tt
        $branches:tt
        $default:tt
        $complete:tt
    ) => {
        $crate::select_internal!(@missing-comma concat!("clause \"", stringify!($pattern = $future), "\""));
    };


    (@list
        $tokens:tt
        $count:tt
        $branches:tt
        $default:tt
        $complete:tt
    ) => {
        compile_error!(concat!("fail to list select: ", stringify!($tokens)))
    };

    (@none $_:tt) => { ::core::option::Option::None };

    // Init select.
    (@poll
        non_blocking = $non_blocking:expr,
        when_completed = $when_completed:expr,
        count = $count:tt,
        branches = ($([$index:tt, $capture:pat] $pattern:pat = $future:expr, if $condition:expr => $evaluation:expr,)+),
        default = $default:expr,
        complete = $complete:expr
    ) => {{
        const BRANCHES: usize = $crate::select_internal!(@count $count);
        const COMPLETED: u64 = if BRANCHES == 64 { u64::MAX } else { (1u64 << BRANCHES) - 1 };
        $crate::select_internal!(@output-type $count);
        let mut output = {
            // Scope to drop before call evaluation code.
            let mut __select_futures = ( $($crate::select_internal!(@none $index) ,)* );
            let mut __select_futures = &mut __select_futures;
            $(
                if $condition {
                    $crate::select_internal!(@assign $index, __select_futures, Some($future));
                }
            )*
            ::core::future::poll_fn(|cx| {
                let stack_addr = 0usize;
                let start = ((&stack_addr as *const usize as usize) >> 3) % BRANCHES;
                let mut completions = 0;
                for i in 0..BRANCHES {
                    let branch = (start + i) % BRANCHES;
                    match branch {
                        $(
                            $crate::select_internal!(@count $index) => {
                                let Some(future) = &mut $crate::select_internal!(@access $index, __select_futures) else {
                                    completions |= 1 << $crate::select_internal!(@count $index);
                                    continue;
                                };
                                #[allow(unused_unsafe)]
                                let future = unsafe { ::core::pin::Pin::new_unchecked(future) };
                                let mut output = match ::core::future::Future::poll(future, cx) {
                                    ::core::task::Poll::Ready(output) => output,
                                    ::core::task::Poll::Pending => continue,
                                };
                                $crate::select_internal!(@assign $index, __select_futures, ::core::option::Option::None);
                                completions |= 1 << $crate::select_internal!(@count $index);
                                #[allow(unreachable_patterns)]
                                #[allow(unused_variables)]
                                match &mut output {
                                    $capture => {},
                                    _  => continue,
                                };
                                return ::core::task::Poll::Ready($crate::select_internal!(@wrap $index, output));
                            }
                         )*
                        _ => unreachable!("select! encounter mismatch branch in polling"),
                    }
                }
                if completions == COMPLETED && ($when_completed || !$non_blocking) {
                    return ::core::task::Poll::Ready(__SelectOutput::Completed);
                }
                if $non_blocking {
                    return ::core::task::Poll::Ready(__SelectOutput::WouldBlock);
                }
                ::core::task::Poll::Pending
            }).await
        };
        match output {
            $(
                $crate::select_internal!(@unwrap $index, $pattern) => $evaluation,
             )*
            __SelectOutput::Completed => $complete,
            __SelectOutput::WouldBlock => $default,
            #[allow(unreachable_patterns)]
            _ => unreachable!("select! fail to pattern match"),
        }
    }};

    (@missing-comma $msg:expr) => {
        compile_error!(concat!("`,` is required for expression in not last branch, non in ", $msg))
    };

    (@too-many-branches) => {
        compile_error!("too many branches, at most 64")
    };

    (@no-branch {
        default: (),
        complete: (),
    }) => {
        compile_error!("`select!`: no branch")
    };

    (@no-branch {
        default: (),
        complete: ($complete:tt),
    }) => {
        compile_error!("`select!`: no branch except `complete`")
    };

    (@no-branch {
        default: ($default:tt),
        complete: (),
    }) => {
        compile_error!("`select!`: no branch except `default`")
    };

    (@no-branch {
        default: ($default:tt),
        complete: ($complete:tt),
    }) => {
        compile_error!("no branch except `default` and `complete`")
    };

    (@count ()) => { 0 };
    (@count (_)) => { 1 };
    (@count (_ _)) => { 2 };
    (@count (_ _ _)) => { 3 };
    (@count (_ _ _ _)) => { 4 };
    (@count (_ _ _ _ _)) => { 5 };
    (@count (_ _ _ _ _ _)) => { 6 };
    (@count (_ _ _ _ _ _ _)) => { 7 };
    (@count (_ _ _ _ _ _ _ _)) => { 8 };
    (@count (_ _ _ _ _ _ _ _ _)) => { 9 };
    (@count (_ _ _ _ _ _ _ _ _ _)) => { 10 };
    (@count (_ _ _ _ _ _ _ _ _ _ _)) => { 11 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _)) => { 12 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _)) => { 13 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 14 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 15 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 16 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 17 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 18 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 19 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 20 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 21 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 22 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 23 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 24 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 25 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 26 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 27 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 28 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 29 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 30 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 31 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 32 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 33 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 34 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 35 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 36 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 37 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 38 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 39 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 40 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 41 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 42 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 43 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 44 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 45 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 46 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 47 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 48 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 49 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 50 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 51 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 52 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 53 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 54 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 55 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 56 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 57 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 58 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 59 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 60 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 61 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 62 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 63 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => { 64 };
    (@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ $($_:pat)*)) => {
        $crate::select_internal!(@too-many-branches)
    };

    (@access (), $futures:ident) => { $futures.0 };
    (@access (_), $futures:ident) => { $futures.1 };
    (@access (_ _), $futures:ident) => { $futures.2 };
    (@access (_ _ _), $futures:ident) => { $futures.3 };
    (@access (_ _ _ _), $futures:ident) => { $futures.4 };
    (@access (_ _ _ _ _), $futures:ident) => { $futures.5 };
    (@access (_ _ _ _ _ _), $futures:ident) => { $futures.6 };
    (@access (_ _ _ _ _ _ _), $futures:ident) => { $futures.7 };
    (@access (_ _ _ _ _ _ _ _), $futures:ident) => { $futures.8 };
    (@access (_ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.9 };
    (@access (_ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.10 };
    (@access (_ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.11 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.12 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.13 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.14 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.15 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.16 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.17 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.18 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.19 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.20 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.21 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.22 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.23 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.24 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.25 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.26 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.27 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.28 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.29 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.30 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.31 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.32 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.33 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.34 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.35 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.36 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.37 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.38 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.39 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.40 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.41 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.42 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.43 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.44 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.45 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.46 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.47 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.48 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.49 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.50 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.51 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.52 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.53 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.54 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.55 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.56 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.57 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.58 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.59 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.60 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.61 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.62 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident) => { $futures.63 };
    (@access (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ $($_:pat)*) $($_2:tt)*) => {
        $crate::select_internal!(@too-many-branches)
    };

    (@assign (), $futures:ident, $future:expr) => { $futures.0 = $future; };
    (@assign (_), $futures:ident, $future:expr) => { $futures.1 = $future; };
    (@assign (_ _), $futures:ident, $future:expr) => { $futures.2 = $future; };
    (@assign (_ _ _), $futures:ident, $future:expr) => { $futures.3 = $future; };
    (@assign (_ _ _ _), $futures:ident, $future:expr) => { $futures.4 = $future; };
    (@assign (_ _ _ _ _), $futures:ident, $future:expr) => { $futures.5 = $future; };
    (@assign (_ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.6 = $future; };
    (@assign (_ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.7 = $future; };
    (@assign (_ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.8 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.9 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.10 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.11 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.12 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.13 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.14 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.15 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.16 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.17 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.18 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.19 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.20 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.21 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.22 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.23 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.24 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.25 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.26 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.27 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.28 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.29 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.30 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.31 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.32 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.33 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.34 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.35 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.36 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.37 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.38 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.39 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.40 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.41 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.42 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.43 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.44 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.45 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.46 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.47 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.48 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.49 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.50 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.51 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.52 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.53 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.54 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.55 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.56 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.57 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.58 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.59 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.60 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.61 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.62 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $futures:ident, $future:expr) => { $futures.63 = $future; };
    (@assign (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ $($_:pat)*) $($_2:tt)*) => {
        $crate::select_internal!(@too-many-branches)
    };

    (@wrap (), $name:ident) => { __SelectOutput::_0($name) };
    (@wrap (_), $name:ident) => { __SelectOutput::_1($name) };
    (@wrap (_ _), $name:ident) => { __SelectOutput::_2($name) };
    (@wrap (_ _ _), $name:ident) => { __SelectOutput::_3($name) };
    (@wrap (_ _ _ _), $name:ident) => { __SelectOutput::_4($name) };
    (@wrap (_ _ _ _ _), $name:ident) => { __SelectOutput::_5($name) };
    (@wrap (_ _ _ _ _ _), $name:ident) => { __SelectOutput::_6($name) };
    (@wrap (_ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_7($name) };
    (@wrap (_ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_8($name) };
    (@wrap (_ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_9($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_10($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_11($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_12($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_13($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_14($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_15($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_16($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_17($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_18($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_19($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_20($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_21($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_22($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_23($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_24($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_25($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_26($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_27($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_28($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_29($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_30($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_31($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_32($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_33($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_34($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_35($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_36($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_37($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_38($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_39($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_40($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_41($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_42($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_43($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_44($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_45($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_46($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_47($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_48($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_49($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_50($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_51($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_52($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_53($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_54($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_55($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_56($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_57($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_58($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_59($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_60($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_61($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_62($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $name:ident) => { __SelectOutput::_63($name) };
    (@wrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ $($_:pat)*) $($_2:tt)*) => {
        $crate::select_internal!(@too-many-branches)
    };

    (@unwrap (), $pattern:pat) => { __SelectOutput::_0($pattern) };
    (@unwrap (_), $pattern:pat) => { __SelectOutput::_1($pattern) };
    (@unwrap (_ _), $pattern:pat) => { __SelectOutput::_2($pattern) };
    (@unwrap (_ _ _), $pattern:pat) => { __SelectOutput::_3($pattern) };
    (@unwrap (_ _ _ _), $pattern:pat) => { __SelectOutput::_4($pattern) };
    (@unwrap (_ _ _ _ _), $pattern:pat) => { __SelectOutput::_5($pattern) };
    (@unwrap (_ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_6($pattern) };
    (@unwrap (_ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_7($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_8($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_9($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_10($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_11($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_12($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_13($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_14($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_15($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_16($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_17($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_18($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_19($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_20($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_21($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_22($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_23($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_24($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_25($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_26($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_27($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_28($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_29($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_30($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_31($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_32($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_33($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_34($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_35($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_36($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_37($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_38($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_39($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_40($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_41($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_42($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_43($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_44($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_45($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_46($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_47($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_48($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_49($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_50($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_51($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_52($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_53($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_54($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_55($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_56($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_57($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_58($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_59($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_60($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_61($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_62($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _), $pattern:pat) => { __SelectOutput::_63($pattern) };
    (@unwrap (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ $($_:pat)*) $($_2:tt)*) => {
        $crate::select_internal!(@too-many-branches)
    };

    (@output-type (_)) => {
        enum __SelectOutput<_0> {
            Completed,
            WouldBlock,
            _0(_0),
        }
    };
    (@output-type (_ _)) => {
        enum __SelectOutput<_0, _1> {
            Completed,
            WouldBlock,
            _0(_0),
            _1(_1),
        }
    };
    (@output-type (_ _ _)) => {
        enum __SelectOutput<_0, _1, _2> {
            Completed,
            WouldBlock,
            _0(_0),
            _1(_1),
            _2(_2),
        }
    };
    (@output-type (_ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
        }
    };
    (@output-type (_ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
        }
    };
    (@output-type (_ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
        }
    };
    (@output-type (_ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
            _21(T21),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
            _21(T21),
            _22(T22),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
            _21(T21),
            _22(T22),
            _23(T23),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
            _21(T21),
            _22(T22),
            _23(T23),
            _24(T24),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
            _21(T21),
            _22(T22),
            _23(T23),
            _24(T24),
            _25(T25),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
            _21(T21),
            _22(T22),
            _23(T23),
            _24(T24),
            _25(T25),
            _26(T26),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
            _21(T21),
            _22(T22),
            _23(T23),
            _24(T24),
            _25(T25),
            _26(T26),
            _27(T27),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
            _21(T21),
            _22(T22),
            _23(T23),
            _24(T24),
            _25(T25),
            _26(T26),
            _27(T27),
            _28(T28),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
            _21(T21),
            _22(T22),
            _23(T23),
            _24(T24),
            _25(T25),
            _26(T26),
            _27(T27),
            _28(T28),
            _29(T29),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
            _21(T21),
            _22(T22),
            _23(T23),
            _24(T24),
            _25(T25),
            _26(T26),
            _27(T27),
            _28(T28),
            _29(T29),
            _30(T30),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
            _21(T21),
            _22(T22),
            _23(T23),
            _24(T24),
            _25(T25),
            _26(T26),
            _27(T27),
            _28(T28),
            _29(T29),
            _30(T30),
            _31(T31),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
            _21(T21),
            _22(T22),
            _23(T23),
            _24(T24),
            _25(T25),
            _26(T26),
            _27(T27),
            _28(T28),
            _29(T29),
            _30(T30),
            _31(T31),
            _32(T32),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
            _21(T21),
            _22(T22),
            _23(T23),
            _24(T24),
            _25(T25),
            _26(T26),
            _27(T27),
            _28(T28),
            _29(T29),
            _30(T30),
            _31(T31),
            _32(T32),
            _33(T33),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
            _21(T21),
            _22(T22),
            _23(T23),
            _24(T24),
            _25(T25),
            _26(T26),
            _27(T27),
            _28(T28),
            _29(T29),
            _30(T30),
            _31(T31),
            _32(T32),
            _33(T33),
            _34(T34),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
            _21(T21),
            _22(T22),
            _23(T23),
            _24(T24),
            _25(T25),
            _26(T26),
            _27(T27),
            _28(T28),
            _29(T29),
            _30(T30),
            _31(T31),
            _32(T32),
            _33(T33),
            _34(T34),
            _35(T35),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
            _21(T21),
            _22(T22),
            _23(T23),
            _24(T24),
            _25(T25),
            _26(T26),
            _27(T27),
            _28(T28),
            _29(T29),
            _30(T30),
            _31(T31),
            _32(T32),
            _33(T33),
            _34(T34),
            _35(T35),
            _36(T36),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36, T37> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
            _21(T21),
            _22(T22),
            _23(T23),
            _24(T24),
            _25(T25),
            _26(T26),
            _27(T27),
            _28(T28),
            _29(T29),
            _30(T30),
            _31(T31),
            _32(T32),
            _33(T33),
            _34(T34),
            _35(T35),
            _36(T36),
            _37(T37),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36, T37, T38> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
            _21(T21),
            _22(T22),
            _23(T23),
            _24(T24),
            _25(T25),
            _26(T26),
            _27(T27),
            _28(T28),
            _29(T29),
            _30(T30),
            _31(T31),
            _32(T32),
            _33(T33),
            _34(T34),
            _35(T35),
            _36(T36),
            _37(T37),
            _38(T38),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36, T37, T38, T39> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
            _21(T21),
            _22(T22),
            _23(T23),
            _24(T24),
            _25(T25),
            _26(T26),
            _27(T27),
            _28(T28),
            _29(T29),
            _30(T30),
            _31(T31),
            _32(T32),
            _33(T33),
            _34(T34),
            _35(T35),
            _36(T36),
            _37(T37),
            _38(T38),
            _39(T39),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36, T37, T38, T39, T40> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
            _21(T21),
            _22(T22),
            _23(T23),
            _24(T24),
            _25(T25),
            _26(T26),
            _27(T27),
            _28(T28),
            _29(T29),
            _30(T30),
            _31(T31),
            _32(T32),
            _33(T33),
            _34(T34),
            _35(T35),
            _36(T36),
            _37(T37),
            _38(T38),
            _39(T39),
            _40(T40),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36, T37, T38, T39, T40, T41> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
            _21(T21),
            _22(T22),
            _23(T23),
            _24(T24),
            _25(T25),
            _26(T26),
            _27(T27),
            _28(T28),
            _29(T29),
            _30(T30),
            _31(T31),
            _32(T32),
            _33(T33),
            _34(T34),
            _35(T35),
            _36(T36),
            _37(T37),
            _38(T38),
            _39(T39),
            _40(T40),
            _41(T41),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36, T37, T38, T39, T40, T41, T42> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
            _21(T21),
            _22(T22),
            _23(T23),
            _24(T24),
            _25(T25),
            _26(T26),
            _27(T27),
            _28(T28),
            _29(T29),
            _30(T30),
            _31(T31),
            _32(T32),
            _33(T33),
            _34(T34),
            _35(T35),
            _36(T36),
            _37(T37),
            _38(T38),
            _39(T39),
            _40(T40),
            _41(T41),
            _42(T42),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36, T37, T38, T39, T40, T41, T42, T43> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
            _21(T21),
            _22(T22),
            _23(T23),
            _24(T24),
            _25(T25),
            _26(T26),
            _27(T27),
            _28(T28),
            _29(T29),
            _30(T30),
            _31(T31),
            _32(T32),
            _33(T33),
            _34(T34),
            _35(T35),
            _36(T36),
            _37(T37),
            _38(T38),
            _39(T39),
            _40(T40),
            _41(T41),
            _42(T42),
            _43(T43),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36, T37, T38, T39, T40, T41, T42, T43, T44> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
            _21(T21),
            _22(T22),
            _23(T23),
            _24(T24),
            _25(T25),
            _26(T26),
            _27(T27),
            _28(T28),
            _29(T29),
            _30(T30),
            _31(T31),
            _32(T32),
            _33(T33),
            _34(T34),
            _35(T35),
            _36(T36),
            _37(T37),
            _38(T38),
            _39(T39),
            _40(T40),
            _41(T41),
            _42(T42),
            _43(T43),
            _44(T44),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36, T37, T38, T39, T40, T41, T42, T43, T44, T45> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
            _21(T21),
            _22(T22),
            _23(T23),
            _24(T24),
            _25(T25),
            _26(T26),
            _27(T27),
            _28(T28),
            _29(T29),
            _30(T30),
            _31(T31),
            _32(T32),
            _33(T33),
            _34(T34),
            _35(T35),
            _36(T36),
            _37(T37),
            _38(T38),
            _39(T39),
            _40(T40),
            _41(T41),
            _42(T42),
            _43(T43),
            _44(T44),
            _45(T45),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36, T37, T38, T39, T40, T41, T42, T43, T44, T45, T46> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
            _21(T21),
            _22(T22),
            _23(T23),
            _24(T24),
            _25(T25),
            _26(T26),
            _27(T27),
            _28(T28),
            _29(T29),
            _30(T30),
            _31(T31),
            _32(T32),
            _33(T33),
            _34(T34),
            _35(T35),
            _36(T36),
            _37(T37),
            _38(T38),
            _39(T39),
            _40(T40),
            _41(T41),
            _42(T42),
            _43(T43),
            _44(T44),
            _45(T45),
            _46(T46),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36, T37, T38, T39, T40, T41, T42, T43, T44, T45, T46, T47> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
            _21(T21),
            _22(T22),
            _23(T23),
            _24(T24),
            _25(T25),
            _26(T26),
            _27(T27),
            _28(T28),
            _29(T29),
            _30(T30),
            _31(T31),
            _32(T32),
            _33(T33),
            _34(T34),
            _35(T35),
            _36(T36),
            _37(T37),
            _38(T38),
            _39(T39),
            _40(T40),
            _41(T41),
            _42(T42),
            _43(T43),
            _44(T44),
            _45(T45),
            _46(T46),
            _47(T47),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36, T37, T38, T39, T40, T41, T42, T43, T44, T45, T46, T47, T48> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
            _21(T21),
            _22(T22),
            _23(T23),
            _24(T24),
            _25(T25),
            _26(T26),
            _27(T27),
            _28(T28),
            _29(T29),
            _30(T30),
            _31(T31),
            _32(T32),
            _33(T33),
            _34(T34),
            _35(T35),
            _36(T36),
            _37(T37),
            _38(T38),
            _39(T39),
            _40(T40),
            _41(T41),
            _42(T42),
            _43(T43),
            _44(T44),
            _45(T45),
            _46(T46),
            _47(T47),
            _48(T48),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36, T37, T38, T39, T40, T41, T42, T43, T44, T45, T46, T47, T48, T49> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
            _21(T21),
            _22(T22),
            _23(T23),
            _24(T24),
            _25(T25),
            _26(T26),
            _27(T27),
            _28(T28),
            _29(T29),
            _30(T30),
            _31(T31),
            _32(T32),
            _33(T33),
            _34(T34),
            _35(T35),
            _36(T36),
            _37(T37),
            _38(T38),
            _39(T39),
            _40(T40),
            _41(T41),
            _42(T42),
            _43(T43),
            _44(T44),
            _45(T45),
            _46(T46),
            _47(T47),
            _48(T48),
            _49(T49),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36, T37, T38, T39, T40, T41, T42, T43, T44, T45, T46, T47, T48, T49, T50> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
            _21(T21),
            _22(T22),
            _23(T23),
            _24(T24),
            _25(T25),
            _26(T26),
            _27(T27),
            _28(T28),
            _29(T29),
            _30(T30),
            _31(T31),
            _32(T32),
            _33(T33),
            _34(T34),
            _35(T35),
            _36(T36),
            _37(T37),
            _38(T38),
            _39(T39),
            _40(T40),
            _41(T41),
            _42(T42),
            _43(T43),
            _44(T44),
            _45(T45),
            _46(T46),
            _47(T47),
            _48(T48),
            _49(T49),
            _50(T50),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36, T37, T38, T39, T40, T41, T42, T43, T44, T45, T46, T47, T48, T49, T50, T51> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
            _21(T21),
            _22(T22),
            _23(T23),
            _24(T24),
            _25(T25),
            _26(T26),
            _27(T27),
            _28(T28),
            _29(T29),
            _30(T30),
            _31(T31),
            _32(T32),
            _33(T33),
            _34(T34),
            _35(T35),
            _36(T36),
            _37(T37),
            _38(T38),
            _39(T39),
            _40(T40),
            _41(T41),
            _42(T42),
            _43(T43),
            _44(T44),
            _45(T45),
            _46(T46),
            _47(T47),
            _48(T48),
            _49(T49),
            _50(T50),
            _51(T51),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36, T37, T38, T39, T40, T41, T42, T43, T44, T45, T46, T47, T48, T49, T50, T51, T52> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
            _21(T21),
            _22(T22),
            _23(T23),
            _24(T24),
            _25(T25),
            _26(T26),
            _27(T27),
            _28(T28),
            _29(T29),
            _30(T30),
            _31(T31),
            _32(T32),
            _33(T33),
            _34(T34),
            _35(T35),
            _36(T36),
            _37(T37),
            _38(T38),
            _39(T39),
            _40(T40),
            _41(T41),
            _42(T42),
            _43(T43),
            _44(T44),
            _45(T45),
            _46(T46),
            _47(T47),
            _48(T48),
            _49(T49),
            _50(T50),
            _51(T51),
            _52(T52),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36, T37, T38, T39, T40, T41, T42, T43, T44, T45, T46, T47, T48, T49, T50, T51, T52, T53> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
            _21(T21),
            _22(T22),
            _23(T23),
            _24(T24),
            _25(T25),
            _26(T26),
            _27(T27),
            _28(T28),
            _29(T29),
            _30(T30),
            _31(T31),
            _32(T32),
            _33(T33),
            _34(T34),
            _35(T35),
            _36(T36),
            _37(T37),
            _38(T38),
            _39(T39),
            _40(T40),
            _41(T41),
            _42(T42),
            _43(T43),
            _44(T44),
            _45(T45),
            _46(T46),
            _47(T47),
            _48(T48),
            _49(T49),
            _50(T50),
            _51(T51),
            _52(T52),
            _53(T53),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36, T37, T38, T39, T40, T41, T42, T43, T44, T45, T46, T47, T48, T49, T50, T51, T52, T53, T54> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
            _21(T21),
            _22(T22),
            _23(T23),
            _24(T24),
            _25(T25),
            _26(T26),
            _27(T27),
            _28(T28),
            _29(T29),
            _30(T30),
            _31(T31),
            _32(T32),
            _33(T33),
            _34(T34),
            _35(T35),
            _36(T36),
            _37(T37),
            _38(T38),
            _39(T39),
            _40(T40),
            _41(T41),
            _42(T42),
            _43(T43),
            _44(T44),
            _45(T45),
            _46(T46),
            _47(T47),
            _48(T48),
            _49(T49),
            _50(T50),
            _51(T51),
            _52(T52),
            _53(T53),
            _54(T54),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36, T37, T38, T39, T40, T41, T42, T43, T44, T45, T46, T47, T48, T49, T50, T51, T52, T53, T54, T55> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
            _21(T21),
            _22(T22),
            _23(T23),
            _24(T24),
            _25(T25),
            _26(T26),
            _27(T27),
            _28(T28),
            _29(T29),
            _30(T30),
            _31(T31),
            _32(T32),
            _33(T33),
            _34(T34),
            _35(T35),
            _36(T36),
            _37(T37),
            _38(T38),
            _39(T39),
            _40(T40),
            _41(T41),
            _42(T42),
            _43(T43),
            _44(T44),
            _45(T45),
            _46(T46),
            _47(T47),
            _48(T48),
            _49(T49),
            _50(T50),
            _51(T51),
            _52(T52),
            _53(T53),
            _54(T54),
            _55(T55),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36, T37, T38, T39, T40, T41, T42, T43, T44, T45, T46, T47, T48, T49, T50, T51, T52, T53, T54, T55, T56> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
            _21(T21),
            _22(T22),
            _23(T23),
            _24(T24),
            _25(T25),
            _26(T26),
            _27(T27),
            _28(T28),
            _29(T29),
            _30(T30),
            _31(T31),
            _32(T32),
            _33(T33),
            _34(T34),
            _35(T35),
            _36(T36),
            _37(T37),
            _38(T38),
            _39(T39),
            _40(T40),
            _41(T41),
            _42(T42),
            _43(T43),
            _44(T44),
            _45(T45),
            _46(T46),
            _47(T47),
            _48(T48),
            _49(T49),
            _50(T50),
            _51(T51),
            _52(T52),
            _53(T53),
            _54(T54),
            _55(T55),
            _56(T56),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36, T37, T38, T39, T40, T41, T42, T43, T44, T45, T46, T47, T48, T49, T50, T51, T52, T53, T54, T55, T56, T57> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
            _21(T21),
            _22(T22),
            _23(T23),
            _24(T24),
            _25(T25),
            _26(T26),
            _27(T27),
            _28(T28),
            _29(T29),
            _30(T30),
            _31(T31),
            _32(T32),
            _33(T33),
            _34(T34),
            _35(T35),
            _36(T36),
            _37(T37),
            _38(T38),
            _39(T39),
            _40(T40),
            _41(T41),
            _42(T42),
            _43(T43),
            _44(T44),
            _45(T45),
            _46(T46),
            _47(T47),
            _48(T48),
            _49(T49),
            _50(T50),
            _51(T51),
            _52(T52),
            _53(T53),
            _54(T54),
            _55(T55),
            _56(T56),
            _57(T57),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36, T37, T38, T39, T40, T41, T42, T43, T44, T45, T46, T47, T48, T49, T50, T51, T52, T53, T54, T55, T56, T57, T58> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
            _21(T21),
            _22(T22),
            _23(T23),
            _24(T24),
            _25(T25),
            _26(T26),
            _27(T27),
            _28(T28),
            _29(T29),
            _30(T30),
            _31(T31),
            _32(T32),
            _33(T33),
            _34(T34),
            _35(T35),
            _36(T36),
            _37(T37),
            _38(T38),
            _39(T39),
            _40(T40),
            _41(T41),
            _42(T42),
            _43(T43),
            _44(T44),
            _45(T45),
            _46(T46),
            _47(T47),
            _48(T48),
            _49(T49),
            _50(T50),
            _51(T51),
            _52(T52),
            _53(T53),
            _54(T54),
            _55(T55),
            _56(T56),
            _57(T57),
            _58(T58),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36, T37, T38, T39, T40, T41, T42, T43, T44, T45, T46, T47, T48, T49, T50, T51, T52, T53, T54, T55, T56, T57, T58, T59> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
            _21(T21),
            _22(T22),
            _23(T23),
            _24(T24),
            _25(T25),
            _26(T26),
            _27(T27),
            _28(T28),
            _29(T29),
            _30(T30),
            _31(T31),
            _32(T32),
            _33(T33),
            _34(T34),
            _35(T35),
            _36(T36),
            _37(T37),
            _38(T38),
            _39(T39),
            _40(T40),
            _41(T41),
            _42(T42),
            _43(T43),
            _44(T44),
            _45(T45),
            _46(T46),
            _47(T47),
            _48(T48),
            _49(T49),
            _50(T50),
            _51(T51),
            _52(T52),
            _53(T53),
            _54(T54),
            _55(T55),
            _56(T56),
            _57(T57),
            _58(T58),
            _59(T59),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36, T37, T38, T39, T40, T41, T42, T43, T44, T45, T46, T47, T48, T49, T50, T51, T52, T53, T54, T55, T56, T57, T58, T59, T60> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
            _21(T21),
            _22(T22),
            _23(T23),
            _24(T24),
            _25(T25),
            _26(T26),
            _27(T27),
            _28(T28),
            _29(T29),
            _30(T30),
            _31(T31),
            _32(T32),
            _33(T33),
            _34(T34),
            _35(T35),
            _36(T36),
            _37(T37),
            _38(T38),
            _39(T39),
            _40(T40),
            _41(T41),
            _42(T42),
            _43(T43),
            _44(T44),
            _45(T45),
            _46(T46),
            _47(T47),
            _48(T48),
            _49(T49),
            _50(T50),
            _51(T51),
            _52(T52),
            _53(T53),
            _54(T54),
            _55(T55),
            _56(T56),
            _57(T57),
            _58(T58),
            _59(T59),
            _60(T60),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36, T37, T38, T39, T40, T41, T42, T43, T44, T45, T46, T47, T48, T49, T50, T51, T52, T53, T54, T55, T56, T57, T58, T59, T60, T61> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
            _21(T21),
            _22(T22),
            _23(T23),
            _24(T24),
            _25(T25),
            _26(T26),
            _27(T27),
            _28(T28),
            _29(T29),
            _30(T30),
            _31(T31),
            _32(T32),
            _33(T33),
            _34(T34),
            _35(T35),
            _36(T36),
            _37(T37),
            _38(T38),
            _39(T39),
            _40(T40),
            _41(T41),
            _42(T42),
            _43(T43),
            _44(T44),
            _45(T45),
            _46(T46),
            _47(T47),
            _48(T48),
            _49(T49),
            _50(T50),
            _51(T51),
            _52(T52),
            _53(T53),
            _54(T54),
            _55(T55),
            _56(T56),
            _57(T57),
            _58(T58),
            _59(T59),
            _60(T60),
            _61(T61),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36, T37, T38, T39, T40, T41, T42, T43, T44, T45, T46, T47, T48, T49, T50, T51, T52, T53, T54, T55, T56, T57, T58, T59, T60, T61, T62> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
            _21(T21),
            _22(T22),
            _23(T23),
            _24(T24),
            _25(T25),
            _26(T26),
            _27(T27),
            _28(T28),
            _29(T29),
            _30(T30),
            _31(T31),
            _32(T32),
            _33(T33),
            _34(T34),
            _35(T35),
            _36(T36),
            _37(T37),
            _38(T38),
            _39(T39),
            _40(T40),
            _41(T41),
            _42(T42),
            _43(T43),
            _44(T44),
            _45(T45),
            _46(T46),
            _47(T47),
            _48(T48),
            _49(T49),
            _50(T50),
            _51(T51),
            _52(T52),
            _53(T53),
            _54(T54),
            _55(T55),
            _56(T56),
            _57(T57),
            _58(T58),
            _59(T59),
            _60(T60),
            _61(T61),
            _62(T62),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36, T37, T38, T39, T40, T41, T42, T43, T44, T45, T46, T47, T48, T49, T50, T51, T52, T53, T54, T55, T56, T57, T58, T59, T60, T61, T62, T63> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
            _21(T21),
            _22(T22),
            _23(T23),
            _24(T24),
            _25(T25),
            _26(T26),
            _27(T27),
            _28(T28),
            _29(T29),
            _30(T30),
            _31(T31),
            _32(T32),
            _33(T33),
            _34(T34),
            _35(T35),
            _36(T36),
            _37(T37),
            _38(T38),
            _39(T39),
            _40(T40),
            _41(T41),
            _42(T42),
            _43(T43),
            _44(T44),
            _45(T45),
            _46(T46),
            _47(T47),
            _48(T48),
            _49(T49),
            _50(T50),
            _51(T51),
            _52(T52),
            _53(T53),
            _54(T54),
            _55(T55),
            _56(T56),
            _57(T57),
            _58(T58),
            _59(T59),
            _60(T60),
            _61(T61),
            _62(T62),
            _63(T63),
        }
    };
    (@output-type (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)) => {
        enum __SelectOutput<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36, T37, T38, T39, T40, T41, T42, T43, T44, T45, T46, T47, T48, T49, T50, T51, T52, T53, T54, T55, T56, T57, T58, T59, T60, T61, T62, T63, T64> {
            Completed,
            WouldBlock,
            _0(T0),
            _1(T1),
            _2(T2),
            _3(T3),
            _4(T4),
            _5(T5),
            _6(T6),
            _7(T7),
            _8(T8),
            _9(T9),
            _10(T10),
            _11(T11),
            _12(T12),
            _13(T13),
            _14(T14),
            _15(T15),
            _16(T16),
            _17(T17),
            _18(T18),
            _19(T19),
            _20(T20),
            _21(T21),
            _22(T22),
            _23(T23),
            _24(T24),
            _25(T25),
            _26(T26),
            _27(T27),
            _28(T28),
            _29(T29),
            _30(T30),
            _31(T31),
            _32(T32),
            _33(T33),
            _34(T34),
            _35(T35),
            _36(T36),
            _37(T37),
            _38(T38),
            _39(T39),
            _40(T40),
            _41(T41),
            _42(T42),
            _43(T43),
            _44(T44),
            _45(T45),
            _46(T46),
            _47(T47),
            _48(T48),
            _49(T49),
            _50(T50),
            _51(T51),
            _52(T52),
            _53(T53),
            _54(T54),
            _55(T55),
            _56(T56),
            _57(T57),
            _58(T58),
            _59(T59),
            _60(T60),
            _61(T61),
            _62(T62),
            _63(T63),
            _64(T64),
        }
    };

    // Entry points.
    ($($tokens:tt)*) => {
        $crate::select_internal!(@list ($($tokens)*) () () () ())
    }
}

#[cfg(test)]
mod tests;
