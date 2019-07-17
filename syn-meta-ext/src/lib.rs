extern crate ident_case;
extern crate proc_macro2;
#[cfg_attr(test, macro_use)]
extern crate quote;
#[cfg_attr(test, macro_use)]
extern crate syn;

use proc_macro2::TokenStream;
use quote::ToTokens;

use syn::parse::*;
use syn::{parenthesized, parse, token, Attribute, Ident, Lit, LitBool, Path, Token};

pub mod ast;
pub mod error;
mod from_derive_input;
mod from_field;
mod from_meta;
mod from_variant;

pub use ast::*;
pub use from_derive_input::FromDeriveInput;
pub use from_field::FromField;
pub use from_meta::FromMeta;
pub use from_variant::FromVariant;

pub trait PathExt {
  fn single_ident(&self) -> Option<&Ident>;
  fn to_compact_string(&self) -> String;
}

impl PathExt for Path {
  fn single_ident(&self) -> Option<&Ident> {
    if self.leading_colon.is_none()
      && self.segments.len() == 1
      && self.segments[0].arguments == syn::PathArguments::None
    {
      Some(&self.segments[0].ident)
    } else {
      None
    }
  }

  fn to_compact_string(&self) -> String {
    self
      .segments
      .iter()
      .map(|s| {
        let mut ident = s.ident.to_string();
        match &s.arguments {
          syn::PathArguments::None => ident,
          syn::PathArguments::AngleBracketed(args) => {
            if args.colon2_token.is_some() {
              ident.push_str("::");
            }

            ident.push_str("<");
            for arg in &args.args {
              match arg {
                syn::GenericArgument::Lifetime(l) => {
                  ident.push('\'');
                  ident.push_str(&l.ident.to_string());
                }

                syn::GenericArgument::Type(t) => {
                  let mut ts = TokenStream::new();
                  t.to_tokens(&mut ts);
                  ident.push_str(&format!("{}", ts));
                }

                syn::GenericArgument::Binding(b) => {
                  ident.push_str(&b.ident.to_string());
                  ident.push('=');
                  let mut ts = TokenStream::new();
                  b.ty.to_tokens(&mut ts);
                  ident.push_str(&format!("{}", ts));
                }

                syn::GenericArgument::Constraint(c) => {
                  ident.push_str(&c.ident.to_string());
                  ident.push_str(": ");
                  let mut ts = TokenStream::new();
                  c.bounds.to_tokens(&mut ts);
                  ident.push_str(&format!("{}", ts));
                }

                syn::GenericArgument::Const(e) => {
                  let mut ts = TokenStream::new();
                  e.to_tokens(&mut ts);
                  ident.push_str(&format!("{}", ts));
                }
              }
            }

            ident.push_str(">");
            ident
          }
          syn::PathArguments::Parenthesized(args) => {
            let mut ts = TokenStream::new();
            args.to_tokens(&mut ts);
            ident.push_str(&format!("{}", ts));
            ident
          }
        }
      })
      .collect::<Vec<String>>()
      .join("::")
  }
}

pub trait AttrExt {
  fn meta(&self) -> Result<Meta>;
}

impl AttrExt for Attribute {
  fn meta(&self) -> Result<Meta> {
    let parser = |input: ParseStream| parse_meta_after_path(self.path.clone(), input);

    parse::Parser::parse2(parser, self.tts.clone())
  }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Meta {
  Path(syn::Path),
  List(MetaList),
  NameValue(MetaNameValue),
}

impl Meta {
  pub fn path(&self) -> &Path {
    match self {
      Meta::Path(meta) => meta,
      Meta::List(meta) => &meta.path,
      Meta::NameValue(meta) => &meta.path,
    }
  }

  pub fn is<S>(&self, path: S) -> bool
  where
    S: AsRef<str>,
  {
    match self.path().single_ident() {
      None => false,
      Some(i) => {
        let actual = i.to_string();
        &actual == path.as_ref()
      }
    }
  }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MetaList {
  pub path: syn::Path,
  pub paren_token: syn::token::Paren,
  pub nested: syn::punctuated::Punctuated<NestedMeta, syn::token::Comma>,
}

impl IntoIterator for MetaList {
  type Item = <syn::punctuated::Punctuated<NestedMeta, syn::token::Comma> as IntoIterator>::Item;
  type IntoIter =
    <syn::punctuated::Punctuated<NestedMeta, syn::token::Comma> as IntoIterator>::IntoIter;

  #[inline]
  fn into_iter(self) -> Self::IntoIter {
    self.nested.into_iter()
  }
}

impl<'a> IntoIterator for &'a MetaList {
  type Item =
    <&'a syn::punctuated::Punctuated<NestedMeta, syn::token::Comma> as IntoIterator>::Item;
  type IntoIter =
    <&'a syn::punctuated::Punctuated<NestedMeta, syn::token::Comma> as IntoIterator>::IntoIter;

  #[inline]
  fn into_iter(self) -> Self::IntoIter {
    (&self.nested).into_iter()
  }
}

/// Element of a compile-time attribute list.
#[derive(Debug, Clone, PartialEq)]
pub enum NestedMeta {
  /// A structured meta item, like the `Copy` in `#[derive(Copy)]` which
  /// would be a nested `Meta::Path`.
  Meta(Meta),

  /// A Rust literal, like the `"new_name"` in `#[rename("new_name")]`.
  Literal(syn::Lit),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MetaNameValue {
  pub path: syn::Path,
  pub eq_token: syn::token::Eq,
  pub val: MetaValue,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MetaValue {
  Path(syn::Path),
  Literal(syn::Lit),
}

impl Parse for MetaValue {
  fn parse(input: ParseStream) -> Result<Self> {
    let ahead = input.fork();

    if ahead.call(Path::parse).is_ok() {
      input.parse().map(MetaValue::Path)
    } else if ahead.call(Lit::parse).is_ok() {
      input.parse().map(MetaValue::Literal)
    } else {
      Err(input.error("expected path or literal"))
    }
  }
}

impl Parse for Meta {
  fn parse(input: ParseStream) -> Result<Self> {
    let path = input.call(Path::parse)?;
    parse_meta_after_path(path, input)
  }
}

impl Parse for MetaList {
  fn parse(input: ParseStream) -> Result<Self> {
    let path = input.call(Path::parse)?;
    parse_meta_list_after_path(path, input)
  }
}

impl Parse for MetaNameValue {
  fn parse(input: ParseStream) -> Result<Self> {
    let path = input.call(Path::parse)?;
    parse_meta_name_value_after_path(path, input)
  }
}

impl Parse for NestedMeta {
  fn parse(input: ParseStream) -> Result<Self> {
    let ahead = input.fork();

    if ahead.peek(Lit) && !(ahead.peek(LitBool) && ahead.peek2(Token![=])) {
      input.parse().map(NestedMeta::Literal)
    } else if ahead.call(Path::parse).is_ok() {
      input.parse().map(NestedMeta::Meta)
    } else {
      Err(input.error("expected path or literal"))
    }
  }
}

fn parse_meta_after_path(path: Path, input: ParseStream) -> Result<Meta> {
  if input.peek(token::Paren) {
    parse_meta_list_after_path(path, input).map(Meta::List)
  } else if input.peek(Token![=]) {
    parse_meta_name_value_after_path(path, input).map(Meta::NameValue)
  } else {
    Ok(Meta::Path(path))
  }
}

fn parse_meta_list_after_path(path: Path, input: ParseStream) -> Result<MetaList> {
  let content;
  Ok(MetaList {
    path: path,
    paren_token: parenthesized!(content in input),
    nested: content.parse_terminated(NestedMeta::parse)?,
  })
}

fn parse_meta_name_value_after_path(path: Path, input: ParseStream) -> Result<MetaNameValue> {
  Ok(MetaNameValue {
    path: path,
    eq_token: input.parse()?,
    val: input.parse()?,
  })
}

impl ToTokens for MetaValue {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    match self {
      MetaValue::Literal(l) => l.to_tokens(tokens),
      MetaValue::Path(p) => p.to_tokens(tokens),
    }
  }
}

impl ToTokens for MetaNameValue {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    self.path.to_tokens(tokens);
    self.eq_token.to_tokens(tokens);
    self.val.to_tokens(tokens);
  }
}

impl ToTokens for NestedMeta {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    match self {
      NestedMeta::Meta(meta) => meta.to_tokens(tokens),
      NestedMeta::Literal(lit) => lit.to_tokens(tokens),
    }
  }
}

impl ToTokens for MetaList {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    self.path.to_tokens(tokens);
    self.paren_token.surround(tokens, |tokens| {
      self.nested.to_tokens(tokens);
    })
  }
}

impl ToTokens for Meta {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    match self {
      Meta::Path(path) => path.to_tokens(tokens),
      Meta::List(list) => list.to_tokens(tokens),
      Meta::NameValue(nv) => nv.to_tokens(tokens),
    }
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  pub fn derive_test() {
    let input: syn::ItemStruct = parse_quote! { #[evitable::from = ::std::io::Error] struct Foo; };
    let attr = &input.attrs[0];
    let meta = attr.meta().unwrap();
    let path_str = format!("{}", meta.path().into_token_stream());
    println!("{:?}", meta);
    assert_eq!(path_str, "evitable :: from");
  }
}
