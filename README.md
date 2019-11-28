# Evitable

[![Crate](https://img.shields.io/crates/v/evitable.svg)](https://crates.io/crates/evitable)
[![Documentation](https://docs.rs/evitable/badge.svg)](https://docs.rs/evitable)
[![Build Status](https://dev.azure.com/yolo-dev/yolodev-github-projects/_apis/build/status/YoloDev.evitable?branchName=master)](https://dev.azure.com/yolo-dev/yolodev-github-projects/_build/latest?definitionId=2&branchName=master)

Evitable is a library for easily creating and using custom
error types in libraries. It's intended to make the creation
of custom domain specific error types easier, as well as
reduce the noise related to converting from underlying errors
to domain specific errors, while keeping the underlying error
as `source()`. This crate by default has a feature called
`derive` enabled, which enables deriving ErrorContexts.

## Quick example

This example showcases a typical usecase of calling some API that
(pretends) to read a file, only to fail, and then converts the
error into a domain specific error.

```rust
use evitable::*;

// Typically, this is in another file
mod error {
  use super::*;

  #[evitable]
  pub enum ParseContext {
    #[evitable(description = "Io error", from = std::io::Error)]
    Io,

    #[evitable(description("Invalid token. Expected {}, was {}.", expected, actual))]
    InvalidToken {
       expected: String,
       actual: String,
    },
  }
}

use error::*;

// pretend token type
#[derive(Debug)]
pub enum Token {
  EndOfFile,
}

fn read_file() -> Result<String, std::io::Error> {
  // we're pretending to read a file here
  Err(std::io::Error::from(std::io::ErrorKind::NotFound))
}

// main function
fn parse_file() -> ParseResult<Token> {
  let content = read_file()?;
  ensure!(content == "EOF", ParseContext::InvalidToken {
    expected: "EOF".to_owned(),
    actual: content,
  });

  Ok(Token::EndOfFile)
}

let result = parse_file();
let err = result.unwrap_err();
assert_eq!(err.kind(), ParseErrorKind::Io);
```
