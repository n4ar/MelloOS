//! false - do nothing, unsuccessfully

use crate::error::Result;

pub fn main(_argv: &'static [&'static str]) -> Result<i32> {
    Ok(1)
}
