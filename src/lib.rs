#![allow(dead_code)]

//!
//! Level 1
//! - plus and minus years; 1BCE=+0000, 2BCE=-0001, also handle 0000 and -0000 obviously

mod helpers;
pub mod level0;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum ParseError {
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

struct Edtf {
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
