extern crate proc_macro;

extern crate evitable_derive_core;

use evitable_derive_core::{derive_evitable, parse_macro_input};
use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn evitable(meta: TokenStream, input: TokenStream) -> TokenStream {
  derive_evitable(&parse_macro_input!(meta), &mut parse_macro_input!(input)).into()
}
