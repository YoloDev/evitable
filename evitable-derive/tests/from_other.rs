extern crate evitable;

use evitable::*;
use std::io::Error as IoError;

mod unit_struct {
  use super::*;

  #[derive(ErrorContext)]
  #[evitable(description = "Err", from = IoError, from = std::fmt::Error)]
  pub(super) struct Test;
}

mod named_struct {
  use super::*;

  #[derive(ErrorContext)]
  #[evitable(description = "Err", from = IoError, from = std::fmt::Error)]
  pub(super) struct Test {}
}

mod unnamed_struct {
  use super::*;

  #[derive(ErrorContext)]
  #[evitable(description = "Err", from = IoError, from = std::fmt::Error)]
  pub(super) struct Test();
}

mod all_enum {
  use super::*;

  #[derive(ErrorContext)]
  pub(super) enum Test {
    #[evitable(description = "Io", from = IoError)]
    Io,

    #[evitable(description = "Fmt", from = std::fmt::Error)]
    Fmt {},

    #[evitable(description = "Utf8", from = std::str::Utf8Error)]
    Utf8(),
  }
}
