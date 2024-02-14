extern crate proc_macro;
use std::fmt::Display;

use expander::{Expander, Edition};
use litrs::StringLit;
use proc_macro::TokenStream;
use proc_macro2::Ident;
use syn::{parse::Parse, parse_macro_input, LitStr, Token};
use wars_core::Opts;
fn to_compile_error(msg: impl Display) -> TokenStream {
    let msg = format!("{msg}");
    return quote::quote! { compile_error!(#msg) }.into();
}
struct z {
    l: LitStr,
    m: Token![=>],
    b: Ident,
    n: Option<LitStr>
}
impl Parse for z {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        return Ok(z {
            l: input.parse()?,
            m: input.parse()?,
            b: input.parse()?,
            n: input.parse()?,
        });
    }
}

#[proc_macro]
pub fn wars(input: TokenStream) -> TokenStream {
    // if not, a relevant error message will be generated.
    let input = parse_macro_input!(input as z);

    // get value of the string literal.
    let str_value = input.l.value();
    let f = std::fs::read(str_value);
    let f = match f {
        Err(e) => return to_compile_error(e),
        Ok(lit) => lit,
    };
    let t = wars_core::lower(&f, input.b,input.n.clone().map(|a|Opts{r#async: a.value().contains("a"),result: a.value().contains("r")}).unwrap_or(Opts{r#async: false,result: false}));
    let t = match t {
        Ok(t) => t,
        Err(e) => return to_compile_error(e),
    };
    eprintln!("done");
    // let expanded = Expander::new("wasm")
    //     .add_comment("This is generated code!".to_owned())
    //     .fmt(Edition::_2021)
    //     .verbose(true)
    //     // common way of gating this, by making it part of the default feature set
    //     .dry(false)
    //     .write_to_out_dir(t.clone())
    //     .unwrap_or_else(|e| {
    //         eprintln!("Failed to write to file: {:?}", e);
    //         t
    //     });
    // eprintln!("{}",t);
    return t.into();
}

#[cfg(test)]
mod tests {
    use super::*;
}
