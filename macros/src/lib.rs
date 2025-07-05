//! Procedural macros for `select!`.

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::{Expr, Ident, Index, Pat, Result, Token};

mod kw {
    syn::custom_keyword!(complete);
}

struct Clause {
    expr: Expr,
}

impl Parse for Clause {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        input.parse::<Token![=>]>()?;
        let expr = Expr::parse_with_earlier_boundary_rule(input)?;
        if matches!(expr, Expr::Block(_)) {
            input.parse::<Option<Token![,]>>()?;
        } else if !input.is_empty() {
            input.parse::<Token![,]>()?;
        }
        Ok(Clause { expr })
    }
}

impl ToTokens for Clause {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.expr.to_tokens(tokens)
    }
}

struct Condition {
    expr: Expr,
}

impl Parse for Condition {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        input.parse::<Token![,]>()?;
        input.parse::<Token![if]>()?;
        let expr = Expr::parse_without_eager_brace(input)?;
        Ok(Condition { expr })
    }
}

impl ToTokens for Condition {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.expr.to_tokens(tokens)
    }
}

struct Branch {
    bind: Pat,
    check: Pat,
    future: Expr,
    condition: Option<Condition>,
    clause: Clause,
}

impl Branch {
    fn conditional_future(&self) -> ConditionalFuture<'_> {
        ConditionalFuture { future: &self.future, condition: self.condition.as_ref() }
    }
}

struct ConditionalFuture<'a> {
    future: &'a Expr,
    condition: Option<&'a Condition>,
}

impl ToTokens for ConditionalFuture<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let future = self.future;
        match self.condition {
            None => quote! { ::core::option::Option::Some(#future) },
            Some(condition) => quote! { if #condition { ::core::option::Option::Some(#future) } else { None } },
        }
        .to_tokens(tokens);
    }
}

#[derive(Default)]
struct Select {
    default_clause: Option<Clause>,
    complete_clause: Option<Clause>,
    branches: Vec<Branch>,
}

// This is mainly copied from https://github.com/tokio-rs/tokio/blob/tokio-1.46.1/tokio-macros/src/select.rs#L58
//
// See the LICENSE: https://github.com/tokio-rs/tokio/blob/tokio-1.46.1/LICENSE
fn clean_pattern(pat: &mut Pat) {
    match pat {
        syn::Pat::Ident(ident) => {
            ident.by_ref = None;
            ident.mutability = None;
            if let Some((_at, pat)) = &mut ident.subpat {
                clean_pattern(&mut *pat);
            }
        },
        syn::Pat::Or(or) => {
            for case in &mut or.cases {
                clean_pattern(case);
            }
        },
        syn::Pat::Slice(slice) => {
            for elem in &mut slice.elems {
                clean_pattern(elem);
            }
        },
        syn::Pat::Struct(struct_pat) => {
            for field in &mut struct_pat.fields {
                clean_pattern(&mut field.pat);
            }
        },
        syn::Pat::Tuple(tuple) => {
            for elem in &mut tuple.elems {
                clean_pattern(elem);
            }
        },
        syn::Pat::TupleStruct(tuple) => {
            for elem in &mut tuple.elems {
                clean_pattern(elem);
            }
        },
        syn::Pat::Reference(reference) => {
            reference.mutability = None;
            clean_pattern(&mut reference.pat);
        },
        syn::Pat::Type(type_pat) => {
            clean_pattern(&mut type_pat.pat);
        },
        _ => {},
    };
}

fn to_check_pat(pat: &Pat) -> Pat {
    let mut pat = pat.clone();
    clean_pattern(&mut pat);
    pat
}

impl Parse for Select {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut select = Select::default();
        while !input.is_empty() {
            if input.peek(Token![default]) && input.peek2(Token![=>]) {
                if select.default_clause.is_some() {
                    return Err(input.error("`select!`: more than one `default` clauses"));
                }
                input.parse::<Token![default]>()?;
                let clause = Clause::parse(input)?;
                select.default_clause = Some(clause);
            } else if input.peek(kw::complete) && input.peek2(Token![=>]) {
                if select.complete_clause.is_some() {
                    return Err(input.error("`select!`: more than one `complete` clauses"));
                }
                input.parse::<kw::complete>()?;
                let clause = Clause::parse(input)?;
                select.complete_clause = Some(clause);
            } else {
                let bind = Pat::parse_multi(input)?;
                input.parse::<Token![=]>()?;
                let future = input.parse::<Expr>()?;
                let condition = if input.peek(Token![,]) { Some(input.parse::<Condition>()?) } else { None };
                let clause = Clause::parse(input)?;
                let check = to_check_pat(&bind);
                select.branches.push(Branch { bind, check, future, condition, clause });
            }
        }
        match (select.branches.is_empty(), select.complete_clause.is_some(), select.default_clause.is_some()) {
            (true, false, false) => return Err(input.error("`select!`: no branch")),
            (true, false, true) => return Err(input.error("`select!`: no branch except `default`")),
            (true, true, false) => return Err(input.error("`select!`: no branch except `complete`")),
            (true, true, true) => return Err(input.error("`select!`: no branch except `default` and `complete`")),
            (_, _, _) => {},
        };
        Ok(select)
    }
}

fn define_output_enum(ident: &Ident, branches: usize, span: Span) -> (Vec<Ident>, TokenStream) {
    let type_names: Vec<_> = (0..branches).map(|i| format_ident!("T{i}", span = span)).collect();
    let branch_names: Vec<_> = (0..branches).map(|i| format_ident!("_{i}", span = span)).collect();
    let output_enum = quote! {
        enum #ident<#(#type_names,)*> {
            Completed,
            WouldBlock,
            #(
                #branch_names(#type_names),
            )*
        };
    };
    (branch_names, output_enum)
}

fn select_internal(input: proc_macro::TokenStream, biased: bool) -> proc_macro::TokenStream {
    let select = syn::parse_macro_input!(input as Select);
    let span = Span::call_site();
    let output_ident = Ident::new("__SelectOutput", span);
    let (branch_names, output_enum) = define_output_enum(&output_ident, select.branches.len(), span);

    let branch_futures = select.branches.iter().map(|branch| branch.conditional_future());

    let select_futures_declartion = quote! {
        let mut __select_futures = (#(#branch_futures,)*);
        // Shadow it so it won't be moved accidentally.
        let mut __select_futures = &mut __select_futures;
    };

    let default_handler = match select.default_clause.as_ref() {
        None => quote! { ::core::unreachable!("not in unblocking mode") },
        Some(clause) => quote! { #clause },
    };

    let complete_handler = match select.complete_clause.as_ref() {
        None => quote! {
            ::core::panic!("all branches are disabled or completed and there is no `default` nor `complete`")
        },
        Some(clause) => quote! { #clause },
    };

    let (pending_declaration, pending_assignment, pending_check) =
        match select.complete_clause.is_some() || select.default_clause.is_none() {
            true => (
                quote! {
                    let mut any_pending = false;
                },
                quote! {
                    any_pending = true;
                },
                quote! {
                    if !any_pending {
                        return ::core::task::Poll::Ready(__SelectOutput::Completed);
                    }
                },
            ),
            false => (quote! {}, quote! {}, quote! {}),
        };
    let default_clause = match select.default_clause.is_some() {
        true => quote! { ::core::task::Poll::Ready(__SelectOutput::WouldBlock) },
        false => quote! { ::core::task::Poll::Pending },
    };

    let (biased_start, biased_branch) = match biased {
        true => (quote! {}, quote! { let branch = i; }),
        false => (
            quote! {
                let start = (&__select_futures as *const _ as usize) >> 3;
            },
            quote! {
                #[allow(clippy::modulo_one)]
                let branch = (start +i ) % BRANCHES;
            },
        ),
    };

    let branch_handlers = select.branches.iter().map(|branch| &branch.clause);
    let branch_bindings = select.branches.iter().map(|branch| &branch.bind);
    let branch_binding_checks = select.branches.iter().map(|branch| &branch.check);

    let n_branches = select.branches.len();
    let branch_indices = (0..n_branches).map(Index::from);

    quote! {{
        #output_enum
        const BRANCHES: usize = #n_branches;
        let mut output = {
            #select_futures_declartion
            ::core::future::poll_fn(|cx| {
                #biased_start
                #pending_declaration
                for i in 0..BRANCHES {
                    #biased_branch
                    match branch {
                        #(
                            #branch_indices => {
                                let ::core::option::Option::Some(future) = __select_futures.#branch_indices.as_mut() else {
                                    continue;
                                };
                                #[allow(unused_unsafe)]
                                let future = unsafe {
                                    ::core::pin::Pin::new_unchecked(future)
                                };
                                let mut output = match ::core::future::Future::poll(
                                    future,
                                    cx,
                                ) {
                                    ::core::task::Poll::Ready(output) => output,
                                    ::core::task::Poll::Pending => {
                                        #pending_assignment
                                        continue;
                                    },
                                };
                                __select_futures.#branch_indices = ::core::option::Option::None;
                                #[allow(unreachable_patterns)]
                                #[allow(unused_variables)]
                                match &output {
                                    #branch_binding_checks => {},
                                    _ => continue,
                                };
                                return ::core::task::Poll::Ready(__SelectOutput::#branch_names(output));
                            }
                        )*
                            _ => ::core::unreachable!("select! encounter mismatch branch in polling"),
                    }
                }
                #pending_check
                #default_clause
            }).await
        };
        match output {
            __SelectOutput::WouldBlock => #default_handler,
            __SelectOutput::Completed => #complete_handler,
            #(
                __SelectOutput::#branch_names(#branch_bindings) => #branch_handlers,
            )*
            #[allow(unreachable_patterns)] // In case of refutable patterns in branches
            _ => ::core::unreachable!("select! fail to pattern match"),
        }
    }}.into()
}

#[proc_macro]
pub fn select_default(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    select_internal(input, false)
}

#[proc_macro]
pub fn select_biased(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    select_internal(input, true)
}
