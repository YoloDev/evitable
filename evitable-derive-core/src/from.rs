use super::*;

#[derive(Clone)]
pub(crate) struct FromImpl {
  path: Path,
}

impl FromImpl {
  pub fn for_struct(
    &self,
    struct_data: &ErrorStruct,
    mod_name: &Ident,
    ty: &Ident,
  ) -> Result<FromImplFor> {
    let ctor = match &struct_data.fields {
      Fields::Unit => Constructor::Unit,
      Fields::Named(f) if f.len() == 0 => Constructor::Named,
      Fields::Unnamed(f) if f.len() == 0 => Constructor::Unnamed,
      Fields::Named(_) => {
        return Err(
          Error::unsupported_shape("Can't derive From for context types that has fields.")
            .with_span(&self.path),
        )
      }
      Fields::Unnamed(_) => {
        return Err(
          Error::unsupported_shape("Can't derive From for context types that has fields.")
            .with_span(&self.path),
        )
      }
    };

    Ok(FromImplFor {
      from_impl: self.clone(),
      mod_name: mod_name.clone(),
      owner: ty.clone(),
      ctor,
    })
  }

  pub fn for_variant(
    &self,
    variant: &ErrorVariant,
    mod_name: &Ident,
    ty: &Ident,
  ) -> Result<FromImplFor> {
    let ctor = match &variant.fields {
      Fields::Unit => Constructor::VariantUnit(variant.ident.clone()),
      Fields::Named(f) if f.len() == 0 => Constructor::VariantNamed(variant.ident.clone()),
      Fields::Unnamed(f) if f.len() == 0 => Constructor::VariantUnnamed(variant.ident.clone()),
      Fields::Named(_) => {
        return Err(
          Error::unsupported_shape("Can't derive From for context types that has fields.")
            .with_span(&self.path),
        )
      }
      Fields::Unnamed(_) => {
        return Err(
          Error::unsupported_shape("Can't derive From for context types that has fields.")
            .with_span(&self.path),
        )
      }
    };

    Ok(FromImplFor {
      from_impl: self.clone(),
      mod_name: mod_name.clone(),
      owner: ty.clone(),
      ctor,
    })
  }
}

impl FromMeta for FromImpl {
  fn from_path(value: &Path) -> Result<Self> {
    Ok(FromImpl {
      path: value.clone(),
    })
  }
}

enum Constructor {
  Unit,                  /* struct Foo; */
  Named,                 /* struct Foo{}; */
  Unnamed,               /* struct Foo(); */
  VariantUnit(Ident),    /* enum Foo { Variant } */
  VariantNamed(Ident),   /* enum Foo { Variant{} } */
  VariantUnnamed(Ident), /* enum Foo { Variant() } */
}

pub(crate) struct FromImplFor {
  from_impl: FromImpl,
  mod_name: Ident,
  owner: Ident,
  ctor: Constructor,
}

impl<'a> ToTokens for FromImplFor {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let mod_name = &self.mod_name;
    let owner = &self.owner;
    let path = &self.from_impl.path;
    let inst = match &self.ctor {
      Constructor::Unit => quote! { #owner },
      Constructor::Named => quote! { (#owner {}) },
      Constructor::Unnamed => quote! { (#owner()) },
      Constructor::VariantUnit(v) => quote! { #owner::#v },
      Constructor::VariantNamed(v) => quote! { (#owner::#v {}) },
      Constructor::VariantUnnamed(v) => quote! { (#owner::#v()) },
    };

    tokens.extend(quote! {
      impl ::std::convert::From<#path> for #mod_name::Error {
        fn from(err: #path) -> Self {
          #inst.into_error(err)
        }
      }
    })
  }
}
