use syn::Ident;

enum Unreachable {}

impl std::fmt::Debug for Unreachable {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    unreachable!()
  }
}

pub enum Fields<T> {
  Named(Vec<(Ident, T)>),
  Unnamed(Vec<(usize, T)>),
  Unit,
}

impl<T> Fields<T> {
  pub fn iter<'a>(&'a self) -> FieldsIter<'a, T> {
    match self {
      Fields::Named(v) => FieldsIter::Named(v.iter()),
      Fields::Unnamed(v) => FieldsIter::Unnamed(v.iter()),
      Fields::Unit => FieldsIter::Unit,
    }
  }
}

pub trait MapFields<T> {
  fn try_map_fields<E, U>(self, f: impl Fn(T) -> Result<U, E>) -> Result<Fields<U>, E>;
  fn map_fields<U>(self, f: impl Fn(T) -> U) -> Fields<U>
  where
    Self: Sized,
  {
    self
      .try_map_fields(|field| Result::<_, Unreachable>::Ok(f(field)))
      .unwrap()
  }
}

impl MapFields<syn::Field> for syn::Fields {
  fn try_map_fields<E, U>(self, f: impl Fn(syn::Field) -> Result<U, E>) -> Result<Fields<U>, E> {
    match self {
      syn::Fields::Named(fields) => {
        let mut v = Vec::with_capacity(fields.named.len());
        for field in fields.named {
          let ident = field.ident.clone().unwrap();
          v.push((ident, f(field)?))
        }

        Ok(Fields::Named(v))
      }

      syn::Fields::Unnamed(fields) => {
        let mut v = Vec::with_capacity(fields.unnamed.len());
        for (index, field) in fields.unnamed.into_iter().enumerate() {
          v.push((index, f(field)?))
        }

        Ok(Fields::Unnamed(v))
      }

      syn::Fields::Unit => Ok(Fields::Unit),
    }
  }
}

impl<'a> MapFields<&'a syn::Field> for &'a syn::Fields {
  fn try_map_fields<E, U>(
    self,
    f: impl Fn(&'a syn::Field) -> Result<U, E>,
  ) -> Result<Fields<U>, E> {
    match self {
      syn::Fields::Named(fields) => {
        let mut v = Vec::with_capacity(fields.named.len());
        for field in &fields.named {
          let ident = field.ident.clone().unwrap();
          v.push((ident, f(field)?))
        }

        Ok(Fields::Named(v))
      }

      syn::Fields::Unnamed(fields) => {
        let mut v = Vec::with_capacity(fields.unnamed.len());
        for (index, field) in fields.unnamed.iter().enumerate() {
          v.push((index, f(field)?))
        }

        Ok(Fields::Unnamed(v))
      }

      syn::Fields::Unit => Ok(Fields::Unit),
    }
  }
}

impl<T> MapFields<T> for Fields<T> {
  fn try_map_fields<E, U>(self, f: impl Fn(T) -> Result<U, E>) -> Result<Fields<U>, E> {
    match self {
      Fields::Named(fields) => {
        let mut v = Vec::with_capacity(fields.len());
        for field in fields {
          v.push((field.0, f(field.1)?))
        }

        Ok(Fields::Named(v))
      }

      Fields::Unnamed(fields) => {
        let mut v = Vec::with_capacity(fields.len());
        for (index, field) in fields {
          v.push((index, f(field)?))
        }

        Ok(Fields::Unnamed(v))
      }

      Fields::Unit => Ok(Fields::Unit),
    }
  }
}

pub enum FieldsIter<'a, T> {
  Named(std::slice::Iter<'a, (Ident, T)>),
  Unnamed(std::slice::Iter<'a, (usize, T)>),
  Unit,
}

impl<'a, T> Iterator for FieldsIter<'a, T> {
  type Item = (Option<&'a Ident>, &'a T);

  fn next(&mut self) -> Option<Self::Item> {
    match self {
      FieldsIter::Named(iter) => match iter.next() {
        None => None,
        Some(tpl) => Some((Some(&tpl.0), &tpl.1)),
      },

      FieldsIter::Unnamed(iter) => match iter.next() {
        None => None,
        Some((_, val)) => Some((None, val)),
      },

      FieldsIter::Unit => None,
    }
  }
}
