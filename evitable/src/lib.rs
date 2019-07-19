//! # Evitable
//!
//! Evitable is a library for easily creating and using custom
//! error types in libraries. It's intended to make the creation
//! of custom domain specific error types easier, as well as
//! reduce the noise related to converting from underlying errors
//! to domain specific errors, while keeping the underlying error
//! as `source()`. This crate by default has a feature called
//! `derive` enabled, which enables deriving [ErrorContext](ErrorContext)s.
//!
//! ## Quick example
//!
//! This example showcases a typical usecase of calling some API that
//! (pretends) to read a file, only to fail, and then converts the
//! error into a domain specific error.
//!
//! ```rust
//! use evitable::*;
//!
//! // Typically, this is in another file
//! mod error {
//!   use super::*;
//! #  use evitable_derive::*;
//!
//!   #[derive(ErrorContext)]
//!   pub enum Context {
//!     #[evitable(description = "Io error", from = std::io::Error)]
//!     Io,
//!
//!     #[evitable(description("Invalid token. Expected {}, was {}.", expected, actual))]
//!     InvalidToken {
//!        expected: String,
//!        actual: String,
//!     },
//!   }
//! }
//!
//! use error::*;
//!
//! // pretend token type
//! #[derive(Debug)]
//! pub enum Token {
//!   EndOfFile,
//! }
//!
//! fn read_file() -> std::result::Result<String, std::io::Error> {
//!   // we're pretending to read a file here
//!   Err(std::io::Error::from(std::io::ErrorKind::NotFound))
//! }
//!
//! // main function
//! fn parse_file() -> Result<Token> {
//!   let content = read_file()?;
//!   ensure!(content == "EOF", Context::InvalidToken {
//!     expected: "EOF".to_owned(),
//!     actual: content,
//!   });
//!
//!   Ok(Token::EndOfFile)
//! }
//!
//! let result = parse_file();
//! let err = result.unwrap_err();
//! assert_eq!(err.kind(), evitable_context::ErrorKind::Io);
//! ```

extern crate backtrace;

#[cfg(feature = "derive")]
extern crate evitable_derive;

use std::fmt::{Debug, Display};

#[cfg(feature = "derive")]
pub use evitable_derive::ErrorContext;

pub use backtrace::Backtrace;
#[doc(hidden)]
pub use std::error::Error as StdError;

/// Trait for "error kinds". An `ErrorKind` enum is generated for
/// every `#[derive(ErrorContext)]` which typically just contains
/// variants for each error variant (or just a single variant in
/// case of error structs).
pub trait EvitableErrorKind: PartialEq + Display {}

/// Trait implemented for all error types generated by `#[derive(ErrorContext)]`.
/// Allows for creating new errors from the [ErrorContext](ErrorContext), and
/// allows getting the error kind.
pub trait EvitableError: StdError + Sized + 'static {
  /// Error kind type. Generated by `#[derive(ErrorContext)]`.
  type Kind: EvitableErrorKind;

  /// Error context type. The type marked by `#[derive(ErrorContext)]`.
  type Context: ErrorContext<Error = Self, Kind = Self::Kind>;

  /// Create a new error instance, based on an error context and an optional source error.
  /// Instead of using this directly, see [from_error_context](EvitableError::from_error_context) when
  /// wanting to create error instances with source errors, and [from_context](EvitableError::from_context)
  /// when not. Derived implementations of this trait also implements `From<ErrorContext>`, so using
  /// [from](std::convert::From::from) or [into](std::convert::Into::into) is also an option.
  ///
  /// # Arguments
  ///
  /// * `context` - Error context
  /// * `source` - Optional error source
  ///
  /// # Example
  ///
  /// ```rust
  ///# use evitable::*;
  ///  #[derive(ErrorContext)]
  ///  #[evitable(description = "Error")]
  ///  pub struct Context;
  ///
  ///  // Later
  ///# fn main() {
  ///  let error = Error::new(Context, None);
  ///# }
  /// ```
  fn new(context: Self::Context, source: Option<Box<dyn StdError + 'static>>) -> Self;

  /// Get the error kind.
  ///
  /// # Example
  ///
  /// ```rust
  ///# use evitable::*;
  ///  #[derive(ErrorContext)]
  ///  pub enum Context {
  ///    #[evitable(description("Io error ({})", 0))]
  ///    Io(u8),
  ///
  ///    #[evitable(description = "Fmt error")]
  ///    Fmt,
  ///  }
  ///
  ///  // Later
  ///# fn main() {
  ///  let error = Error::new(Context::Io(42), None);
  ///  let t =
  ///    match error.kind() {
  ///      evitable_context::ErrorKind::Io => "Io",
  ///      evitable_context::ErrorKind::Fmt => "Fmt",
  ///      _ => "Other",
  ///    };
  ///
  ///  assert_eq!(t, "Io");
  ///# }
  /// ```
  fn kind(&self) -> Self::Kind;

  /// Get the error context.
  fn context(&self) -> &Self::Context;

  /// Get backtrace.
  fn backtrace(&self) -> &Backtrace;

  /// Create a new error instance from an error context.
  ///
  /// # Arguments
  ///
  /// * `context` - Error context
  ///
  /// # Example
  ///
  /// ```rust
  ///# use evitable::*;
  ///  #[derive(ErrorContext)]
  ///  #[evitable(description = "Error")]
  ///  pub struct Context;
  ///
  ///  // Later
  ///# fn main() {
  ///  let error = Error::from_context(Context);
  ///# }
  /// ```
  #[inline]
  fn from_context(context: Self::Context) -> Self {
    Self::new(context, None)
  }

  /// Create a new error instance from an error context and a source error.
  ///
  /// # Arguments
  ///
  /// * `context` - Error context
  /// * `source` - Error source
  ///
  /// # Example
  ///
  /// ```rust
  ///# use evitable::*;
  ///# use std::io;
  ///  #[derive(ErrorContext)]
  ///  #[evitable(description = "Error")]
  ///  pub struct Context;
  ///
  ///  // Later
  ///# fn main() {
  ///  let io_error = io::Error::from(io::ErrorKind::NotFound);
  ///  let error = Error::from_error_context(Context, io_error);
  ///# }
  /// ```
  #[inline]
  fn from_error_context<S: StdError + 'static>(context: Self::Context, error: S) -> Self {
    Self::new(context, Some(Box::new(error)))
  }
}

/// Error context trait, typically used with `#[derive(ErrorContext)]`.
/// This produces Error and ErrorKind types for the given context.
pub trait ErrorContext: Display + Debug + Sized + 'static {
  /// Associated error kind enum.
  type Kind: EvitableErrorKind;

  /// Associated error struct.
  type Error: EvitableError<Context = Self, Kind = Self::Kind>;

  /// Get the error kind.
  ///
  /// # Example
  ///
  /// ```rust
  ///# use evitable::*;
  ///  #[derive(ErrorContext)]
  ///  pub enum Context {
  ///    #[evitable(description("Io error ({})", 0))]
  ///    Io(u8),
  ///
  ///    #[evitable(description = "Fmt error")]
  ///    Fmt,
  ///  }
  ///
  ///  // Later
  ///# fn main() {
  ///  let error = Context::Io(42);
  ///  let t =
  ///    match error.kind() {
  ///      evitable_context::ErrorKind::Io => "Io",
  ///      evitable_context::ErrorKind::Fmt => "Fmt",
  ///      _ => "Other",
  ///    };
  ///
  ///  assert_eq!(t, "Io");
  ///# }
  /// ```
  fn kind(&self) -> Self::Kind;

  /// Convert the current context into an error.
  ///
  /// # Arguments
  ///
  /// * `source` - Error source
  ///
  /// # Example
  ///
  /// ```rust
  ///# use evitable::*;
  ///# use std::io;
  ///  #[derive(ErrorContext)]
  ///  #[evitable(description = "Error")]
  ///  pub struct Context(u8);
  ///
  ///  // Later
  ///# fn main() {
  ///  let io_error = io::Error::from(io::ErrorKind::NotFound);
  ///  let error = Context(42).into_error(io_error);
  ///# }
  /// ```
  #[inline]
  fn into_error<S: StdError + 'static>(self, source: S) -> Self::Error {
    Self::Error::from_error_context(self, source)
  }
}

/// Extension trait for option and result types for easy convertion
/// to evitable errors.
pub trait OptionExt<T, C: ErrorContext> {
  /// Convert to a [Result](std::result::Result) where ther error
  /// case is constructed from the given context factory.
  ///
  /// # Arguments
  ///
  /// * `f` - Error context factory
  ///
  /// # Example
  ///
  /// ```rust
  ///# use evitable::*;
  ///# use std::io;
  ///  #[derive(ErrorContext)]
  ///  #[evitable(description = "Error")]
  ///  pub struct Context(u8);
  ///
  ///  // Later
  ///# fn main() {
  ///  let option_error: Option<u8> = None;
  ///  let io_error: std::io::Result<u8> = Err(io::Error::from(io::ErrorKind::NotFound));
  ///  let option_ok: Option<u8> = Some(42);
  ///  let io_ok: std::io::Result<u8> = Ok(42);
  ///
  ///  // The functions here are called
  ///  let error_with_source = io_error.context(|| Context(42));
  ///  let error_without_source = option_error.context(|| Context(42));
  ///
  ///  // The functions here will never be called
  ///  let ok_option = option_ok.context(|| Context(unimplemented!()));
  ///  let ok_result = io_ok.context(|| Context(unimplemented!()));
  ///
  ///  assert!(error_with_source.unwrap_err().source().is_some());
  ///  assert!(error_without_source.unwrap_err().source().is_none());
  ///
  ///  ok_option.unwrap();
  ///  ok_result.unwrap();
  ///# }
  fn context(self, f: impl FnOnce() -> C) -> Result<T, C::Error>;
}

/// Extension trait for result types (and other types carrying
/// errors with them).
pub trait ResultExt<T, E, C: ErrorContext>: OptionExt<T, C> {
  /// Convert to a [Result](std::result::Result) where ther error
  /// case is constructed from the given context factory which also
  /// accepts a reference to the original error.
  ///
  /// # Arguments
  ///
  /// * `f` - Error context factory
  ///
  /// # Example
  ///
  /// ```rust
  ///# use evitable::*;
  ///# use std::io;
  ///  #[derive(ErrorContext)]
  ///  #[evitable(description = "Error")]
  ///  pub struct Context(String);
  ///
  ///  // Later
  ///# fn main() {
  ///  let io_error: std::io::Result<u8> = Err(io::Error::from(io::ErrorKind::NotFound));
  ///  let io_ok: std::io::Result<u8> = Ok(42);
  ///
  ///  // The functions here are called
  ///  let error_with_source = io_error.context_with(|e| Context(format!("{:?}", e)));
  ///
  ///  // The functions here will never be called
  ///  let ok_result = io_ok.context_with(|e| Context(unimplemented!()));
  ///
  ///  assert!(error_with_source.unwrap_err().source().is_some());
  ///
  ///  ok_result.unwrap();
  ///# }
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

/// Utility macro to return errors if a given condition is false.
///
/// # Arguments
///
/// * `test` - Test expression
/// * `ctx` - Error context - only evaluated if test expression is false
///
/// # Example
///
/// ```rust
///# use evitable::*;
///# use std::io;
///  #[derive(ErrorContext, PartialEq)]
///  #[evitable(description = "Error")]
///  pub struct Context(u8);
///
///  fn validate(val: u8) -> Result<()> {
///    ensure!(val < 10, Context(val));
///    Ok(())
///  }
///
///# fn main() {
///  validate(5).unwrap();
///  assert_eq!(validate(15).unwrap_err().context(), &Context(15));
///# }
/// ```
#[macro_export]
macro_rules! ensure {
  ($test:expr, $ctx:expr) => {
    if !($test) {
      let _ = Err($ctx)?;
    }
  };
}

/// Utility macro to return errors.
///
/// # Arguments
///
/// * `ctx` - Error context
///
/// # Example
///
/// ```rust
///# use evitable::*;
///# use std::io;
///  #[derive(ErrorContext, PartialEq)]
///  #[evitable(description = "Error")]
///  pub struct Context(u8);
///
///  fn do_fail(val: u8) -> Result<()> {
///    fail!(Context(val));
///    Ok(())
///  }
///
///# fn main() {
///  assert_eq!(do_fail(15).unwrap_err().context(), &Context(15));
///# }
/// ```
#[macro_export]
macro_rules! fail {
  ($ctx:expr) => {
    let _ = Err($ctx)?;
  };
}
