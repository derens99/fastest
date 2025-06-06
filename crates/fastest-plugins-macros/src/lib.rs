//! Procedural macros for Fastest plugin development

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, ItemFn};

/// Derive macro for implementing the Plugin trait
#[proc_macro_derive(FastestPlugin, attributes(plugin))]
pub fn derive_plugin(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let expanded = quote! {
        impl fastest_plugins::api::Plugin for #name {
            fn metadata(&self) -> &fastest_plugins::api::PluginMetadata {
                &self.metadata
            }

            fn as_any(&self) -> &dyn std::any::Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }
        }

        impl std::fmt::Debug for #name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct(stringify!(#name))
                    .field("metadata", &self.metadata)
                    .finish()
            }
        }
    };

    TokenStream::from(expanded)
}

/// Attribute macro for marking hook functions
#[proc_macro_attribute]
pub fn hook(args: TokenStream, input: TokenStream) -> TokenStream {
    let _ = args; // Hook name would be parsed from args
    let input = parse_macro_input!(input as ItemFn);

    // For now, just return the function as-is
    // In a full implementation, this would generate hook registration code
    TokenStream::from(quote! { #input })
}

/// Attribute macro for pytest-compatible fixtures
#[proc_macro_attribute]
pub fn fixture(args: TokenStream, input: TokenStream) -> TokenStream {
    let _ = args; // Fixture parameters would be parsed from args
    let input = parse_macro_input!(input as ItemFn);

    // For now, just return the function as-is
    // In a full implementation, this would generate fixture registration code
    TokenStream::from(quote! { #input })
}
