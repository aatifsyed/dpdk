use proc_macro2::{Span, TokenStream};
use quote::ToTokens as _;
use syn::{
    parse::Nothing, parse_macro_input, token, AttrStyle, Attribute, ImplItem, ItemImpl, Meta, Path,
    Token,
};

/// Accept an `impl` block, and add a `#[no_mangle]` attribute to each of the member
#[proc_macro_attribute]
pub fn no_mangle(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let config = parse_macro_input!(attr as Nothing);
    let item = parse_macro_input!(item as ItemImpl);
    expand(config, item)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn expand(_: Nothing, mut item: ItemImpl) -> syn::Result<TokenStream> {
    let span = Span::call_site();
    for item in &mut item.items {
        if let ImplItem::Fn(it) = item {
            if !it.attrs.iter().any(|it| it.path().is_ident("no_mangle")) {
                it.attrs.push(Attribute {
                    pound_token: Token![#](span),
                    style: AttrStyle::Outer,
                    bracket_token: token::Bracket(span),
                    meta: Meta::Path(Path::from(syn::Ident::new("no_mangle", span))),
                });
            }
        }
    }
    Ok(item.into_token_stream())
}
