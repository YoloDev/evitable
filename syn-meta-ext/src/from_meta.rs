use crate::quote::ToTokens;
use proc_macro2::TokenStream;
use std::rc::Rc;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use syn::spanned::Spanned;
use syn::{Ident, Lit, Path};

use super::error::{Error, Result};
use super::{Meta, MetaValue, NestedMeta};

pub trait FromMeta: Sized {
  fn from_nested_meta(item: &NestedMeta) -> Result<Self> {
    (match item {
      NestedMeta::Literal(lit) => Self::from_lit(lit),
      NestedMeta::Meta(mi) => match mi {
        Meta::Path(p) => Self::from_path(p),
        Meta::NameValue(_) => Err(Error::unexpected_type("name value").with_span(item)),
        Meta::List(_) => Err(Error::unexpected_type("list").with_span(item)),
      },
    })
    .map_err(|e| e.with_span(item))
  }

  /// Create an instance from a `syn::Meta` by dispatching to the format-appropriate
  /// trait function. This generally should not be overridden by implementers.
  ///
  /// # Error Spans
  /// If this method is overridden and can introduce errors that weren't passed up from
  /// other `from_meta` calls, the override must call `with_span` on the error using the
  /// `item` to make sure that the emitted diagnostic points to the correct location in
  /// source code.
  fn from_meta(item: &Meta) -> Result<Self> {
    (match item {
      Meta::Path(_) => Self::from_empty(),
      Meta::List(value) => {
        Self::from_list(value.nested.iter().collect::<Vec<&NestedMeta>>().as_ref())
      }
      Meta::NameValue(value) => Self::from_value(&value.val),
    })
    .map_err(|e| e.with_span(item))
  }

  /// Create an instance from the presence of the word in the attribute with no
  /// additional options specified.
  fn from_empty() -> Result<Self> {
    Err(Error::unsupported_format("empty"))
  }

  /// Create an instance from a list of nested meta items.
  #[allow(unused_variables)]
  fn from_list(items: &[&NestedMeta]) -> Result<Self> {
    match items.len() {
      0 => Self::from_empty(),
      1 => Self::from_nested_meta(&items[0]),
      _ => Err(Error::unsupported_format("list")),
    }
  }

  /// Create an instance from a literal value of either `foo = "bar"` or `foo("bar")`.
  /// This dispatches to the appropriate method based on the type of literal encountered,
  /// and generally should not be overridden by implementers.
  ///
  /// # Error Spans
  /// If this method is overridden, the override must make sure to add `value`'s span
  /// information to the returned error by calling `with_span(value)` on the `Error` instance.
  fn from_value(value: &MetaValue) -> Result<Self> {
    (match value {
      MetaValue::Path(path) => Self::from_path(path),
      MetaValue::Literal(lit) => Self::from_lit(lit),
    })
    .map_err(|e| e.with_span(value))
  }

  fn from_lit(lit: &Lit) -> Result<Self> {
    (match lit {
      Lit::Bool(b) => Self::from_bool(b.value, b),
      Lit::Str(s) => Self::from_string(&s.value(), s),
      Lit::Char(c) => Self::from_char(c.value(), c),
      Lit::Int(i) => Self::from_int(i.base10_parse()?, i),
      _ => Err(Error::unexpected_lit_type(lit)),
    })
    .map_err(|e| e.with_span(lit))
  }

  /// Create an instance from a char literal in a value position.
  #[allow(unused_variables)]
  fn from_char<S: Spanned>(value: char, span: &S) -> Result<Self> {
    Err(Error::unexpected_type("char").with_span(span))
  }

  /// Create an instance from a int literal in a value position.
  #[allow(unused_variables)]
  fn from_int<S: Spanned>(value: u64, span: &S) -> Result<Self> {
    Err(Error::unexpected_type("int").with_span(span))
  }

  /// Create an instance from a char literal in a value position.
  #[allow(unused_variables)]
  fn from_path(value: &Path) -> Result<Self> {
    if value.leading_colon.is_none()
      && value.segments.len() == 1
      && value.segments[0].arguments == syn::PathArguments::None
    {
      Self::from_ident(&value.segments[0].ident)
    } else {
      Err(Error::unexpected_type("path"))
    }
  }

  /// Create an instance from a string literal in a value position.
  #[allow(unused_variables)]
  fn from_string<S: Spanned>(value: &str, span: &S) -> Result<Self> {
    Err(Error::unexpected_type("string").with_span(span))
  }

  /// Create an instance from a bool literal in a value position.
  #[allow(unused_variables)]
  fn from_bool<S: Spanned>(value: bool, span: &S) -> Result<Self> {
    Err(Error::unexpected_type("bool").with_span(span))
  }

  #[allow(unused_variables)]
  fn from_ident(value: &Ident) -> Result<Self> {
    Err(Error::unexpected_type("ident"))
  }
}

// FromMeta impls for std and syn types.

impl FromMeta for () {
  fn from_empty() -> Result<Self> {
    Ok(())
  }
}

impl FromMeta for bool {
  fn from_empty() -> Result<Self> {
    Ok(true)
  }

  fn from_bool<S: Spanned>(value: bool, _: &S) -> Result<Self> {
    Ok(value)
  }

  fn from_string<S: Spanned>(value: &str, span: &S) -> Result<Self> {
    value
      .parse()
      .map_err(|_| Error::unknown_value(value).with_span(span))
  }
}

impl FromMeta for AtomicBool {
  fn from_meta(mi: &Meta) -> Result<Self> {
    FromMeta::from_meta(mi)
      .map(AtomicBool::new)
      .map_err(|e| e.with_span(mi))
  }
}

impl FromMeta for String {
  fn from_string<S: Spanned>(s: &str, _span: &S) -> Result<Self> {
    Ok(s.to_string())
  }

  fn from_path(p: &Path) -> Result<Self> {
    let mut ss = TokenStream::new();
    p.to_tokens(&mut ss);
    Ok(format!("{}", ss))
  }
}

/// Generate an impl of `FromMeta` that will accept strings which parse to numbers or
/// integer literals.
macro_rules! from_meta_num {
  ($ty:ident) => {
    impl FromMeta for $ty {
      fn from_string<S: Spanned>(s: &str, span: &S) -> Result<Self> {
        s.parse()
          .map_err(|_| Error::unknown_value(s).with_span(span))
      }

      fn from_lit(value: &Lit) -> Result<Self> {
        (match value {
          Lit::Str(s) => Self::from_string(&s.value(), s),
          Lit::Int(s) => Ok(s.base10_parse()?),
          v => Err(Error::unexpected_lit_type(value).with_span(&v)),
        })
        .map_err(|e| e.with_span(value))
      }
    }
  };
}

from_meta_num!(u8);
from_meta_num!(u16);
from_meta_num!(u32);
from_meta_num!(u64);
from_meta_num!(usize);
from_meta_num!(i8);
from_meta_num!(i16);
from_meta_num!(i32);
from_meta_num!(i64);
from_meta_num!(isize);

/// Generate an impl of `FromMeta` that will accept strings which parse to floats or
/// float literals.
macro_rules! from_meta_float {
  ($ty:ident) => {
    impl FromMeta for $ty {
      fn from_string<S: Spanned>(s: &str, span: &S) -> Result<Self> {
        s.parse()
          .map_err(|_| Error::unknown_value(s).with_span(span))
      }

      fn from_lit(value: &Lit) -> Result<Self> {
        (match value {
          Lit::Str(s) => Self::from_string(&s.value(), s),
          Lit::Float(s) => Ok(s.base10_parse()?),
          v => Err(Error::unexpected_lit_type(value).with_span(&v)),
        })
        .map_err(|e| e.with_span(value))
      }
    }
  };
}

from_meta_float!(f32);
from_meta_float!(f64);

/// Parsing support for identifiers. This attempts to preserve span information
/// when available, but also supports parsing strings with the call site as the
/// emitted span.
impl FromMeta for syn::Ident {
  fn from_ident(value: &Ident) -> Result<Self> {
    Ok(value.clone())
  }

  fn from_string<S: Spanned>(value: &str, span: &S) -> Result<Self> {
    Ok(syn::Ident::new(value, span.span()))
  }
}

/// Parsing support for paths. This attempts to preserve span information when available,
/// but also supports parsing strings with the call site as the emitted span.
impl FromMeta for syn::Path {
  fn from_path(path: &Path) -> Result<Self> {
    Ok(path.clone())
  }

  fn from_string<S: Spanned>(value: &str, span: &S) -> Result<Self> {
    syn::parse_str(value).map_err(|_| Error::unknown_value(value).with_span(span))
  }

  fn from_lit(value: &Lit) -> Result<Self> {
    if let Lit::Str(ref path_str) = *value {
      path_str
        .parse()
        .map_err(|_| Error::unknown_lit_str_value(path_str).with_span(value))
    } else {
      Err(Error::unexpected_lit_type(value).with_span(value))
    }
  }
}

impl FromMeta for syn::Lit {
  fn from_lit(value: &Lit) -> Result<Self> {
    Ok(value.clone())
  }
}

macro_rules! from_meta_lit {
  ($impl_ty:path, $lit_variant:path) => {
    impl FromMeta for $impl_ty {
      fn from_lit(value: &Lit) -> Result<Self> {
        if let $lit_variant(ref value) = *value {
          Ok(value.clone())
        } else {
          Err(Error::unexpected_lit_type(value).with_span(value))
        }
      }
    }
  };
}

from_meta_lit!(syn::LitInt, Lit::Int);
from_meta_lit!(syn::LitFloat, Lit::Float);
from_meta_lit!(syn::LitStr, Lit::Str);
from_meta_lit!(syn::LitByte, Lit::Byte);
from_meta_lit!(syn::LitByteStr, Lit::ByteStr);
from_meta_lit!(syn::LitChar, Lit::Char);
from_meta_lit!(syn::LitBool, Lit::Bool);
from_meta_lit!(proc_macro2::Literal, Lit::Verbatim);

impl FromMeta for Meta {
  fn from_meta(value: &Meta) -> Result<Self> {
    Ok(value.clone())
  }
}

impl FromMeta for ident_case::RenameRule {
  fn from_string<S: Spanned>(value: &str, span: &S) -> Result<Self> {
    value
      .parse()
      .map_err(|_| Error::unknown_value(value).with_span(span))
  }
}

impl<T: FromMeta> FromMeta for Option<T> {
  fn from_meta(item: &Meta) -> Result<Self> {
    FromMeta::from_meta(item).map(Some)
  }
}

impl<T: FromMeta> FromMeta for Box<T> {
  fn from_meta(item: &Meta) -> Result<Self> {
    FromMeta::from_meta(item).map(Box::new)
  }
}

impl<T: FromMeta> FromMeta for Result<T> {
  fn from_meta(item: &Meta) -> Result<Self> {
    Ok(FromMeta::from_meta(item))
  }
}

/// Parses the meta-item, and in case of error preserves a copy of the input for
/// later analysis.
impl<T: FromMeta> FromMeta for ::std::result::Result<T, Meta> {
  fn from_meta(item: &Meta) -> Result<Self> {
    T::from_meta(item)
      .map(Ok)
      .or_else(|_| Ok(Err(item.clone())))
  }
}

impl<T: FromMeta> FromMeta for Rc<T> {
  fn from_meta(item: &Meta) -> Result<Self> {
    FromMeta::from_meta(item).map(Rc::new)
  }
}

impl<T: FromMeta> FromMeta for Arc<T> {
  fn from_meta(item: &Meta) -> Result<Self> {
    FromMeta::from_meta(item).map(Arc::new)
  }
}

impl<T: FromMeta> FromMeta for Vec<T> {
  fn from_empty() -> Result<Self> {
    Ok(Vec::new())
  }

  fn from_list(items: &[&NestedMeta]) -> Result<Self> {
    let mut ret = Vec::with_capacity(items.len());
    for item in items {
      ret.push(T::from_nested_meta(item)?);
    }

    Ok(ret)
  }

  fn from_value(val: &MetaValue) -> Result<Self> {
    let mut ret = Vec::with_capacity(1);
    ret.push(T::from_value(val)?);
    Ok(ret)
  }
}

/// Tests for `FromMeta` implementations. Wherever the word `ignore` appears in test input,
/// it should not be considered by the parsing.
#[cfg(test)]
mod tests {
  use proc_macro2::TokenStream;
  use syn;

  use super::Meta;
  use super::{FromMeta, Result};
  use crate::AttrExt;

  /// parse a string as a syn::Meta instance.
  fn pm(tokens: TokenStream) -> ::std::result::Result<Meta, String> {
    let attribute: syn::Attribute = parse_quote!(#[#tokens]);
    attribute.meta().map_err(|_| "Unable to parse".into())
  }

  fn fm<T: FromMeta>(tokens: TokenStream) -> T {
    FromMeta::from_meta(&pm(tokens).expect("Tests should pass well-formed input"))
      .expect("Tests should pass valid input")
  }

  #[test]
  fn unit_succeeds() {
    assert_eq!(fm::<()>(quote!(ignore)), ());
  }

  #[test]
  fn bool_succeeds() {
    // word format
    assert_eq!(fm::<bool>(quote!(ignore)), true);

    // bool literal
    assert_eq!(fm::<bool>(quote!(ignore = true)), true);
    assert_eq!(fm::<bool>(quote!(ignore = false)), false);

    // string literals
    assert_eq!(fm::<bool>(quote!(ignore = "true")), true);
    assert_eq!(fm::<bool>(quote!(ignore = "false")), false);
  }

  #[test]
  fn string_succeeds() {
    // cooked form
    assert_eq!(&fm::<String>(quote!(ignore = "world")), "world");

    // raw form
    assert_eq!(&fm::<String>(quote!(ignore = r#"world"#)), "world");
  }

  #[test]
  fn number_succeeds() {
    assert_eq!(fm::<u8>(quote!(ignore = "2")), 2u8);
    assert_eq!(fm::<i16>(quote!(ignore = "-25")), -25i16);
    assert_eq!(fm::<f64>(quote!(ignore = "1.4e10")), 1.4e10);
  }

  #[test]
  fn int_without_quotes() {
    assert_eq!(fm::<u8>(quote!(ignore = 2)), 2u8);
    assert_eq!(fm::<u16>(quote!(ignore = 255)), 255u16);
    assert_eq!(fm::<u32>(quote!(ignore = 5000)), 5000u32);

    // Check that we aren't tripped up by incorrect suffixes
    assert_eq!(fm::<u32>(quote!(ignore = 5000i32)), 5000u32);
  }

  #[test]
  fn float_without_quotes() {
    assert_eq!(fm::<f32>(quote!(ignore = 2.)), 2.0f32);
    assert_eq!(fm::<f32>(quote!(ignore = 2.0)), 2.0f32);
    assert_eq!(fm::<f64>(quote!(ignore = 1.4e10)), 1.4e10f64);
  }

  #[test]
  fn meta_succeeds() {
    assert_eq!(
      fm::<Meta>(quote!(hello(world, today))),
      pm(quote!(hello(world, today))).unwrap()
    );
  }

  /// Tests that fallible parsing will always produce an outer `Ok` (from `fm`),
  /// and will accurately preserve the inner contents.
  #[test]
  fn result_succeeds() {
    fm::<Result<()>>(quote!(ignore)).unwrap();
    fm::<Result<()>>(quote!(ignore(world))).unwrap_err();
  }
}
