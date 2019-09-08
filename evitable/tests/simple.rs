extern crate evitable;

use evitable::*;

#[evitable(description = "Error")]
struct Context;

#[test]
fn test() {
  let _ = Error::new(Context, None);
}
