extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, parse_macro_input};

#[proc_macro_attribute]
pub fn multi_testcase(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);

    let attrs = &input_fn.attrs;
    let vis = &input_fn.vis;
    let sig = &input_fn.sig; // 関数名や引数などのシグネチャ
    let block = &input_fn.block; // 関数の中身({ ... })

    let expanded = quote! {
        #(#attrs)*
        #vis #sig {
            let mut __t_str = String::new();
            std::io::stdin().read_line(&mut __t_str).unwrap();
            let __t: usize = __t_str.trim().parse().unwrap();

            for _ in 0..__t {
                #block
            }
        }
    };

    TokenStream::from(expanded)
}
