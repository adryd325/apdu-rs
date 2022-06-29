//! Implementation of procedural macro for apdu crate.
//! By deriving apdu_derive::Response, you can convert from APDU raw response to an Enum easily.
//! Macro interface is inspired by thiserror crate.
//!
//! ## Examples
//! Here is a simple example to derive Response:
//! ```rust
//! #[derive(apdu_derive::Response)]
//! enum Response {
//!     #[apdu(0x90, 0x00)]
//!     Ok(Vec<u8>),
//!
//!     #[apdu(0x60..=0x69, _)]
//!     NotOk,
//!
//!     #[apdu(_, _)]
//!     Unknown(u8, u8),
//! }
//! ```
//!
//! This is equivalent to implementing this:
//! ```rust
//! enum Response {
//!     Ok(Vec<u8>),
//!     NotOk,
//!     Unknown(u8, u8),
//! }
//!
//! impl From<apdu_core::Response> for Response {
//!     fn from(response: apdu_core::Response) -> Self {
//!         match response.trailer {
//!             (0x90, 0x00) => Self::Ok(response.payload),
//!             (0x60..=0x69, _) => Self::NotOk,
//!             (_, _) => Self::Unknown(response.trailer.0, response.trailer.1),
//!         }
//!     }
//! }
//! ```
//!
//! Also you can combine them with thiserror derive:
//! ```rust
//! #[derive(Debug, apdu_derive::Response, thiserror::Error)]
//! enum Response {
//!     #[apdu(0x60..=0x69, _)]
//!     #[error("not ok!")]
//!     NotOk,
//!
//!     #[apdu(_, _)]
//!     #[error("unknown: {0:#X} {1:#X}")]
//!     Unknown(u8, u8),
//! }
//! ```

extern crate proc_macro;

use syn::{parse_macro_input, DeriveInput, Data};
use proc_macro::{TokenStream};
use quote::quote;

#[proc_macro_derive(Response, attributes(apdu))]
pub fn derive_response(input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as DeriveInput);
    let output: proc_macro2::TokenStream = match item.data {
        Data::Enum(d) => {
            let ty = &item.ident;
            let arms = d.variants.iter().map(|variant| {
                let ident = &variant.ident;
                let attr = variant.attrs.iter().find(|attr| attr.path.is_ident("apdu"));
                if let None = attr {
                    return quote! {}
                }

                let tokens = &attr.unwrap().tokens;
                let left = match tokens.is_empty() {
                    true => quote! { _ },
                    _ => tokens.clone(),
                };

                let right = if variant.fields.is_empty() {
                    quote! { #ty::#ident }
                } else if variant.fields.len() == 1 {
                    quote! { #ty::#ident(response.payload) }
                } else if variant.fields.len() == 2 {
                    quote! { #ty::#ident(response.trailer.0, response.trailer.1) }
                } else {
                    panic!("unsupported type of fields found")
                };

                quote! {
                    #left => #right,
                }
            });

            quote! {
                impl ::std::convert::From<Vec<u8>> for #ty {
                    fn from(bytes: Vec<u8>) -> Self {
                        let response = ::apdu_core::Response::from(bytes);
                        let (sw1, sw2) = response.trailer;

                        match (sw1, sw2) {
                            #(#arms)*
                        }
                    }
                }
            }
        },
        _ => panic!("deriving for Enum is only supported"),
    };

    proc_macro::TokenStream::from(output)
}