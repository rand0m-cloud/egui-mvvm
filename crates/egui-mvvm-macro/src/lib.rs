mod view;
mod viewmodel;

use crate::view::is_view_attr;
use crate::viewmodel::{is_viewmodel_attr, ViewModelMacroInput};
use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{parse_macro_input, Attribute, Error};
use view::ViewMacroInput;

struct MacroInput {
    views: Vec<ViewMacroInput>,
    viewmodels: Vec<ViewModelMacroInput>,
}

#[proc_macro]
pub fn view_model(input: TokenStream) -> TokenStream {
    let MacroInput { views, viewmodels } = parse_macro_input!(input as MacroInput);

    quote! {
       #(#views)*

       #(#viewmodels)*
    }
    .into()
}

impl Parse for MacroInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut views = Vec::new();
        let mut viewmodels = Vec::new();

        while !input.is_empty() {
            let (span, is_view, is_viewmodel) = {
                // Keep the original cursor, we only check for the #[view] and #[viewmodel].
                let test_buf = input.fork();

                let attrs: Vec<Attribute> =
                    test_buf.call(Attribute::parse_outer).unwrap_or_default();

                let is_view = attrs.iter().any(|attr| is_view_attr(attr).is_some());
                let is_viewmodel = attrs.iter().any(|attr| is_viewmodel_attr(attr).is_some());
                let span = test_buf.span();
                (span, is_view, is_viewmodel)
            };

            match (is_view, is_viewmodel) {
                (true, true) => {
                    return Err(Error::new(span, "item cannot be a view and a viewmodel"));
                }
                (false, false) => {
                    return Err(Error::new(span, "item is missing #[view] or #[viewmodel]"));
                }
                (true, false) => views.push(input.parse()?),
                (false, true) => viewmodels.push(input.parse()?),
            }
        }

        Ok(MacroInput { views, viewmodels })
    }
}
