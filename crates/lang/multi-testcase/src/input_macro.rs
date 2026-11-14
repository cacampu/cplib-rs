use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Expr, Pat, Token, Type, bracketed, parenthesized};

mod keyword {
    syn::custom_keyword!(from);
    syn::custom_keyword!(with);
}

enum Kind {
    Value(Type),
    Array(Box<Kind>, Option<Expr>),
    Tuple(Vec<Kind>),
    Runtime(Expr),
}

impl Parse for Kind {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        if input.peek(keyword::with) {
            input.parse::<keyword::with>()?;
            return Ok(Self::Runtime(input.parse()?));
        }
        if input.peek(syn::token::Bracket) {
            let inner;
            bracketed!(inner in input);
            let kind = inner.parse()?;
            let length = if inner.peek(Token![;]) {
                inner.parse::<Token![;]>()?;
                Some(inner.parse()?)
            } else {
                None
            };
            if !inner.is_empty() {
                return Err(inner.error("expected [type; length] or [type]"));
            }
            return Ok(Self::Array(Box::new(kind), length));
        }
        if input.peek(syn::token::Paren) {
            let inner;
            parenthesized!(inner in input);
            let kinds = inner.parse_terminated(Self::parse, Token![,])?;
            return Ok(Self::Tuple(kinds.into_iter().collect()));
        }
        Ok(Self::Value(input.parse()?))
    }
}

impl Kind {
    fn expand(&self, runtime: &TokenStream, source: &syn::Ident) -> TokenStream {
        match self {
            Self::Value(ty) => quote!(<#ty as #runtime::Readable>::read(&mut #source)),
            Self::Runtime(reader) => quote!(#runtime::RuntimeReadable::read(#reader, &mut #source)),
            Self::Tuple(kinds) if kinds.is_empty() => quote!({}),
            Self::Tuple(kinds) => {
                let values = kinds.iter().map(|kind| kind.expand(runtime, source));
                quote!((#(#values,)*))
            }
            Self::Array(kind, length) => {
                let value = kind.expand(runtime, source);
                let length = length
                    .as_ref()
                    .map(|expr| quote!(#expr))
                    .unwrap_or_else(|| quote!(<usize as #runtime::Readable>::read(&mut #source)));
                quote!({
                    let __cplib_length: usize = #length;
                    (0..__cplib_length).map(|_| #value).collect::<::std::vec::Vec<_>>()
                })
            }
        }
    }
}

struct Binding {
    pattern: Pat,
    kind: Kind,
}

impl Parse for Binding {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let pattern = input.call(Pat::parse_single)?;
        input.parse::<Token![:]>()?;
        Ok(Self {
            pattern,
            kind: input.parse()?,
        })
    }
}

enum Body {
    Bindings(Vec<Binding>),
    Expression(Kind),
}

pub struct Input {
    source: Option<Expr>,
    body: Body,
}

impl Parse for Input {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let source = if input.peek(keyword::from) && !input.peek2(Token![:]) {
            input.parse::<keyword::from>()?;
            let source = input.parse()?;
            input.parse::<Token![,]>()?;
            Some(source)
        } else {
            None
        };
        let fork = input.fork();
        let declaration =
            fork.call(Pat::parse_single).is_ok() && fork.peek(Token![:]) && !fork.peek(Token![::]);
        let body = if input.is_empty() || declaration {
            let bindings = input.parse_terminated(Binding::parse, Token![,])?;
            Body::Bindings(bindings.into_iter().collect())
        } else {
            let kind = input.parse()?;
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
            Body::Expression(kind)
        };
        Ok(Self { source, body })
    }
}

impl Input {
    pub fn expand(self, runtime: &TokenStream) -> syn::Result<TokenStream> {
        let source = format_ident!("__cplib_source", span = Span::mixed_site());
        let handle = format_ident!("__cplib_input", span = Span::mixed_site());
        let setup = match &self.source {
            Some(expr) => quote!(
                let mut #handle = #expr;
                let mut #source = #runtime::InputSource::lock_source(&mut #handle);
            ),
            None => quote!(let mut #source = #runtime::INPUT.lock();),
        };
        Ok(match self.body {
            Body::Expression(kind) => {
                let value = kind.expand(runtime, &source);
                quote!({ #setup #value })
            }
            Body::Bindings(bindings) => {
                let declarations = bindings.iter().map(|binding| {
                    let pattern = &binding.pattern;
                    let value = binding.kind.expand(runtime, &source);
                    quote!(let #pattern = #value;)
                });
                quote!(#setup #(#declarations)* ::std::mem::drop(#source);)
            }
        })
    }
}
