use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::{Attribute, ItemStruct, Meta, Type};

pub fn is_view_attr(attr: &Attribute) -> Option<()> {
    is_view_meta(&attr.meta)
}

pub fn is_view_meta(meta: &Meta) -> Option<()> {
    meta.path()
        .get_ident()
        .is_some_and(|i| i == "view")
        .then_some(())
}

fn is_viewmodel_field_attr(attr: &Attribute) -> bool {
    attr.path().get_ident().is_some_and(|i| i == "viewmodel")
}

pub struct ViewMacroInput {
    item_struct: ItemStruct,
}

impl Parse for ViewMacroInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let item_struct: ItemStruct = input.parse()?;
        Ok(Self { item_struct })
    }
}

impl ToTokens for ViewMacroInput {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let mut item = self.item_struct.clone();

        assert_eq!(
            item.generics.lifetimes().count(),
            0,
            "todo: support view structs with lifetimes"
        );

        // Add a 'viewmodel lifetime to the struct.
        item.generics
            .params
            .push(syn::parse2(quote! { 'viewmodel }).unwrap());

        for field in &mut item.fields {
            // We want to modify fields with #[viewmodel].
            if !field.attrs.iter().any(is_viewmodel_field_attr) {
                continue;
            }

            // Remove the #[viewmodel] attr.
            field.attrs.retain(|attr| !is_viewmodel_field_attr(attr));

            match field.ty.clone() {
                Type::Reference(tyref) => {
                    let view_model = tyref.elem;
                    assert!(
                        tyref.lifetime.is_none(),
                        "todo: support view structs with lifetimes"
                    );
                    if tyref.mutability.is_some() {
                        field.ty = syn::parse2(quote! { egui_mvvm::view_model::ViewModelMutRef<'viewmodel, #view_model> }).unwrap();
                    } else {
                        field.ty = syn::parse2(
                            quote! { egui_mvvm::view_model::ViewModelRef<'viewmodel, #view_model> },
                        )
                        .unwrap();
                    };
                }
                _ => {
                    panic!("#[viewmodel] field in a #[view] should be a &VM or &mut VM");
                }
            }
        }

        let final_item = ItemStruct {
            attrs: item
                .attrs
                .iter()
                .filter(|&attr| is_view_attr(attr).is_none())
                .cloned()
                .collect(),
            ..item
        };

        final_item.to_tokens(tokens);
    }
}
