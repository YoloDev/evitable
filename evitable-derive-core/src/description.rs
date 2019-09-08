use super::*;
use syn::{Lit, LitInt, LitStr};

#[derive(Debug)]
pub(crate) enum FieldRef {
  Ident(Ident),
  Index(LitInt),
}

impl ToTokens for FieldRef {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    match self {
      FieldRef::Ident(i) => i.to_tokens(tokens),
      FieldRef::Index(i) => i.to_tokens(tokens),
    }
  }
}

impl FromMeta for FieldRef {
  fn from_ident(value: &Ident) -> Result<Self> {
    Ok(FieldRef::Ident(value.clone()))
  }

  fn from_string<S: Spanned>(value: &str, span: &S) -> Result<Self> {
    Ok(FieldRef::Ident(syn::Ident::new(value, span.span())))
  }

  fn from_lit(lit: &Lit) -> Result<Self> {
    (match lit {
      Lit::Bool(b) => Self::from_bool(b.value, b),
      Lit::Str(s) => Self::from_string(&s.value(), s),
      Lit::Char(c) => Self::from_char(c.value(), c),
      Lit::Int(i) => Ok(FieldRef::Index(i.clone())),
      _ => Err(Error::unexpected_lit_type(lit)),
    })
    .map_err(|e| e.with_span(lit))
  }
}

#[derive(Debug)]
pub(crate) struct FormatExpression {
  format: LitStr,
  args: Vec<FieldRef>,
}

#[derive(Debug)]
pub(crate) enum Description {
  String(LitStr),
  FormatExpression(FormatExpression),
}

impl FromMeta for Description {
  fn from_lit(lit: &Lit) -> Result<Self> {
    (match lit {
      Lit::Bool(b) => Self::from_bool(b.value, b),
      Lit::Str(s) => Ok(Description::String(s.clone())),
      Lit::Char(c) => Self::from_char(c.value(), c),
      Lit::Int(i) => Self::from_int(i.base10_parse()?, i),
      _ => Err(Error::unexpected_lit_type(lit)),
    })
    .map_err(|e| e.with_span(lit))
  }

  fn from_list(items: &[&NestedMeta]) -> Result<Self> {
    match items.len() {
      0 => Self::from_empty(),
      1 => Self::from_nested_meta(&items[0]),
      n => {
        let format = <LitStr as FromMeta>::from_nested_meta(&items[0])?;
        let mut args = Vec::with_capacity(n - 1);
        for item in items.iter().skip(1) {
          args.push(FromMeta::from_nested_meta(item)?);
        }

        Ok(Description::FormatExpression(FormatExpression {
          format,
          args,
        }))
      }
    }
  }
}

pub(crate) struct ResolvedDescription {
  format: LitStr,
  args: Vec<TokenStream>,
}

impl Description {
  fn resolve(
    self,
    lookup: impl Fn(FieldRef) -> Result<TokenStream>,
  ) -> Result<ResolvedDescription> {
    match self {
      Description::String(s) => Ok(ResolvedDescription {
        format: s,
        args: Vec::with_capacity(0),
      }),

      Description::FormatExpression(f) => Ok(ResolvedDescription {
        format: f.format,
        args: f.args.into_iter().map(lookup).collect::<Result<Vec<_>>>()?,
      }),
    }
  }

  pub fn resolve_from_inst<'a, T, I: IntoIdent<'a>>(
    self,
    fields: &Fields<T>,
    ident: I,
  ) -> Result<ResolvedDescription> {
    let ident = &ident.into_ident();

    match fields {
      Fields::Unit => self.resolve(|r| match r {
        FieldRef::Ident(i) => Err(Error::unknown_field(&i.to_string()).with_span(&i)),
        FieldRef::Index(i) => Err(Error::unknown_field(i.base10_digits()).with_span(&i)),
      }),
      Fields::Named(f) => self.resolve(|r| match r {
        FieldRef::Index(i) => Err(Error::unknown_field(i.base10_digits()).with_span(&i)),
        FieldRef::Ident(i) => match f.iter().find(|(f, _)| f.to_string() == i.to_string()) {
          None => Err(Error::unknown_field(&i.to_string()).with_span(&i)),
          Some(_) => Ok(quote! { #ident.#i }),
        },
      }),
      Fields::Unnamed(f) => self.resolve(|r| match r {
        FieldRef::Ident(i) => Err(Error::unknown_field(&i.to_string()).with_span(&i)),
        FieldRef::Index(i) => match f.iter().find(|(f, _)| *f as u64 == i.base10_parse().unwrap()) {
          None => Err(Error::unknown_field(i.base10_digits()).with_span(&i)),
          Some(_) => Ok(quote! { #ident.#i }),
        },
      }),
    }
  }

  pub fn resolve_from_variant<T>(self, fields: &Fields<T>) -> Result<ResolvedDescription> {
    match fields {
      Fields::Unit => self.resolve(|r| match r {
        FieldRef::Ident(i) => Err(Error::unknown_field(&i.to_string()).with_span(&i)),
        FieldRef::Index(i) => Err(Error::unknown_field(i.base10_digits()).with_span(&i)),
      }),
      Fields::Named(f) => self.resolve(|r| match r {
        FieldRef::Index(i) => Err(Error::unknown_field(i.base10_digits()).with_span(&i)),
        FieldRef::Ident(i) => match f.iter().find(|(f, _)| f.to_string() == i.to_string()) {
          None => Err(Error::unknown_field(&i.to_string()).with_span(&i)),
          Some(_) => Ok(quote! { #i }),
        },
      }),
      Fields::Unnamed(f) => self.resolve(|r| match r {
        FieldRef::Ident(i) => Err(Error::unknown_field(&i.to_string()).with_span(&i)),
        FieldRef::Index(i) => match f.iter().find(|(f, _)| *f as u64 == i.base10_parse().unwrap()) {
          None => Err(Error::unknown_field(i.base10_digits()).with_span(&i)),
          Some((i, _)) => Ok({
            let ident = i.into_ident();
            quote! { #ident }
          }),
        },
      }),
    }
  }
}

impl ToTokens for ResolvedDescription {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let format = &self.format;
    if self.args.len() == 0 {
      tokens.extend(quote! {
        f.write_str(#format)
      });
    } else {
      let args = &self.args;
      tokens.extend(quote! {
        write!(f, #format, #(#args),*)
      });
    }
  }
}
