use super::*;
use std::borrow::Borrow;

struct Attr {
  meta: Meta,
  name: String,
  used: bool,
}

pub(crate) struct Attrs {
  attrs: Vec<Attr>,
}

impl Attrs {
  pub fn from_attributes<A, I>(iter: I) -> Result<Self>
  where
    A: Borrow<Attribute>,
    I: IntoIterator<Item = A>,
  {
    let mut vec = Vec::new();
    for item in iter {
      let attr = item.borrow();
      if let Ok(meta) = attr.meta() {
        if meta.is("evitable") {
          if let Meta::List(items) = meta {
            for item in items {
              match item {
                NestedMeta::Literal(l) => return Err(Error::unexpected_lit_type(&l).with_span(&l)),
                NestedMeta::Meta(m) => match m.path().single_ident() {
                  None => return Err(Error::unexpected_type("path").with_span(m.path())),
                  Some(ident) => {
                    let name = ident.to_string();
                    vec.push(Attr {
                      meta: m,
                      name,
                      used: false,
                    })
                  }
                },
              }
            }
          } else {
            return Err(Error::unexpected_type("Expected meta list.").with_span(attr));
          }
        }
      }
    }

    Ok(Attrs { attrs: vec })
  }

  pub fn get_optional<T: FromMeta, S: AsRef<str>>(&mut self, name: S) -> Result<Option<T>> {
    let name = name.as_ref();
    for attr in self.attrs.iter_mut() {
      if &attr.name == name {
        let result = FromMeta::from_meta(&attr.meta);
        attr.used = true;
        return result.map(Some);
      }
    }

    Ok(None)
  }

  pub fn get_required<T: FromMeta, S: AsRef<str>, O: Spanned>(
    &mut self,
    name: S,
    span: &O,
  ) -> Result<T> {
    self
      .get_optional(&name)
      .and_then(|v| v.ok_or_else(|| Error::missing_field(name.as_ref()).with_span(span)))
  }

  pub fn get_list<T: FromMeta, S: AsRef<str>>(&mut self, name: S) -> Result<Vec<T>> {
    let name = name.as_ref();
    let mut ret = Vec::new();
    for attr in self.attrs.iter_mut() {
      if &attr.name == name {
        let items = <Vec<T> as FromMeta>::from_meta(&attr.meta)?;
        attr.used = true;
        ret.extend(items);
      }
    }

    Ok(ret)
  }

  pub fn ensure_used(&self) -> Result<()> {
    for attr in self.attrs.iter() {
      if !attr.used {
        return Err(Error::unknown_field(&attr.name).with_span(attr.meta.path()));
      }
    }

    Ok(())
  }
}
