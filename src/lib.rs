#![allow(dead_code)]

#![cfg_attr(docsrs, feature(doc_cfg))]

pub(crate) mod common;
pub(crate) mod helpers;
mod level0;
mod level1;
mod level2;
pub use level0::api as level_0;
pub use level1::api as level_1;
#[doc(hidden)]
pub use level2::api as level_2;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// A field is out of the permitted range.
    OutOfRange,

    /// The input string has some invalid character sequence.
    Invalid,
}

impl std::error::Error for ParseError {}

use core::fmt;
impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
