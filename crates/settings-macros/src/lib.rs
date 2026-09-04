//! `#[derive(MergeFrom)]` — generates a field-wise [`MergeFrom::merge_from`]
//! impl for a struct: `self.field.merge_from(&other.field)` for every field,
//! recursing into whatever `MergeFrom` impl that field's type has (the
//! `Option<T>` / `Vec<T>` / `BTreeMap<K, V>` / primitive impls live in
//! `labonair-settings-content::merge_from`).
//!
//! This crate is intentionally tiny and is only ever used from within
//! `labonair-settings-content` itself, so the generated code refers to the
//! trait via `crate::MergeFrom` (re-exported at that crate's root) rather
//! than trying to resolve an external crate path generically.
//!
//! Port of the pattern used by `zed-refrence/zed/crates/settings_content`
//! (Zed derives the equivalent via a similar internal derive macro), trimmed
//! to struct-with-named-fields only — this tree has no enum `SettingsContent`
//! variants that need merging.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_derive(MergeFrom)]
pub fn derive_merge_from(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let body = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => {
                let calls = fields.named.iter().map(|f| {
                    let ident = f.ident.as_ref().expect("named field");
                    quote! { self.#ident.merge_from(&other.#ident); }
                });
                quote! { #(#calls)* }
            }
            Fields::Unnamed(fields) => {
                let calls = (0..fields.unnamed.len()).map(|i| {
                    let idx = syn::Index::from(i);
                    quote! { self.#idx.merge_from(&other.#idx); }
                });
                quote! { #(#calls)* }
            }
            Fields::Unit => quote! {},
        },
        _ => {
            return syn::Error::new_spanned(
                &input.ident,
                "MergeFrom can only be derived for structs",
            )
            .to_compile_error()
            .into();
        }
    };

    let expanded = quote! {
        impl #impl_generics crate::MergeFrom for #name #ty_generics #where_clause {
            fn merge_from(&mut self, other: &Self) {
                #body
            }
        }
    };
    expanded.into()
}
