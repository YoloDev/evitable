extern crate evitable;

use evitable::*;
use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct IncrementWhenCloned(u8);

impl Clone for IncrementWhenCloned {
  fn clone(&self) -> Self {
    IncrementWhenCloned(self.0 + 1)
  }
}

impl Copy for IncrementWhenCloned {}

impl fmt::Display for IncrementWhenCloned {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    fmt::Display::fmt(&self.0, f)
  }
}

mod clone {
  use super::*;

  #[derive(ErrorContext)]
  #[evitable(description("Some test: {}", cloned))]
  pub(super) struct Test {
    #[evitable(include_in_kind, clone)]
    cloned: IncrementWhenCloned,
  }

  #[test]
  pub fn error_context_cloned() {
    let err = Test {
      cloned: IncrementWhenCloned(0),
    };

    let kind = err.kind();
    assert_eq!(
      kind,
      evitable_test::ErrorKind::Test {
        cloned: IncrementWhenCloned(1)
      }
    );
  }

  #[test]
  pub fn display() {
    let err = Test {
      cloned: IncrementWhenCloned(0),
    };

    let display_str = format!("{}", err);
    assert_eq!("Some test: 0", display_str);
  }
}

mod copy {
  use super::*;

  #[derive(ErrorContext)]
  #[evitable(description("Other test: {}", copied))]
  pub(super) struct Test {
    #[evitable(include_in_kind)]
    copied: IncrementWhenCloned,
  }

  #[test]
  pub fn error_context_copied() {
    let err = Test {
      copied: IncrementWhenCloned(0),
    };

    let kind = err.kind();
    assert_eq!(
      kind,
      evitable_test::ErrorKind::Test {
        copied: IncrementWhenCloned(0)
      }
    );
  }
}
