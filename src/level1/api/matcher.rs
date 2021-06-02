//! Helpers for matching dates.
use crate::DateTime;

// for docs
#[allow(unused_imports)]
use super::{Date, Edtf};

// Re-exports
#[doc(no_inline)]
pub use super::Certainty;
#[doc(no_inline)]
pub use super::Season;
#[doc(no_inline)]
pub use super::YearDigits;

/// A month or a day in [DatePrecision]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatePart {
    /// `-XX`. Month or day.
    Unspecified,
    /// e.g. `-04`. Month or day.
    Normal(u32),
}

impl DatePart {
    pub fn value(&self) -> Option<u32> {
        match *self {
            Self::Normal(v) => Some(v),
            Self::Unspecified => None,
        }
    }
}

/// An enum used to conveniently match on a [Date].
///
/// The i32 field in each is a year.
///
/// ```
/// use edtf::level_1::{Date, Certainty::*};
/// use edtf::level_1::matcher::{DatePrecision::*, DatePart::*};
/// match Date::from_ym_masked_day(2019, 4).as_matcher() {
///     (Day(year, Normal(m), Unspecified), Certain) => {
///         assert_eq!(year, 2019);
///         assert_eq!(m, 4);
///     }
///     (Month(year, Normal(m)), Certain) => {
///         panic!("not matched");
///     }
///     _ => panic!("not matched"),
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatePrecision {
    Year(i32, YearDigits),
    Month(i32, DatePart),
    Day(i32, DatePart, DatePart),
    Season(i32, Season),
}

/// See [Matcher::Interval]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Terminal {
    /// An actual date in an interval
    Fixed(DatePrecision, Certainty),
    /// `..`
    Open,
    /// Null terminal. `/2020`
    Unknown,
}

/// An enum used to conveniently match on an [Edtf].
///
/// Note that the various Interval possibilities have some impossible representations, that are
/// never produced by [Edtf::as_matcher].
///
/// ```
/// use edtf::level_1::Edtf;
/// use edtf::level_1::matcher::{
///     Matcher::*, YearDigits::*, Terminal::*, DatePrecision::*, DatePart::*,
///     Certainty::*,
/// };
/// match Edtf::parse("2019/..").unwrap().as_matcher() {
///     Interval(Fixed(Year(y, NoX), Certain), Open) => {
///         assert_eq!(y, 2019);
///     },
///     Interval(Unknown, Unknown) |
///     Interval(Open, Open) |
///     Interval(Open, Unknown) |
///     Interval(Unknown, Open) => {
///         unreachable!()
///     },
///     Interval(Unknown, Fixed(Day(_y, Unspecified, Unspecified), Uncertain)) => {
///         panic!("not matched, demo purposes")
///     },
///     Single(Year(y, NoX), ApproximateUncertain) => {
///         panic!("not matched, demo purposes")
///     },
///     _ => panic!("not matched"),
/// }
/// ```
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Matcher {
    Single(DatePrecision, Certainty),
    WithTime(DateTime),
    Scientific(i64),
    /// in a `Matcher` returned from [Edtf::as_matcher], one of these is guaranteed to be
    /// [Terminal::Fixed]
    Interval(Terminal, Terminal),
}
