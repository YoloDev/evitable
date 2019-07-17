extern crate proc_macro;

extern crate evitable_derive_core;

use evitable_derive_core::derive_evitable as derive_evitable_impl;
use evitable_derive_core::parse_macro_input;
use proc_macro::TokenStream;

#[proc_macro_derive(ErrorContext, attributes(evitable))]
pub fn derive_evitable(input: TokenStream) -> TokenStream {
  derive_evitable_impl(&parse_macro_input!(input)).into()
}
