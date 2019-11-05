#![recursion_limit = "4096"]

extern crate proc_macro2;
extern crate syn;
#[macro_use]
extern crate quote;
extern crate evitable_syn_meta_ext;

use attrs::Attrs;
use description::{Description, ResolvedDescription};
use display::DisplayImpl;
use evitable_syn_meta_ext::{
  error::Error, error::Result, AttrExt, Fields, FromDeriveInput, FromField, FromMeta, FromVariant,
  MapFields, Meta, NestedMeta, PathExt,
};
use from::FromImpl;
use ident_case::RenameRule;
use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use std::borrow::Cow;
pub use syn::parse_macro_input;
use syn::{
  parse::Parser, parse_str, spanned::Spanned, Attribute, DataEnum, DataStruct, DeriveInput, Field,
  Generics, Ident, Path, Type, Variant, Visibility,
};
use trait_assert::assert_trait_impl;

mod attrs;
mod description;
mod display;
mod from;
mod from_context;
mod impl_display;
mod kinds;
mod trait_assert;
mod visibility;

trait IntoIdent<'a> {
  fn into_ident(self) -> Cow<'a, Ident>;
}

impl<'a> IntoIdent<'a> for Ident {
  #[inline]
  fn into_ident(self) -> Cow<'a, Ident> {
    Cow::Owned(self)
  }
}

impl<'a> IntoIdent<'a> for &'a Ident {
  #[inline]
  fn into_ident(self) -> Cow<'a, Ident> {
    Cow::Borrowed(self)
  }
}

impl<'a> IntoIdent<'a> for String {
  #[inline]
  fn into_ident(self) -> Cow<'a, Ident> {
    Cow::Owned(Ident::new(&self, Span::call_site()))
  }
}

impl<'a> IntoIdent<'a> for &'a str {
  #[inline]
  fn into_ident(self) -> Cow<'a, Ident> {
    Cow::Owned(Ident::new(self, Span::call_site()))
  }
}

impl<'a> IntoIdent<'a> for usize {
  #[inline]
  fn into_ident(self) -> Cow<'a, Ident> {
    format!("_{}", self).into_ident()
  }
}

impl<'a> IntoIdent<'a> for &'a usize {
  #[inline]
  fn into_ident(self) -> Cow<'a, Ident> {
    (*self).into_ident()
  }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CopyMethod {
  Copy,
  Clone,
}

impl Default for CopyMethod {
  fn default() -> Self {
    CopyMethod::Copy
  }
}

impl CopyMethod {
  pub fn trait_path(&self) -> Path {
    match self {
      CopyMethod::Copy => parse_str("::std::marker::Copy").unwrap(),
      CopyMethod::Clone => parse_str("::std::clone::Clone").unwrap(),
    }
  }

  pub fn copy<T: ToTokens>(&self, from_expr: &T) -> TokenStream {
    let mut tokens = TokenStream::new();
    match self {
      CopyMethod::Copy => tokens.extend(quote! { *#from_expr }),
      CopyMethod::Clone => tokens.extend(quote! { ::std::clone::Clone::clone(#from_expr) }),
    }

    tokens
  }

  #[inline]
  pub fn is_copy(&self) -> bool {
    match self {
      CopyMethod::Copy => true,
      _ => false,
    }
  }
}

impl FromMeta for CopyMethod {
  fn from_string<S: Spanned>(value: &str, span: &S) -> Result<Self> {
    match value {
      "copy" => Ok(CopyMethod::Copy),
      "clone" => Ok(CopyMethod::Clone),
      v => Err(Error::unknown_value(&v).with_span(span)),
    }
  }

  fn from_ident(value: &Ident) -> Result<Self> {
    let s = value.to_string();
    Self::from_string(&s, value)
  }
}

struct ErrorField {
  ty: Type,
  include_in_kind: bool,
  method: CopyMethod,
}

impl ErrorField {
  #[inline]
  pub fn is_copy(&self) -> bool {
    self.method.is_copy()
  }
}

struct ErrorVariant {
  ident: Ident,
  description: ResolvedDescription,
  from_impls: Vec<FromImpl>,
  fields: Fields<ErrorField>,
}

impl ErrorVariant {
  pub(crate) fn destruct(&self) -> TokenStream {
    match &self.fields {
      Fields::Unit => TokenStream::new(),
      Fields::Named(f) => {
        let idents = f.iter().map(|(i, _)| i);
        quote! { { #(#idents,)* } }
      }
      Fields::Unnamed(f) => {
        let idents = f.iter().map(|(i, _)| i.into_ident());
        quote! { ( #(#idents,)* ) }
      }
    }
  }
}

struct ErrorStruct {
  description: ResolvedDescription,
  from_impls: Vec<FromImpl>,
  fields: Fields<ErrorField>,
}

enum ErrorData {
  Struct(ErrorStruct),
  Enum(Vec<ErrorVariant>),
}

enum TypeAliasName {
  Default,
  Override(Ident),
  Disabled,
  Enabled,
}

impl TypeAliasName {
  fn ident(&self, default: &str) -> Option<Ident> {
    match self {
      TypeAliasName::Disabled => None,
      TypeAliasName::Override(i) => Some(i.clone()),
      TypeAliasName::Default | TypeAliasName::Enabled => {
        Some(Ident::new(default, Span::call_site()))
      }
    }
  }
}

impl Default for TypeAliasName {
  fn default() -> Self {
    TypeAliasName::Default
  }
}

impl FromMeta for TypeAliasName {
  fn from_bool<S: Spanned>(value: bool, _span: &S) -> Result<Self> {
    if value {
      Ok(TypeAliasName::Enabled)
    } else {
      Ok(TypeAliasName::Disabled)
    }
  }

  fn from_string<S: Spanned>(value: &str, span: &S) -> Result<Self> {
    let ident = Ident::new(value, span.span());
    Ok(TypeAliasName::Override(ident))
  }

  fn from_ident(value: &Ident) -> Result<Self> {
    Ok(TypeAliasName::Override(value.clone()))
  }

  fn from_empty() -> Result<Self> {
    Ok(TypeAliasName::Enabled)
  }
}

struct ErrorType {
  ident: Ident,
  vis: Visibility,
  generics: Generics,
  data: ErrorData,
  attrs: ErrorTypeAttrs,
  mod_name: Ident,
  mod_vis: Visibility,
  impls_from: Vec<from::FromImplFor>,
}

struct ErrorTypeAttrs {
  error_type_name: TypeAliasName,
  result_type_name: TypeAliasName,
  kind_type_name: TypeAliasName,
}

impl ErrorTypeAttrs {
  fn from_attrs(attrs: &mut Attrs) -> Result<Self> {
    let error_type_name = attrs.get_optional("error_type")?.unwrap_or_default();
    let result_type_name = attrs.get_optional("result_type")?.unwrap_or_default();
    let kind_type_name = attrs.get_optional("kind_type")?.unwrap_or_default();

    Ok(Self {
      error_type_name,
      result_type_name,
      kind_type_name,
    })
  }
}

impl FromVariant for ErrorVariant {
  fn from_variant(variant: &Variant) -> Result<Self> {
    let mut attrs = Attrs::from_attributes(&variant.attrs)?;
    let fields = variant
      .fields
      .clone()
      .try_map_fields(|field| ErrorField::from_field(&field))?;

    let description: Description = attrs.get_required("description", &variant.ident)?;
    let description = description.resolve_from_variant(&fields)?;
    let from_impls = attrs.get_list("from")?;
    attrs.ensure_used()?;

    Ok(ErrorVariant {
      ident: variant.ident.clone(),
      description,
      from_impls,
      fields,
    })
  }
}

impl FromField for ErrorField {
  fn from_field(field: &Field) -> Result<Self> {
    let mut attrs = Attrs::from_attributes(&field.attrs)?;
    let include_in_kind = attrs.get_optional("include_in_kind")?.unwrap_or(false);
    let method = if include_in_kind {
      match attrs.get_optional("method")? {
        Some(m) => m,
        None => match attrs.get_optional("clone")? {
          None => Default::default(),
          Some(true) => CopyMethod::Clone,
          Some(false) => CopyMethod::Copy,
        },
      }
    } else {
      Default::default()
    };

    attrs.ensure_used()?;
    Ok(ErrorField {
      ty: field.ty.clone(),
      include_in_kind,
      method,
    })
  }
}

impl ErrorType {
  fn impl_froms<'a>(
    data: &ErrorData,
    ident: &Ident,
    mod_name: &Ident,
  ) -> Result<Vec<from::FromImplFor>> {
    match data {
      ErrorData::Struct(s) => s
        .from_impls
        .iter()
        .map(|f| f.for_struct(s, mod_name, ident))
        .collect(),
      ErrorData::Enum(variants) => variants
        .iter()
        .flat_map(|variant| {
          variant
            .from_impls
            .iter()
            .map(move |f| f.for_variant(&variant, mod_name, ident))
        })
        .collect(),
    }
  }

  fn new(
    ident: Ident,
    vis: Visibility,
    generics: Generics,
    data: ErrorData,
    attrs: ErrorTypeAttrs,
  ) -> Result<Self> {
    let mod_name =
      "evitable_".to_owned() + &RenameRule::SnakeCase.apply_to_variant(ident.to_string());
    let mod_name = Ident::new(&mod_name, ident.span());
    let mod_vis = visibility::inherited(&vis, 1);
    let impls_from = Self::impl_froms(&data, &ident, &mod_name)?;

    Ok(Self {
      ident,
      vis,
      generics,
      data,
      attrs,
      mod_name,
      mod_vis,
      impls_from,
    })
  }
}

impl FromDeriveInput for ErrorType {
  fn from_enum(
    attrs: &Vec<Attribute>,
    vis: &Visibility,
    ident: &Ident,
    generics: &Generics,
    input: &DataEnum,
  ) -> Result<Self> {
    let variants: Result<Vec<ErrorVariant>> = input
      .variants
      .iter()
      .map(ErrorVariant::from_variant)
      .collect();

    let mut evitable_attrs = Attrs::from_attributes(attrs)?;
    let attrs = ErrorTypeAttrs::from_attrs(&mut evitable_attrs)?;
    evitable_attrs.ensure_used()?;
    let data = ErrorData::Enum(variants?);

    ErrorType::new(ident.clone(), vis.clone(), generics.clone(), data, attrs)
  }

  fn from_struct(
    attrs: &Vec<Attribute>,
    vis: &Visibility,
    ident: &Ident,
    generics: &Generics,
    input: &DataStruct,
  ) -> Result<Self> {
    let fields = (&input.fields).try_map_fields(ErrorField::from_field)?;
    let mut evitable_attrs = Attrs::from_attributes(attrs)?;
    let attrs = ErrorTypeAttrs::from_attrs(&mut evitable_attrs)?;
    let description: Description = evitable_attrs.get_required("description", ident)?;
    let description = description.resolve_from_inst(&fields, "self")?;
    let from_impls = evitable_attrs.get_list("from")?;
    evitable_attrs.ensure_used()?;
    let data = ErrorData::Struct(ErrorStruct {
      description,
      fields,
      from_impls,
    });

    ErrorType::new(ident.clone(), vis.clone(), generics.clone(), data, attrs)
  }
}

impl ToTokens for ErrorType {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let ty = &self.ident; // TOOD: Generics
    let vis = &self.vis;
    let mod_name = &self.mod_name;
    let mod_item_vis = &self.mod_vis;

    let kinds = kinds::for_type(self);
    let from_context = from_context::for_type(&kinds, ty);
    let impl_display = DisplayImpl::for_type(self);
    let impls_from = &self.impls_from;

    tokens.extend(quote! {
      #vis mod #mod_name {
        use super::*;

        #kinds
        #from_context

        #mod_item_vis struct Error {
          context: super::#ty,
          backtrace: ::evitable::Backtrace,
          source: Option<Box<dyn ::std::error::Error + ::std::marker::Send + ::std::marker::Sync + 'static>>,
        }

        impl Error {
          #[inline]
          fn context(&self) -> &super::#ty {
            &self.context
          }

          #[inline]
          fn backtrace(&self) -> &::evitable::Backtrace {
            &self.backtrace
          }

          #[inline]
          fn kind(&self) -> ErrorKind {
            ::evitable::ErrorContext::kind(&self.context)
          }
        }

        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::std::convert::From<super::#ty> for Error {
          #[inline]
          fn from(context: super::#ty) -> Self {
            <Error as ::evitable::EvitableError>::new(context, None)
          }
        }

        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::std::fmt::Display for Error {
          fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            ::std::fmt::Display::fmt(&self.context, f)?;
            if let Some(source) = &self.source {
              f.write_str("\n---- source ----\n")?;
              ::std::fmt::Display::fmt(source, f)?;
            }

            Ok(())
          }
        }

        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::std::fmt::Debug for Error {
          fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            // TODO: Include backtrace
            ::std::fmt::Debug::fmt(&self.context, f)?;
            if let Some(source) = &self.source {
              f.write_str("\n---- source ----\n")?;
              ::std::fmt::Debug::fmt(source, f)?;
            }

            Ok(())
          }
        }

        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::std::error::Error for Error {
          fn source(&self) -> Option<&(dyn ::std::error::Error + 'static)> {
            match &self.source {
              None => None,
              Some(b) => Some(b.as_ref()),
            }
          }
        }


        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::evitable::EvitableError for Error {
          type Kind = ErrorKind;
          type Context = super::#ty;

          #[inline]
          fn context(&self) -> &Self::Context {
            Error::context(self)
          }

          #[inline]
          fn kind(&self) -> Self::Kind {
            Error::kind(self)
          }

          #[inline]
          fn backtrace(&self) -> &::evitable::Backtrace {
            Error::backtrace(self)
          }

          #[inline]
          fn new(context: Self::Context, source: Option<Box<dyn ::std::error::Error + ::std::marker::Send + ::std::marker::Sync + 'static>>) -> Self {
            let backtrace = ::evitable::Backtrace::new();

            Self {
              context,
              source,
              backtrace,
            }
          }
        }

        #(#impls_from)*

        #mod_item_vis type Result<T> = ::std::result::Result<T, Error>;
      }

      #impl_display

      #[automatically_derived]
      #[allow(unused_qualifications)]
      impl ::evitable::ErrorContext for #ty {
        type Kind = #mod_name::ErrorKind;
        type Error = #mod_name::Error;

        fn kind(&self) -> Self::Kind {
          #mod_name::ErrorKind::from_context(self)
        }
      }
    });

    if let Some(ident) = self.attrs.error_type_name.ident("Error") {
      tokens.extend(quote! {
        #vis type #ident = #mod_name::Error;
      });
    }

    if let Some(ident) = self.attrs.result_type_name.ident("Result") {
      tokens.extend(quote! {
        #vis type #ident<T> = #mod_name::Result<T>;
      });
    }

    if let Some(ident) = self.attrs.kind_type_name.ident("ErrorKind") {
      tokens.extend(quote! {
        #vis type #ident = #mod_name::ErrorKind;
      });
    }
  }
}

fn add_attribute(input: &mut DeriveInput, attr: Attribute) {
  input.attrs.push(attr);
}

trait AttributeTree {
  fn visit(&mut self, f: &impl Fn(&mut Vec<Attribute>) -> ());
}

impl AttributeTree for syn::Field {
  fn visit(&mut self, f: &impl Fn(&mut Vec<Attribute>) -> ()) {
    f(&mut self.attrs);
  }
}

impl AttributeTree for syn::FieldsNamed {
  fn visit(&mut self, f: &impl Fn(&mut Vec<Attribute>) -> ()) {
    for field in self.named.iter_mut() {
      field.visit(f);
    }
  }
}

impl AttributeTree for syn::FieldsUnnamed {
  fn visit(&mut self, f: &impl Fn(&mut Vec<Attribute>) -> ()) {
    for field in self.unnamed.iter_mut() {
      field.visit(f);
    }
  }
}

impl AttributeTree for syn::Fields {
  fn visit(&mut self, f: &impl Fn(&mut Vec<Attribute>) -> ()) {
    match self {
      syn::Fields::Named(n) => n.visit(f),
      syn::Fields::Unnamed(u) => u.visit(f),
      syn::Fields::Unit => (),
    }
  }
}

impl AttributeTree for syn::DataStruct {
  fn visit(&mut self, f: &impl Fn(&mut Vec<Attribute>) -> ()) {
    self.fields.visit(f);
  }
}

impl AttributeTree for syn::Variant {
  fn visit(&mut self, f: &impl Fn(&mut Vec<Attribute>) -> ()) {
    f(&mut self.attrs);
    self.fields.visit(f);
  }
}

impl AttributeTree for syn::DataEnum {
  fn visit(&mut self, f: &impl Fn(&mut Vec<Attribute>) -> ()) {
    for variant in self.variants.iter_mut() {
      variant.visit(f);
    }
  }
}

impl AttributeTree for syn::Data {
  fn visit(&mut self, f: &impl Fn(&mut Vec<Attribute>) -> ()) {
    match self {
      syn::Data::Struct(s) => s.visit(f),
      syn::Data::Enum(e) => e.visit(f),
      syn::Data::Union(_) => (),
    }
  }
}

impl AttributeTree for syn::DeriveInput {
  fn visit(&mut self, f: &impl Fn(&mut Vec<Attribute>) -> ()) {
    f(&mut self.attrs);
    self.data.visit(f);
  }
}

fn remove_evitable_attrs(input: &mut DeriveInput) {
  input.visit(&|attrs| {
    let a = std::mem::replace(attrs, Vec::new());
    let a = a
      .into_iter()
      .filter(|attr| attr.meta().map(|m| !m.is("evitable")).unwrap_or(true))
      .collect();
    std::mem::replace(attrs, a);
  });
  // //let before_len = input.attrs.len();
  // let attrs = std::mem::replace(&mut input.attrs, Vec::new());
  // let attrs = attrs
  //   .into_iter()
  //   .filter(|attr| attr.meta().map(|m| !m.is("evitable")).unwrap_or(true))
  //   .collect();
  // std::mem::replace(&mut input.attrs, attrs);
  // //let after_len = input.attrs.len();
  // // eprintln!(
  // //   "{} before_len: {}, after_len: {}",
  // //   input.ident, before_len, after_len
  // // );

  // match &mut input.data {
  //   syn::Data::Struct(..) => {}
  //   syn::Data::Enum(e) => {
  //     for v in &mut e.variants {
  //       //let before_len = v.attrs.len();
  //       let attrs = std::mem::replace(&mut v.attrs, Vec::new());
  //       let attrs = attrs
  //         .into_iter()
  //         .filter(|attr| attr.meta().map(|m| !m.is("evitable")).unwrap_or(true))
  //         .collect();
  //       std::mem::replace(&mut v.attrs, attrs);
  //       //let after_len = v.attrs.len();
  //       // eprintln!(
  //       //   "{}::{} before_len: {}, after_len: {}",
  //       //   input.ident, v.ident, before_len, after_len
  //       // );
  //     }
  //   }
  //   _ => {}
  // }
}

pub fn derive_evitable(meta: &TokenStream, input: &mut DeriveInput) -> TokenStream {
  let mut cloned = input.clone();
  remove_evitable_attrs(input);

  if !meta.is_empty() {
    let meta_attr_quote = quote! {#[evitable(#meta)]};
    let parser = Attribute::parse_outer;
    let attr = match parser.parse2(meta_attr_quote) {
      Ok(val) => val[0].clone(),
      Err(err) => return err.to_compile_error(),
    };

    add_attribute(&mut cloned, attr);
  }

  let error_type = ErrorType::from_derive_input(&cloned);
  match error_type {
    Ok(val) => quote! { #input #val },
    Err(err) => err.write_errors(),
  }
}
