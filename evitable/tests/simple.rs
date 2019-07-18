extern crate evitable;

use evitable::*;

#[derive(ErrorContext)]
#[evitable(description = "Error")]
struct Context;

#[test]
fn test() {
  let _ = Error::new(Context, None);
}
