//! Provides a derive macro that implements `Envconfig` trait.
//! For complete documentation please see [envconfig](https://docs.rs/envconfig).

extern crate proc_macro;
extern crate proc_macro2;
extern crate syn;
#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use syn::{DeriveInput, Ident};

#[proc_macro_derive(Envconfig, attributes(envconfig))]
pub fn derive(input: TokenStream) -> TokenStream {
    let derive_input: DeriveInput = syn::parse(input).unwrap();
    let gen = impl_envconfig(&derive_input);
    gen.into()
}

fn impl_envconfig(input: &DeriveInput) -> proc_macro2::TokenStream {
    use syn::Data::*;
    let struct_name = &input.ident;

    let inner_impl = match input.data {
        Struct(ref ds) => match ds.fields {
            syn::Fields::Named(ref fields) => {
                impl_envconfig_for_struct(struct_name, &fields.named, &input.attrs)
            }
            _ => panic!("envconfig supports only named fields"),
        },
        _ => panic!("envconfig only supports non-tuple structs"),
    };

    quote!(#inner_impl)
}

fn impl_envconfig_for_struct(
    struct_name: &Ident,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
    _attrs: &[syn::Attribute],
) -> proc_macro2::TokenStream {
    let gen_fields = fields.iter().map(|f| {
        let ident = &f.ident;
        let field_name = quote!(#ident).to_string();

        let attr = f
            .attrs
            .iter()
            .find(|a| {
                let path = &a.path;
                let name = quote!(#path).to_string();
                name == "envconfig"
            }).unwrap_or_else(|| {
                panic!(
                    "Can not find attribute `envconfig` on field `{}`",
                    field_name
                )
            });

        let opt_meta = attr.interpret_meta().unwrap_or_else(|| {
            panic!(
                "Can not interpret meta of `envconfig` attribute on field `{}`",
                field_name
            )
        });

        let list = match opt_meta {
            syn::Meta::List(l) => l.nested,
            _ => panic!(
                "`envconfig` attribute on field `{}` must contain a list",
                field_name
            ),
        };

        let from_item = list
            .iter()
            .map(|item| match item {
                syn::NestedMeta::Meta(meta) => match meta {
                    syn::Meta::NameValue(name_value) => name_value,
                    _ => panic!(
                        "`envconfig` attribute on field `{}` must contain name/value item",
                        field_name
                    ),
                },
                _ => panic!(
                    "Failed to process `envconfig` attribute on field `{}`",
                    field_name
                ),
            }).find(|name_value| {
                let ident = &name_value.ident;
                quote!(#ident).to_string() == "from"
            }).unwrap_or_else(|| {
                panic!(
                    "`envconfig` attribute on field `{}` must contain `from` item",
                    field_name
                )
            });

        let from_value = &from_item.lit;

        quote! {
            #ident: ::envconfig::load_var(#from_value)?
        }
    });

    quote! {
        impl Envconfig for #struct_name {
            fn init() -> Result<Self, ::envconfig::Error> {
                let config = Self {
                    #(#gen_fields,)*
                };
                Ok(config)
            }
        }
    }
}
