use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Expr, LitStr, Token};

enum Output {
    Format(TokenStream),
    Values(Vec<Expr>),
}

impl Parse for Output {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        if input.peek(LitStr) {
            return Ok(Self::Format(input.parse()?));
        }
        let values = input.parse_terminated(Expr::parse, Token![,])?;
        Ok(Self::Values(values.into_iter().collect()))
    }
}

pub fn expand(tokens: TokenStream, newline: bool, debug: bool) -> syn::Result<TokenStream> {
    let runtime = super::runtime_path()?;
    let args = match syn::parse2::<Output>(tokens)? {
        Output::Format(tokens) => quote!(::std::format_args!(#tokens)),
        Output::Values(values) => {
            let format = LitStr::new(&vec!["{}"; values.len()].join(" "), Span::call_site());
            quote!(::std::format_args!(#format, #(#runtime::printable(&(#values))),*))
        }
    };
    Ok(if debug {
        quote!(if ::std::cfg!(debug_assertions) { #runtime::__debug_args(#args); })
    } else if newline {
        // 改行は実行時に付ける。cargo-equip のマクロ展開で生成リテラルの
        // バックスラッシュが二重にエスケープされる問題も避けられる。
        quote!(#runtime::__print_args_ln(#args))
    } else {
        quote!(#runtime::__print_args(#args))
    })
}
