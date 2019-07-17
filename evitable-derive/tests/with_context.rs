extern crate evitable;

use evitable::*;

#[derive(ErrorContext)]
pub enum Context {
  #[evitable(description("Custom error ({})", code))]
  Custom {
    #[evitable(include_in_kind)]
    code: u8,
  },

  #[evitable(description = "Io", from = std::io::Error)]
  Io,
}

#[test]
fn from_io() {
  use std::io;

  fn fail() -> Result<()> {
    let result = Err(io::Error::from(io::ErrorKind::NotFound))?;

    Ok(result)
  }

  let err = fail().unwrap_err();
  assert_eq!(err.kind(), ErrorKind::Io);
  assert!(err.source().is_some());
}

#[test]
fn from_io_with_context() {
  use std::io;

  fn fail() -> Result<()> {
    let result =
      Err(io::Error::from(io::ErrorKind::NotFound)).context(|| Context::Custom { code: 10 })?;

    Ok(result)
  }

  let err = fail().unwrap_err();
  assert_eq!(err.kind(), ErrorKind::Custom { code: 10 });
  assert!(err.source().is_some());
}

#[test]
fn option_context() {
  fn fail() -> Result<()> {
    let result = None.context(|| Context::Custom { code: 42 })?;

    Ok(result)
  }

  let err = fail().unwrap_err();
  assert_eq!(err.kind(), ErrorKind::Custom { code: 42 });
  assert!(err.source().is_none());
}
