# async-select

[![crates.io](https://img.shields.io/crates/v/async-select)](https://crates.io/crates/async-select)
[![github-ci](https://github.com/kezhuw/async-select/actions/workflows/ci.yml/badge.svg?event=push)](https://github.com/kezhuw/async-select/actions)
[![docs.rs](https://img.shields.io/docsrs/async-select)](https://docs.rs/async-select)
[![Apache-2.0](https://img.shields.io/github/license/kezhuw/async-select)](LICENSE)

`select!` multiplex asynchronous futures simultaneously.

## Motivations
Initially, I opened an issue [Support multiple async runtimes but not only tokio](https://github.com/kezhuw/zookeeper-client-rust/issues/26) for [zookeeper-client-rust](https://github.com/kezhuw/zookeeper-client-rust). I saw, there are other projects in community to assist runtime agnostic libraries. Though, I wouldn't say possibly most of they if not all are horrible -:), but I wouldn't they are elegant either. After created [spawns](https://docs.rs/spawns), I found that it is actually extremely easy to migrate from tokio runtime for library authors with help from runtime agnostic libraries, say `async-io`, `async-net`, `futures`, `futures-rustls`, `futures-lite` and etc.. But there are still tokio dependencies in tree, one is `select!`, I want to give it a try.

## Features
```rust
use core::future::ready;

use async_select::select;

#[derive(Default)]
struct FieldStruct {
    _a: TupleStruct,
    _b: i32,
    _c: i32,
}

#[derive(Default)]
struct TupleStruct((), (), ());

// pattern according to syn::Pat
//
// failure will cause compilation error.
async fn patterns() {
    select! {
        default => {},
        complete => {},

        // Const(PatConst)
        //
        // unstable: #![feature(inline_const_pat)] https://github.com/rust-lang/rust/issues/76001
        // const { 5 } = ready(5) => {},

        // Ident(PatIdent)
        mut _v = ready(()) => {},
        ref _v = ready(()) => {},
        ref mut _v = ready(()) => {},
        ref mut _x@FieldStruct{ _b, ..} = ready(FieldStruct::default()) => {},
        ref mut _x@FieldStruct{ mut _b, ..} = ready(FieldStruct::default()) => {},
        mut _x@FieldStruct{ mut _b, ..} = ready(FieldStruct::default()) => {},


        // Lit(PatLit)
        5 = ready(5) => {},

        // Macro(PatMacro)

        // Or(PatOr)
        5 | 6 = ready(5) => {},

        // Paren(PatParen)
        (5 | 6) = ready(5) => {},

        // Path(PatPath)
        ::core::option::Option::None = ready(Some(5)) => {},
        ::core::option::Option::Some(ref _i) = ready(Some(5)) => {},

        // Range(PatRange)
        1..=2 = ready(5) => {},

        // Reference(PatReference)
        //
        // This is not supported as we are pattern against value.
        // &_v = ready(5) => {}
        // &mut _v = ready(5) => {}

        // Rest(PatRest)
        (ref _i, mut _v, ..) = ready((1, 2, 3, 4)) => {},

        // Slice(PatSlice)
        //
        // Pattern against value but not reference.

        // Struct(PatStruct)
        FieldStruct { ref mut _a, ref _b, .. } = ready(FieldStruct::default()) => {},

        // Tuple(PatTuple)
        (1, 2) = ready((1, 2)) => {},

        // TupleStruct(PatTupleStruct)
        TupleStruct(_a, _b, ..) = ready(TupleStruct::default()) => {},
        TupleStruct(ref mut _a, ref _b, ..) = ready(TupleStruct::default()) => {},

        // Type(PatType)
        // Is this only used in variable definition ?

        // Verbatim(TokenStream)
        //
        // Tokens in pattern position not interpreted by Syn.

        // Wild(PatWild)
        _ = ready(()) => {}
    }
}
```

## Links
* [crossbeam-channel](https://docs.rs/crossbeam/0.8.4/crossbeam/channel/macro.select.html): this is where I learned what real marcos look like.
* [stuck::select](https://docs.rs/stuck/0.4.0/stuck/macro.select.html): this is where `async-select::select!` derive from.
* [tokio](https://docs.rs/tokio): tokio is great on it own. But it is apprently not kind to runtime agnostic library. You simply can't tell which part of it is runtime agnostic. So better to avoid it entirely if you even want runtime agnostic.
* [The Little Book of Rust Macros](https://veykril.github.io/tlborm/decl-macros/macros-methodical.html): hmm, the book.
