use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::{Pair, Punctuated};
use syn::token::{Brace, Comma, Semi};
use syn::{
    braced, Attribute, Expr, Field, FieldMutability, Fields, FieldsNamed, Generics, ItemStruct,
    Meta, Path, Token, Type, Visibility,
};

pub struct ViewModelAttr {
    default: bool,
}

pub fn is_viewmodel_attr(attr: &Attribute) -> Option<ViewModelAttr> {
    is_viewmodel_meta(&attr.meta)
}

pub fn is_viewmodel_meta(meta: &Meta) -> Option<ViewModelAttr> {
    let is_viewmodel_path = |path: &Path| path.get_ident().is_some_and(|i| i == "viewmodel");

    match meta {
        Meta::Path(p) if is_viewmodel_path(p) => Some(ViewModelAttr { default: false }),
        Meta::List(l) if is_viewmodel_path(&l.path) => {
            if l.parse_args::<syn::Ident>().unwrap() == "default" {
                Some(ViewModelAttr { default: true })
            } else {
                panic!("unexpected values for #[viewmodel], only support #[viewmodel(default)] for now");
            }
        }
        _ => None,
    }
}

#[derive(Clone)]
pub struct ViewModelMacroInput {
    attrs: Vec<Attribute>,
    vis: Visibility,
    struct_token: Token![struct],
    ident: syn::Ident,
    generics: Generics,
    fields: ViewModelFields,
    semi_token: Option<Semi>,
}

#[derive(Clone)]
pub struct ViewModelFields {
    brace_token: Brace,
    named: Punctuated<ViewModelField, Comma>,
}

#[derive(Clone)]
pub struct ViewModelField {
    attrs: Vec<Attribute>,
    vis: Visibility,
    mutability: FieldMutability,
    ident: syn::Ident,
    colon_token: Option<Token![:]>,
    ty: Type,
    default_value: Option<ViewModelFieldDefault>,
}

#[derive(Clone)]
pub struct ViewModelFieldDefault {
    #[allow(unused)]
    eq_token: Token![=],
    expr: Expr,
}

impl Parse for ViewModelMacroInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            attrs: input.call(Attribute::parse_outer)?,
            vis: input.parse()?,
            struct_token: input.parse()?,
            ident: input.parse()?,
            generics: input.parse()?,
            fields: input.parse()?,
            semi_token: input.parse()?,
        })
    }
}

impl Parse for ViewModelFields {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            brace_token: braced!(content in input),
            named: content.parse_terminated(ViewModelField::parse, Token![,])?,
        })
    }
}

impl Parse for ViewModelField {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            attrs: input.call(Attribute::parse_outer)?,
            vis: input.parse()?,
            mutability: FieldMutability::None,
            ident: input.parse()?,
            colon_token: input.parse()?,
            ty: input.parse()?,
            default_value: input.parse::<ViewModelFieldDefault>().ok(),
        })
    }
}

impl Parse for ViewModelFieldDefault {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            eq_token: input.parse()?,
            expr: input.parse()?,
        })
    }
}

impl ToTokens for ViewModelMacroInput {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ViewModelMacroInput {
            attrs,
            vis,
            struct_token,
            ident,
            generics,
            fields,
            semi_token,
        } = self.clone();

        let mut item = ItemStruct {
            attrs,
            vis,
            struct_token,
            ident,
            generics,
            fields: fields.into_fields(),
            semi_token,
        };
        let ident = &item.ident;
        let change = format_ident!("{}ChangeDetector", self.ident);
        let model = format_ident!("{}Model", self.ident);
        let vis = &self.vis;
        let default = self
            .attrs
            .iter()
            .find_map(|attr| Some(is_viewmodel_attr(attr)?.default))
            .unwrap_or_default();

        match &mut item.fields {
            Fields::Named(fields) => fields.named.push(
                syn::parse2::<FieldsNamed>(quote!( { task_pool: egui_mvvm::task_pool::TaskPool } ))
                    .unwrap()
                    .named
                    .into_iter()
                    .next()
                    .unwrap(),
            ),
            Fields::Unnamed(_) => panic!("cannot work with unnamed fields"),
            Fields::Unit => panic!("cannot work with unit structs"),
        }

        let mut change_fields = Vec::new();
        let mut model_fields = Vec::new();

        for field in self.fields.named.iter() {
            let ViewModelField { vis, ident, ty, .. } = &field;
            let ident = ident.clone();

            change_fields.push(quote! {
                #ident: <#ty as egui_mvvm::Stateful>::ChangeDetector
            });

            model_fields.push(quote! {
                #vis #ident: <#ty as egui_mvvm::Stateful>::Handle
            });
        }

        let change_struct_literal = {
            let mut fields = vec![];
            for field in self.fields.named.iter() {
                let ident = &field.ident;
                fields.push(quote! { #ident: self.#ident.change_detector() })
            }

            quote! { #change { #(#fields),* } }
        };

        let model_struct_literal = {
            let mut fields = vec![];
            for field in self.fields.named.iter() {
                let ident = &field.ident;
                fields.push(quote! { #ident: self.#ident.handle() })
            }

            quote! { #model { #(#fields),* } }
        };

        let change_detector_impl = {
            let mut select_arms = vec![];
            for field in self.fields.named.iter() {
                let ident = &field.ident;
                select_arms.push(quote! { res = this.#ident.wait_for_change() => res })
            }

            quote! {
                let this = self.clone();
                Box::pin(async move {
                    tokio::select! {
                        #(#select_arms),*
                    }
                })
            }
        };

        let latch_state_impl = {
            let mut fields = vec![];
            for field in self.fields.named.iter() {
                let ident = &field.ident;
                fields.push(quote! { self.#ident.latch_value(); })
            }

            quote! {
                #(#fields)*
            }
        };

        let default_impl = {
            if !default {
                quote! {}
            } else {
                let defaults = self.fields.as_default_fields();
                quote! {
                    impl Default for #ident {
                        fn default() -> #ident {
                            #ident {
                                #defaults
                            }
                        }
                    }
                }
            }
        };

        let item_sub_viewmodel_attr = ItemStruct {
            attrs: item
                .attrs
                .clone()
                .into_iter()
                .filter(|attr| is_viewmodel_attr(attr).is_none())
                .collect(),
            ..item.clone()
        };
        tokens.extend(quote! {
           #item_sub_viewmodel_attr

           impl egui_mvvm::view_model::ViewModelLike for #ident {
               fn latch_state(&mut self) {
                   #latch_state_impl
               }

               fn change_detector_boxed(&self) -> Box<dyn egui_mvvm::ChangeDetector> {
                   Box::new(self.change_detector())
               }
           }


           impl egui_mvvm::view_model::ViewModelTaskPool for #ident {
                fn task_pool(&self) -> egui_mvvm::task_pool::TaskPool {
                    self.task_pool.clone()
                }
            }

           #[derive(Clone)]
           #vis struct #change {
                #(#change_fields),*
           }

           impl egui_mvvm::ChangeDetector for #change {
               fn wait_for_change(&self) -> core::pin::Pin<Box<dyn Future<Output = Option<()>> + Send + 'static>> {
                    #change_detector_impl
               }
           }

           #vis struct #model {
               #(#model_fields),*
           }

           #default_impl

           impl egui_mvvm::Stateful for #ident {
               type ChangeDetector = #change;
               type Handle = egui_mvvm::view_model::ViewModelHandle<#ident>;
           }

           impl egui_mvvm::view_model::ViewModel for #ident {
                type Model = #model;
                type ChangeDetector = #change;

                fn make_model(&self) -> Self::Model {
                    #model_struct_literal
                }

                fn change_detector(&self) -> Self::ChangeDetector {
                    #change_struct_literal
                }
           }
        })
    }
}

impl ViewModelFields {
    pub fn into_fields(self) -> Fields {
        let Self { brace_token, named } = self;
        Fields::Named(FieldsNamed {
            brace_token,
            named: named
                .into_pairs()
                .map(|x| {
                    let (vm_field, comma) = x.into_tuple();
                    Pair::new(vm_field.into_field(), comma)
                })
                .collect(),
        })
    }

    pub fn as_default_fields(&self) -> TokenStream {
        let fields = self
            .named
            .iter()
            .map(|f| {
                let ident = &f.ident;
                let expr = &f.default_value.as_ref().unwrap().expr;
                quote! { #ident: #expr.into() }
            })
            .chain(vec![
                quote! { task_pool: egui_mvvm::task_pool::TaskPool::new() },
            ]);

        quote! { #(#fields),* }
    }
}

impl ViewModelField {
    pub fn into_field(self) -> Field {
        let Self {
            attrs,
            vis,
            mutability,
            ident,
            colon_token,
            ty,
            default_value: _,
        } = self;
        Field {
            attrs,
            vis,
            mutability,
            ident: Some(ident),
            colon_token,
            ty,
        }
    }
}
