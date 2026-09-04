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
//!
//! Also home to `#[derive(RegisterSetting)]` (T19-002) — a second, unrelated
//! derive that only `labonair-settings` itself uses (for its concrete
//! `Settings` structs, `crates/settings/src/concrete.rs`). Kept in this crate
//! rather than a new one because it is equally tiny and the two derives never
//! collide (different trait, different consumer crate).

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

/// `#[derive(RegisterSetting)]` (T19-002) — submits an `inventory::submit!`
/// entry that calls `<Self as Settings>::register(cx)` at
/// `labonair_settings::register_all` time. Only usable from within
/// `labonair-settings` (the generated code addresses everything through the
/// absolute `::labonair_settings::*` path, which that crate makes resolvable
/// to itself via `extern crate self as labonair_settings;` in its root —
/// mirrors the `crate::MergeFrom` trick `derive_merge_from` uses above, one
/// level removed since this derive is meant to be reusable by any type
/// defined in that crate, not just types in the crate root module).
#[proc_macro_derive(RegisterSetting)]
pub fn derive_register_setting(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let expanded = quote! {
        ::labonair_settings::inventory::submit! {
            ::labonair_settings::RegisteredSetting {
                register: |cx: &mut ::labonair_settings::gpui::App| {
                    <#name as ::labonair_settings::Settings>::register(cx);
                },
            }
        }
    };
    expanded.into()
}
