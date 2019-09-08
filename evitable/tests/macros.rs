extern crate evitable;

use evitable::*;

#[evitable]
pub enum Context {
  #[evitable(description = "Io")]
  Io,
}

#[test]
fn ensure() {
  fn fail() -> Result<()> {
    ensure!(false, Context::Io);

    Ok(())
  }

  let ret = fail();
  assert!(ret.is_err());
}

#[test]
fn fail() {
  fn fail() -> Result<()> {
    fail!(Context::Io);

    Ok(())
  }

  let ret = fail();
  assert!(ret.is_err());
}
