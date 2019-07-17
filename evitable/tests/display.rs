extern crate evitable;

use evitable::ErrorContext;

mod unit_struct {
  use super::*;

  #[derive(ErrorContext)]
  #[evitable(description = "Test error")]
  pub(super) struct Test;

  #[test]
  fn test() {
    assert_eq!(Test.to_string(), "Test error");
  }
}

mod named_struct {
  use super::*;

  #[derive(ErrorContext)]
  #[evitable(description("Test error, code={}", code))]
  pub(super) struct Test {
    code: u8,
  }

  #[test]
  fn test() {
    assert_eq!(Test { code: 42 }.to_string(), "Test error, code=42");
  }
}

mod unnamed_struct {
  use super::*;

  #[derive(ErrorContext)]
  #[evitable(description("Test({})", 0))]
  pub(super) struct Test(u8);

  #[test]
  fn test() {
    assert_eq!(Test(42).to_string(), "Test(42)");
  }
}

mod all_enum {
  use super::*;

  #[derive(ErrorContext)]
  pub(super) enum Test {
    #[evitable(description = "Io")]
    Io,

    #[evitable(description("Fmt {{ code: {} }}", code))]
    Fmt { code: u8 },

    #[evitable(description("Utf8({})", 0))]
    Utf8(u8),
  }

  #[test]
  fn test() {
    assert_eq!(Test::Io.to_string(), "Io");
    assert_eq!(Test::Fmt { code: 42 }.to_string(), "Fmt { code: 42 }");
    assert_eq!(Test::Utf8(42).to_string(), "Utf8(42)");
  }
}
