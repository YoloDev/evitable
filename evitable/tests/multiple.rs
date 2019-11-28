extern crate evitable;

use evitable::*;

#[evitable(description = "Not found")]
pub struct NotFoundContext;

#[evitable(description = "Bad request")]
pub struct BadRequestContext;

#[test]
pub fn test_both() {
  let _ = NotFoundError::from(NotFoundContext);
  let _ = BadRequestError::from(BadRequestContext);
}
