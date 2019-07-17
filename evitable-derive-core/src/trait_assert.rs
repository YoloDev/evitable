use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{Path, Type};

struct TraitAssertion<'a> {
  ty: &'a Type,
  trait_path: &'a Path,
}

impl<'a> ToTokens for TraitAssertion<'a> {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let ty = &self.ty;
    let trait_path = &self.trait_path;
    tokens.extend(quote! {{
      #[allow(dead_code)]
      struct AssertHelper<T>(::std::marker::PhantomData<*const T>);
      trait AssertImpl {
        #[inline]
        fn assert() {}
      }

      impl<T: #trait_path> AssertImpl for AssertHelper<T> {}
      AssertHelper::<#ty>::assert();
    }})
  }
}

pub fn assert_trait_impl(ty: &Type, trait_path: &Path, tokens: &mut TokenStream) {
  ToTokens::to_tokens(&TraitAssertion { ty, trait_path }, tokens)
}
