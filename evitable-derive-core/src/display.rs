use super::*;

pub(crate) struct DisplayImpl<'a> {
  ty: &'a ErrorType,
}

impl<'a> ToTokens for DisplayImpl<'a> {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let body = {
      let mut tokens = TokenStream::new();

      match &self.ty.data {
        ErrorData::Struct(struct_data) => struct_data.description.to_tokens(&mut tokens),
        ErrorData::Enum(variants) => {
          let ty = &self.ty.ident;
          let cases = variants.iter().map(|v| {
            let ident = &v.ident;
            let destruct = v.destruct();
            let desc = &v.description;
            quote! { #ty::#ident #destruct => #desc }
          });

          tokens.extend(quote! {
            match self {
              #(#cases,)*
            }
          });
        }
      }

      tokens
    };

    let ty = &self.ty.ident;
    tokens.extend(quote! {
      #[automatically_derived]
      #[allow(unused_qualifications)]
      impl ::std::fmt::Display for #ty {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
          #body
        }
      }

      #[automatically_derived]
      #[allow(unused_qualifications)]
      impl ::std::fmt::Debug for #ty {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
          #body
        }
      }
    })
  }
}

impl<'a> DisplayImpl<'a> {
  pub fn for_type(ty: &'a ErrorType) -> Self {
    Self { ty }
  }
}
