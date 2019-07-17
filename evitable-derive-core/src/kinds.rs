use super::*;
use proc_macro2::Span;

pub(crate) struct ErrorKind<'a> {
  pub included_fields: Fields<&'a ErrorField>,
  pub all_fields: &'a Fields<ErrorField>,
}

impl<'a> ErrorKind<'a> {
  pub fn new(fields: &'a Fields<ErrorField>) -> Self {
    let included_fields = match fields {
      Fields::Unit => Fields::Unit,
      Fields::Named(f) => {
        let included: Vec<_> = f
          .iter()
          .filter(|(_, f)| f.include_in_kind)
          .map(|(i, f)| (i.to_owned(), f))
          .collect();
        Fields::Named(included)
      }

      Fields::Unnamed(f) => {
        let included: Vec<_> = f
          .iter()
          .filter(|(_, f)| f.include_in_kind)
          .map(|(i, f)| (*i, f))
          .collect();
        Fields::Unnamed(included)
      }
    };

    Self {
      included_fields,
      all_fields: fields,
    }
  }

  pub fn is_copy(&self) -> bool {
    self.included_fields.iter().all(|(_, f)| f.is_copy())
  }
}

impl<'a> ToTokens for ErrorKind<'a> {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    match &self.included_fields {
      Fields::Unit => (),
      Fields::Named(f) if f.len() == 0 => (),
      Fields::Unnamed(f) if f.len() == 0 => (),
      Fields::Named(fields) => {
        syn::token::Brace {
          span: Span::call_site(),
        }
        .surround(tokens, |tokens| {
          for (n, f) in fields {
            let t = &f.ty;
            tokens.extend(quote! {
              #n: #t,
            });
          }
        });
      }

      Fields::Unnamed(fields) => syn::token::Paren {
        span: Span::call_site(),
      }
      .surround(tokens, |tokens| {
        for (_, f) in fields {
          let t = &f.ty;
          tokens.extend(quote! {
            #t,
          });
        }
      }),
    }
  }
}

pub(crate) enum ErrorKinds<'a> {
  Enum(Visibility, &'a Ident, Vec<(&'a Ident, ErrorKind<'a>)>),
  Struct(Visibility, &'a Ident, ErrorKind<'a>),
}

impl<'a> ErrorKinds<'a> {
  fn vis(&self) -> &Visibility {
    match self {
      ErrorKinds::Enum(v, _, _) => v,
      ErrorKinds::Struct(v, _, _) => v,
    }
  }

  pub fn is_copy(&self) -> bool {
    match self {
      ErrorKinds::Enum(_, _, variants) => variants.iter().all(|(_, f)| f.is_copy()),
      ErrorKinds::Struct(_, _, f) => f.is_copy(),
    }
  }
}

impl<'a> ToTokens for ErrorKinds<'a> {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let mut kinds = Vec::new();
    let mut copy = true;
    let vis = self.vis();
    match self {
      ErrorKinds::Struct(_, n, k) => {
        if !k.is_copy() {
          copy = false;
        }

        kinds.push(quote! {
          #n #k
        });
      }

      ErrorKinds::Enum(_, _, variants) => {
        for (n, k) in variants {
          if !k.is_copy() {
            copy = false;
          }

          kinds.push(quote! {
            #n #k
          });
        }
      }
    };

    let copy = if copy {
      quote! { , Copy }
    } else {
      TokenStream::new()
    };
    tokens.extend(quote! {
      #[derive(PartialEq, Debug, Clone #copy)]
      #vis enum ErrorKind {
        #(#kinds,)*

        #[doc(hidden)]
        __Nonexhaustive,
      }

      #[automatically_derived]
      #[allow(unused_qualifications)]
      impl ::std::fmt::Display for ErrorKind {
        #[inline]
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
          // TODO: improve
          <Self as ::std::fmt::Debug>::fmt(self, f)
        }
      }

      #[automatically_derived]
      #[allow(unused_qualifications)]
      impl ::evitable::ErrorKind for ErrorKind {}
    });
  }
}

pub(crate) fn for_type<'a>(error_type: &'a ErrorType) -> ErrorKinds<'a> {
  match &error_type.data {
    ErrorData::Struct(error_struct) => ErrorKinds::Struct(
      visibility::inherited(&error_type.vis, 1),
      &error_type.ident,
      ErrorKind::new(&error_struct.fields),
    ),
    ErrorData::Enum(variants) => ErrorKinds::Enum(
      visibility::inherited(&error_type.vis, 1),
      &error_type.ident,
      variants
        .iter()
        .map(|v| (&v.ident, ErrorKind::new(&v.fields)))
        .collect(),
    ),
  }
}
