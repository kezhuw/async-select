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
#[allow(dead_code)]
#[allow(unused_parens)]
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
