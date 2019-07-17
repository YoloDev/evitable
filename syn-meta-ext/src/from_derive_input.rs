use super::error::{Error, Result};
use proc_macro2::TokenStream;
use syn::{
  Attribute, Data, DataEnum, DataStruct, DataUnion, DeriveInput, Generics, Ident, Visibility,
};

/// Creates an instance by parsing an entire proc-macro `derive` input,
/// including the, identity, generics, and visibility of the type.
///
/// This trait should either be derived or manually implemented by a type
/// in the proc macro crate which is directly using `darling`. It is unlikely
/// that these implementations will be reusable across crates.
pub trait FromDeriveInput: Sized {
  /// Create an instance from `syn::DeriveInput`, or return an error.
  fn from_derive_input(input: &DeriveInput) -> Result<Self> {
    match &input.data {
      Data::Union(d) => {
        Self::from_union(&input.attrs, &input.vis, &input.ident, &input.generics, d)
      }
      Data::Enum(d) => Self::from_enum(&input.attrs, &input.vis, &input.ident, &input.generics, d),
      Data::Struct(d) => {
        Self::from_struct(&input.attrs, &input.vis, &input.ident, &input.generics, d)
      }
    }
  }

  fn from_union(
    attrs: &Vec<Attribute>,
    vis: &Visibility,
    ident: &Ident,
    generics: &Generics,
    input: &DataUnion,
  ) -> Result<Self> {
    Err(Error::unsupported_shape("union"))
  }

  fn from_enum(
    attrs: &Vec<Attribute>,
    vis: &Visibility,
    ident: &Ident,
    generics: &Generics,
    input: &DataEnum,
  ) -> Result<Self> {
    Err(Error::unsupported_shape("enum"))
  }

  fn from_struct(
    attrs: &Vec<Attribute>,
    vis: &Visibility,
    ident: &Ident,
    generics: &Generics,
    input: &DataStruct,
  ) -> Result<Self> {
    Err(Error::unsupported_shape("struct"))
  }

  fn parse(input: TokenStream) -> Result<Self> {
    let input = syn::parse2(input)?;
    Self::from_derive_input(&input)
  }
}

impl FromDeriveInput for () {
  fn from_derive_input(_: &DeriveInput) -> Result<Self> {
    Ok(())
  }
}

impl FromDeriveInput for DeriveInput {
  fn from_derive_input(input: &DeriveInput) -> Result<Self> {
    Ok(input.clone())
  }
}
