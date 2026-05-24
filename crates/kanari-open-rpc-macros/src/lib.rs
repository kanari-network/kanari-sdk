// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Attribute, Expr, ExprArray, ExprLit, ExprTuple, Item, ItemMod, Lit, LitBool, LitStr, Meta,
    Result, Token, parse::Parser, parse_macro_input,
};

#[proc_macro_attribute]
pub fn open_rpc(_attr: TokenStream, item: TokenStream) -> TokenStream {
    match expand_open_rpc(parse_macro_input!(item as ItemMod)) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

#[proc_macro_attribute]
pub fn open_rpc_method(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

fn expand_open_rpc(mut module: ItemMod) -> Result<proc_macro2::TokenStream> {
    let Some((brace, items)) = module.content.take() else {
        return Err(syn::Error::new_spanned(
            &module,
            "#[open_rpc] only supports inline modules",
        ));
    };

    let mut generated_methods = Vec::new();
    let mut rewritten_items = Vec::with_capacity(items.len());

    for item in items {
        match item {
            Item::Const(mut item_const) => {
                if let Some(method_attr) = take_open_rpc_method_attr(&mut item_const.attrs) {
                    let doc = parse_method_doc(method_attr)?;
                    let ident = item_const.ident.clone();
                    generated_methods.push(build_method_tokens(&ident, doc)?);
                }
                rewritten_items.push(Item::Const(item_const));
            }
            other => rewritten_items.push(other),
        }
    }

    rewritten_items.push(syn::parse_quote! {
        pub fn open_rpc_methods() -> ::std::vec::Vec<::kanari_open_rpc::MethodObject> {
            vec![#(#generated_methods),*]
        }
    });

    module.content = Some((brace, rewritten_items));
    Ok(quote!(#module))
}

fn take_open_rpc_method_attr(attrs: &mut Vec<Attribute>) -> Option<Attribute> {
    let index = attrs
        .iter()
        .position(|attr| attr.path().is_ident("open_rpc_method"))?;
    Some(attrs.remove(index))
}

struct MethodDoc {
    summary: LitStr,
    description: Option<LitStr>,
    params: ExprArray,
    result: ExprTuple,
    tags: ExprArray,
}

fn parse_method_doc(attr: Attribute) -> Result<MethodDoc> {
    let parser = syn::punctuated::Punctuated::<Meta, Token![,]>::parse_terminated;
    let metas = parser.parse2(attr.meta.require_list()?.tokens.clone())?;

    let mut summary = None;
    let mut description = None;
    let mut params = None;
    let mut result = None;
    let mut tags = None;

    for meta in metas {
        match meta {
            Meta::NameValue(name_value) if name_value.path.is_ident("summary") => {
                summary = Some(expect_string_literal(name_value.value, "summary")?);
            }
            Meta::NameValue(name_value) if name_value.path.is_ident("description") => {
                description = Some(expect_string_literal(name_value.value, "description")?);
            }
            Meta::NameValue(name_value) if name_value.path.is_ident("params") => {
                params = Some(expect_array(name_value.value, "params")?);
            }
            Meta::NameValue(name_value) if name_value.path.is_ident("result") => {
                result = Some(expect_tuple(name_value.value, "result")?);
            }
            Meta::NameValue(name_value) if name_value.path.is_ident("tags") => {
                tags = Some(expect_array(name_value.value, "tags")?);
            }
            other => {
                return Err(syn::Error::new_spanned(
                    other,
                    "unsupported open_rpc_method property",
                ));
            }
        }
    }

    Ok(MethodDoc {
        summary: summary.ok_or_else(|| syn::Error::new_spanned(&attr, "missing `summary`"))?,
        description,
        params: params.unwrap_or_else(empty_array),
        result: result.ok_or_else(|| syn::Error::new_spanned(&attr, "missing `result`"))?,
        tags: tags.unwrap_or_else(empty_array),
    })
}

fn empty_array() -> ExprArray {
    syn::parse_quote!([])
}

fn expect_string_literal(expr: Expr, field: &str) -> Result<LitStr> {
    match expr {
        Expr::Lit(ExprLit {
            lit: Lit::Str(value),
            ..
        }) => Ok(value),
        other => Err(syn::Error::new_spanned(
            other,
            format!("{field} must be a string literal"),
        )),
    }
}

fn expect_array(expr: Expr, field: &str) -> Result<ExprArray> {
    match expr {
        Expr::Array(array) => Ok(array),
        other => Err(syn::Error::new_spanned(
            other,
            format!("{field} must be an array expression"),
        )),
    }
}

fn expect_tuple(expr: Expr, field: &str) -> Result<ExprTuple> {
    match expr {
        Expr::Tuple(tuple) => Ok(tuple),
        other => Err(syn::Error::new_spanned(
            other,
            format!("{field} must be a tuple expression"),
        )),
    }
}

fn build_method_tokens(ident: &syn::Ident, doc: MethodDoc) -> Result<proc_macro2::TokenStream> {
    let summary = doc.summary;
    let description = doc
        .description
        .map(|description| quote!(Some(#description)))
        .unwrap_or_else(|| quote!(None));

    let params = doc
        .params
        .elems
        .into_iter()
        .map(build_param_tokens)
        .collect::<Result<Vec<_>>>()?;

    let result = build_result_tokens(doc.result)?;
    let tags = doc
        .tags
        .elems
        .into_iter()
        .map(|expr| match expr {
            Expr::Lit(ExprLit {
                lit: Lit::Str(value),
                ..
            }) => Ok(quote!(#value)),
            other => Err(syn::Error::new_spanned(
                other,
                "tag entries must be string literals",
            )),
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(quote! {
        ::kanari_open_rpc::method(
            self::#ident,
            #summary,
            #description,
            vec![#(#params),*],
            #result,
            &[#(#tags),*],
        )
    })
}

fn build_param_tokens(expr: Expr) -> Result<proc_macro2::TokenStream> {
    let Expr::Tuple(tuple) = expr else {
        return Err(syn::Error::new_spanned(
            expr,
            "each params entry must be a tuple",
        ));
    };

    if tuple.elems.len() != 4 {
        return Err(syn::Error::new_spanned(
            tuple,
            "each params entry must be (name, description, required, schema)",
        ));
    }

    let mut elems = tuple.elems.into_iter();
    let name = expect_string_literal(elems.next().unwrap(), "param name")?;
    let description = expect_string_literal(elems.next().unwrap(), "param description")?;
    let required = expect_bool_literal(elems.next().unwrap(), "param required")?;
    let schema = elems.next().unwrap();

    Ok(quote! {
        ::kanari_open_rpc::param(#name, #description, #required, #schema)
    })
}

fn build_result_tokens(tuple: ExprTuple) -> Result<proc_macro2::TokenStream> {
    if tuple.elems.len() != 3 {
        return Err(syn::Error::new_spanned(
            tuple,
            "result must be (name, description, schema)",
        ));
    }

    let mut elems = tuple.elems.into_iter();
    let name = expect_string_literal(elems.next().unwrap(), "result name")?;
    let description = expect_string_literal(elems.next().unwrap(), "result description")?;
    let schema = elems.next().unwrap();

    Ok(quote! {
        ::kanari_open_rpc::result(#name, #description, #schema)
    })
}

fn expect_bool_literal(expr: Expr, field: &str) -> Result<LitBool> {
    match expr {
        Expr::Lit(ExprLit {
            lit: Lit::Bool(value),
            ..
        }) => Ok(value),
        other => Err(syn::Error::new_spanned(
            other,
            format!("{field} must be a bool literal"),
        )),
    }
}
