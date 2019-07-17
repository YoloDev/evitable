use super::kinds::*;
use super::*;
use proc_macro2::Span;
use quote::ToTokens;
use std::collections::HashSet;
use syn::Ident;

pub(crate) struct FromContext<'a> {
  kinds: &'a ErrorKinds<'a>,
  ty: &'a Ident,
}

pub(crate) fn for_type<'a>(kinds: &'a ErrorKinds<'a>, ty: &'a Ident) -> FromContext<'a> {
  FromContext { kinds, ty }
}

fn create_struct_kind<'a>(
  name: &Ident,
  included_fields: &Fields<&ErrorField>,
  tokens: &mut TokenStream,
  type_asserts: &mut TokenStream,
) {
  match included_fields {
    Fields::Unit => tokens.extend(quote! { ErrorKind::#name }),
    Fields::Named(fields) => {
      let mut assignments = Vec::with_capacity(fields.len());
      for (n, f) in fields.iter() {
        assert_trait_impl(&f.ty, &f.method.trait_path(), type_asserts);
        let copy = f.method.copy(&quote! { &context.#n });
        assignments.push(quote! { #n: #copy, });
      }

      tokens.extend(quote! { ErrorKind::#name { #(#assignments)* } })
    }
    Fields::Unnamed(fields) => {
      let mut assignments = Vec::with_capacity(fields.len());
      for (i, f) in fields.iter() {
        assert_trait_impl(&f.ty, &f.method.trait_path(), type_asserts);
        let copy = f.method.copy(&quote! { &context.#i });
        assignments.push(quote! { #copy, });
      }

      tokens.extend(quote! { ErrorKind::#name { #(assignments)* } })
    }
  }
}

fn create_enum_case<'a>(
  enum_name: &Ident,
  variant_name: &Ident,
  case: &ErrorKind<'a>,
  type_asserts: &mut TokenStream,
) -> TokenStream {
  let mut tokens = TokenStream::new();
  let extract = match (&case.all_fields, &case.included_fields) {
    (Fields::Unit, Fields::Unit) => TokenStream::new(),
    (Fields::Named(_), Fields::Unit) => quote! { { .. } },
    (Fields::Unnamed(_), Fields::Unit) => quote! { (_) },
    (Fields::Named(all), Fields::Named(included)) => {
      if all.len() == included.len() {
        let idents = all.iter().map(|(n, _)| n);
        quote! { { #(#idents,)* } }
      } else {
        assert!(all.len() > included.len());
        let idents = included.iter().map(|(n, _)| n);
        quote! { { #(#idents,)* .. } }
      }
    }
    (Fields::Unnamed(all), Fields::Unnamed(included)) => {
      if all.len() == included.len() {
        let idents = all.iter().map(|(n, _)| n.into_ident());
        quote! { ( #(#idents,)* ) }
      } else {
        let included_indices: HashSet<usize> = included.iter().map(|(n, _)| *n).collect();
        let idents = all.iter().map(|(n, _)| match included_indices.contains(n) {
          false => "_".into_ident(),
          true => n.into_ident(),
        });
        quote! { ( #(#idents,)* ) }
      }
    }
    _ => unreachable!(),
  };
  let create = match &case.included_fields {
    Fields::Unit => quote! { ErrorKind::#variant_name },
    Fields::Named(f) if f.len() == 0 => quote! { ErrorKind::#variant_name },
    Fields::Unnamed(f) if f.len() == 0 => quote! { ErrorKind::#variant_name },
    Fields::Named(fields) => {
      let mut assignments = Vec::with_capacity(fields.len());
      for (n, f) in fields.iter() {
        assert_trait_impl(&f.ty, &f.method.trait_path(), type_asserts);
        let copy = f.method.copy(&quote! { #n });
        assignments.push(quote! { #n: #copy, });
      }
      quote! { ErrorKind::#variant_name { #(#assignments)* } }
    }
    Fields::Unnamed(fields) => {
      let mut assignments = Vec::with_capacity(fields.len());
      for (i, f) in fields.iter() {
        assert_trait_impl(&f.ty, &f.method.trait_path(), type_asserts);
        let ident = i.into_ident();
        //let copy = f.method.copy(&quote! { #ident });
        assignments.push(quote! { #ident, });
      }
      quote! { ErrorKind::#variant_name( #(#assignments)* ) }
    }
  };

  tokens.extend(quote! {
    super::#enum_name::#variant_name #extract => #create,
  });

  tokens
}

impl<'a> ToTokens for FromContext<'a> {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let (mk_kind, type_asserts) = {
      let mut tokens = TokenStream::new();
      let mut type_asserts = TokenStream::new();
      match &self.kinds {
        ErrorKinds::Struct(_, name, kind) => {
          create_struct_kind(&name, &kind.included_fields, &mut tokens, &mut type_asserts)
        }

        ErrorKinds::Enum(_, enum_name, variants) => {
          let cases = variants.iter().map(|(variant_name, kind)| {
            create_enum_case(enum_name, variant_name, kind, &mut type_asserts)
          });
          tokens.extend(quote! {
            match context {
              #(#cases)*
            }
          })
        }
      }

      (tokens, type_asserts)
    };

    let inline = match self.kinds.is_copy() {
      true => quote! { #[inline] },
      false => TokenStream::new(),
    };

    let ty = &self.ty;
    tokens.extend(quote! {
      impl ErrorKind {
        fn __kind_type_asserts() {
          #type_asserts
        }

        #inline
        pub(super) fn from_context(context: &super::#ty) -> Self {
          #mk_kind
        }
      }
    })
  }
}
