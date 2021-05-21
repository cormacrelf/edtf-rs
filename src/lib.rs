#![allow(dead_code)]

mod helpers;
pub mod level0;
pub mod level1;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// A field is out of the permitted range.
    OutOfRange,

    /// The input string has some invalid character sequence for given formatting items.
    Invalid,
}

impl std::error::Error for ParseError {}

use core::fmt;
impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
