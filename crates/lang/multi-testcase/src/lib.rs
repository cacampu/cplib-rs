//! `io` クレートから再公開する入力・実行用マクロ。
extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::{Span, TokenStream as Tokens};
use quote::quote;
use syn::{Ident, ItemFn, Path, ReturnType, Type, parse_macro_input};

mod input_macro;
mod output_macro;

fn expand_output(tokens: TokenStream, newline: bool, debug: bool) -> TokenStream {
    match output_macro::expand(tokens.into(), newline, debug) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

/// 値を Printable で出力する。先頭が文字列リテラルなら std の書式指定として扱う。
#[proc_macro]
pub fn out(tokens: TokenStream) -> TokenStream {
    expand_output(tokens, false, false)
}

/// out! と同じ形式で末尾に改行を出力する。
#[proc_macro]
pub fn outln(tokens: TokenStream) -> TokenStream {
    expand_output(tokens, true, false)
}

/// debug ビルド時だけ、同じ出力形式で stderr へ改行付きで出力する。
#[proc_macro]
pub fn debug(tokens: TokenStream) -> TokenStream {
    expand_output(tokens, true, true)
}

fn runtime_path() -> syn::Result<Tokens> {
    match crate_name("io") {
        Ok(FoundCrate::Itself) => Ok(quote!(io)),
        Ok(FoundCrate::Name(name)) => {
            let ident = Ident::new(&name, Span::call_site());
            Ok(quote!(#ident))
        }
        Err(error) => Err(syn::Error::new(
            Span::call_site(),
            format!("add the cplib `io` package as a dependency: {error}"),
        )),
    }
}

fn attribute_runtime(attr: TokenStream) -> syn::Result<Tokens> {
    if attr.is_empty() {
        return runtime_path();
    }
    struct Runtime(Path);
    impl syn::parse::Parse for Runtime {
        fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
            input.parse::<syn::Token![crate]>()?;
            input.parse::<syn::Token![=]>()?;
            Ok(Self(input.parse()?))
        }
    }
    let Runtime(path) = syn::parse(attr)?;
    Ok(quote!(#path))
}

/// proconio 風の宣言形式と、`input!(usize)` の式形式。
#[proc_macro]
pub fn input(tokens: TokenStream) -> TokenStream {
    let input = parse_macro_input!(tokens as input_macro::Input);
    match runtime_path().and_then(|runtime| input.expand(&runtime)) {
        Ok(output) => output.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

/// `input!(type)` と同じ式形式。既存の proconio 風コードからの移行用。
#[proc_macro]
pub fn read_value(tokens: TokenStream) -> TokenStream {
    input(tokens)
}

/// 関数の終了時に全関数で共有する出力バッファを flush する。
#[proc_macro_attribute]
pub fn fastout(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut function = parse_macro_input!(item as ItemFn);
    let runtime = match attribute_runtime(attr) {
        Ok(path) => path,
        Err(error) => return error.into_compile_error().into(),
    };
    if function.sig.asyncness.is_some() || function.sig.constness.is_some() {
        return syn::Error::new_spanned(
            &function.sig,
            "fastout requires a non-async, non-const function",
        )
        .into_compile_error()
        .into();
    }
    let body = function.block;
    function.block = syn::parse_quote!({
        let _cplib_flush_guard = #runtime::flush_guard();
        #body
    });
    quote!(#function).into()
}

/// 先頭のケース数を共通スキャナから読み、関数本体をケースごとに呼ぶ。
/// 本体の `return` は現在のケースだけを終了する。関数の返り値は `()`。
#[proc_macro_attribute]
pub fn multi_testcase(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut function = parse_macro_input!(item as ItemFn);
    let runtime = match attribute_runtime(attr) {
        Ok(path) => path,
        Err(error) => return error.into_compile_error().into(),
    };
    let unit = match &function.sig.output {
        ReturnType::Default => true,
        ReturnType::Type(_, ty) => matches!(&**ty, Type::Tuple(tuple) if tuple.elems.is_empty()),
    };
    if !unit || function.sig.asyncness.is_some() || function.sig.constness.is_some() {
        return syn::Error::new_spanned(
            &function.sig,
            "multi_testcase requires a non-async, non-const function returning ()",
        )
        .into_compile_error()
        .into();
    }
    let body = function.block;
    function.block = syn::parse_quote!({
        let _cplib_flush_guard = #runtime::flush_guard();
        let __cplib_cases = #runtime::read::<usize>();
        #[allow(unused_mut)]
        let mut __cplib_case = || #body;
        for _ in 0..__cplib_cases {
            __cplib_case();
        }
    });
    quote!(#function).into()
}
