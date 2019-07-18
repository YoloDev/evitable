#[cfg(feature = "derive")]
extern crate evitable_derive;

use std::fmt::{Debug, Display};

#[cfg(feature = "derive")]
pub use evitable_derive::ErrorContext;

pub use std::error::Error as StdError;

pub trait ErrorKind: PartialEq + Display {}

pub trait EvitableError: StdError + Sized + 'static {
  type Kind: ErrorKind;
  type Context: ErrorContext<Error = Self, Kind = Self::Kind>;

  fn new(context: Self::Context, source: Option<Box<dyn StdError + 'static>>) -> Self;
  fn kind(&self) -> Self::Kind;
  fn context(&self) -> &Self::Context;

  #[inline]
  fn from_context(context: Self::Context) -> Self {
    Self::new(context, None)
  }

  #[inline]
  fn from_error_context<S: StdError + 'static>(context: Self::Context, error: S) -> Self {
    Self::new(context, Some(Box::new(error)))
  }
}

pub trait ErrorContext: Display + Debug + Sized + 'static {
  type Kind: ErrorKind;
  type Error: EvitableError<Context = Self, Kind = Self::Kind>;

  fn kind(&self) -> Self::Kind;

  #[inline]
  fn into_error<S: StdError + 'static>(self, source: S) -> Self::Error {
    Self::Error::from_error_context(self, source)
  }
}

pub trait OptionExt<T, C: ErrorContext> {
  fn context(self, f: impl FnOnce() -> C) -> Result<T, C::Error>;
}

pub trait ResultExt<T, E, C: ErrorContext>: OptionExt<T, C> {
  fn context_with(self, f: impl FnOnce(&E) -> C) -> Result<T, C::Error>;
}

impl<T, C: ErrorContext> OptionExt<T, C> for Option<T> {
  fn context(self, f: impl FnOnce() -> C) -> Result<T, C::Error> {
    self.ok_or_else(|| C::Error::from_context(f()))
  }
}

impl<T, E: StdError + 'static, C: ErrorContext> OptionExt<T, C> for Result<T, E> {
  fn context(self, f: impl FnOnce() -> C) -> Result<T, C::Error> {
    self.map_err(|e| C::Error::from_error_context(f(), e))
  }
}

impl<T, E: StdError + 'static, C: ErrorContext> ResultExt<T, E, C> for Result<T, E> {
  fn context_with(self, f: impl FnOnce(&E) -> C) -> Result<T, C::Error> {
    self.map_err(|e| C::Error::from_error_context(f(&e), e))
  }
}

#[macro_export]
macro_rules! ensure {
  ($test:expr, $ctx:expr) => {
    if !($test) {
      let _ = Err($ctx)?;
    }
  };
}

#[macro_export]
macro_rules! fail {
  ($ctx:expr) => {
    let _ = Err($ctx)?;
  };
}
