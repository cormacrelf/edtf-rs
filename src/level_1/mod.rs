// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright © 2021 Corporation for Digital Scholarship

#![doc = include_str!("README.md")]

pub(crate) mod packed;
mod parser;
mod validate;

mod basic;

/// A set of iterators for stepping through date intervals.
pub mod iter;

#[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
#[cfg(feature = "chrono")]
mod sorting;
#[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
#[cfg(feature = "chrono")]
pub use sorting::{SortOrder, SortOrderEnd, SortOrderStart};

#[cfg(test)]
mod test;

use core::convert::TryInto;
use core::fmt;
use core::num::NonZeroU8;
use core::str::FromStr;

use crate::helpers;
use crate::{DateComplete, DateTime, ParseError, Time, TzOffset};

use self::{
    packed::{DMMask, PackedInt, PackedU8, PackedYear, YearMask},
    parser::{ParsedEdtf, UnvalidatedDate},
};

// TODO: wrap Certainty with one that doesn't expose the implementation detail
pub use packed::Certainty;

/// An EDTF date. Represents a standalone date or one end of a interval.
///
/// ### Equality and comparison
///
/// Presently, only the default derive-generated code is used for PartialEq/PartialOrd. At least
/// you can have some kind of implementation, but the results won't necessarily make sense when
/// some components are masked (unspecified). The current implementation is packed, and the
/// comparisons are done on the packed representation. You may wish to add a wrapper type with its
/// own PartialEq/PartialOrd/Eq/Ord implementations with more complex logic.
///
/// ```
/// use edtf::level_1::{Date, Precision};
/// let d1 = Date::from_precision(Precision::DayOfMonth(2021, 06));
/// let d2 = Date::from_precision(Precision::Day(2021, 06, 09));
/// // d1 is a non-specific day in June, but PartialOrd thinks d1 is "less than" d2.
/// assert!(d1 < d2);
/// assert!( ! (d1 > d2));
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    pub(crate) year: PackedYear,
    pub(crate) month: Option<PackedU8>,
    pub(crate) day: Option<PackedU8>,
    pub(crate) certainty: Certainty,
}

/// Fully represents EDTF Level 1. The representation is lossless.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Edtf {
    /// A full timestamp. `2019-07-15T01:56:00Z`
    DateTime(DateTime),
    /// `2018`, `2019-07-09%`, `1973?`, `1956-XX`, etc
    Date(Date),
    /// `Y170000002`, `Y-170000002`
    ///
    /// Years within the interval -9999..=9999 are explicitly disallowed. Years must contain MORE
    /// THAN four digits.
    YYear(YYear),
    /// `2018/2019`, `2019-12-31/2020-01-15`, etc
    Interval(Date, Date),
    /// `2019/..` (open), `2019/` (unknown)
    IntervalFrom(Date, Terminal),
    /// `../2019` (open), `/2019` (unknown)
    IntervalTo(Terminal, Date),
}

/// Either empty string (unknown start/end date) or `..` in an L1 interval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Terminal {
    /// empty string before or after a slash, e.g. `/2019` or `2019/`
    Unknown,
    /// `..` in e.g. `../2019` or `2019/..`
    Open,
}

/// A season in [Precision]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Season {
    /// 21
    Spring = 21,
    /// 22
    Summer = 22,
    /// 23
    Autumn = 23,
    /// 24
    Winter = 24,
}

impl Season {
    fn from_u32(value: u32) -> Self {
        match value {
            21 => Self::Spring,
            22 => Self::Summer,
            23 => Self::Autumn,
            24 => Self::Winter,
            _ => panic!("invalid season number {}", value),
        }
    }
    fn from_u32_opt(value: u32) -> Option<Self> {
        Some(match value {
            21 => Self::Spring,
            22 => Self::Summer,
            23 => Self::Autumn,
            24 => Self::Winter,
            _ => return None,
        })
    }
}

/// An enum used to conveniently match on a [Date].
///
/// The i32 field in each is a year.
///
/// ```
/// use edtf::level_1::{Date, Certainty, Precision};
/// match Date::parse("2019-04-XX").unwrap().precision() {
///     // 2019-04-XX
///     Precision::DayOfMonth(year, m) => {
///         assert_eq!(year, 2019);
///         assert_eq!(m, 4);
///     }
///     // 2019-XX
///     Precision::MonthOfYear(year) => {
///         panic!("not matched");
///     }
///     // ...
///     _ => panic!("not matched"),
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Precision {
    /// `19XX` => `Century(1900)`; Ends in two zeroes.
    Century(i32),
    /// `193X` => `Decade(1930)`; Ends in a zero.
    Decade(i32),
    /// `1936` => `Year(1936)`; A particular year.
    Year(i32),
    /// `1933-22` => `Season(1933, Season::Summer)`; a particular season in a particular year.
    Season(i32, Season),
    /// `1936-08` => `Month(1936, 08)`; a particular month in a particular year. Month is `1..=12`
    Month(i32, u32),
    /// `1931-08-19` => `Day(1931, 08, 19)`; a full date; a particular day.
    ///
    /// Month `1..-12`, day is valid for that month in that year.
    Day(i32, u32, u32),
    /// `1931-XX` => `MonthOfYear(1931)`; a non-specific month in a particular year.
    MonthOfYear(i32),
    /// `1931-XX-XX` => `DayOfYear(1931)`; a non-specific day in a particular year.
    DayOfYear(i32),
    /// `1931-08-XX` => `DayOfMonth(1931, 08)`; a non-specific day in a particular year.
    ///
    /// Month is `1..=12`
    DayOfMonth(i32, u32),
}

/// Represents a 5+ digit, signed year like `Y12345`, `Y-17000`.
///
#[doc = include_str!("YYear.md")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct YYear(i64);

impl YYear {
    /// Get the year this represents.
    pub fn year(&self) -> i64 {
        self.0
    }

    /// Gets the year. Like [YYear::year] but takes `self`.
    ///
    /// ```
    /// use edtf::level_1::YYear;
    /// assert_eq!(YYear::new_opt(12345).map(YYear::value), Some(12345));
    /// ```
    pub fn value(self) -> i64 {
        self.0
    }

    pub(crate) fn raw(y: i64) -> Self {
        Self(y)
    }

    /// Creates a YYear but **panics** if value is fewer than 5 digits.
    pub fn new(value: i64) -> Self {
        Self::new_opt(value).expect("value outside range for YYear, must be 5-digit year")
    }

    /// If the value is fewer than 5 digits (invalid for `Y`-years), returns None.
    pub fn new_opt(value: i64) -> Option<Self> {
        if helpers::inside_9999(value) {
            return None;
        }
        Some(Self(value))
    }

    /// If the value is fewer than 5 digits (invalid for `Y`-years), returns an [Edtf::Date]
    /// calendar date instead.
    pub fn new_or_cal(value: i64) -> Result<Self, Edtf> {
        if helpers::inside_9999(value) {
            let date = value
                .try_into()
                .ok()
                .and_then(|y| Date::from_ymd_opt(y, 0, 0))
                .map(Edtf::Date)
                .expect("should have already validated as within -9999..=9999");
            return Err(date);
        }
        Ok(Self(value))
    }
}

/// Specifies the number of Xs in `2019`/`201X`/`20XX`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum YearDigits {
    /// `2019`
    NoX,
    /// `201X`
    X,
    /// `20XX`
    XX,
}

#[doc(hidden)]
impl From<YearMask> for YearDigits {
    fn from(ym: YearMask) -> Self {
        match ym {
            YearMask::None => Self::NoX,
            YearMask::OneDigit => Self::X,
            YearMask::TwoDigits => Self::XX,
        }
    }
}

#[doc(hidden)]
impl From<YearDigits> for YearMask {
    fn from(ym: YearDigits) -> Self {
        match ym {
            YearDigits::NoX => YearMask::None,
            YearDigits::X => YearMask::OneDigit,
            YearDigits::XX => YearMask::TwoDigits,
        }
    }
}

/// Represents a possibly-unspecified date component (month or day) or an -XX mask. Pass-through formatting
///
/// ```
/// use edtf::level_1::Component::*;
/// assert_eq!(format!("{}", Unspecified), "X");
/// assert_eq!(format!("{:04}", Unspecified), "XXXX");
/// assert_eq!(format!("{:02}", Value(5)), "05");
/// assert_eq!(format!("{:04}", Value(5)), "0005");
/// ```
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Component {
    /// The component has a value that was specified in the EDTF
    Value(u32),
    /// An `-XX` masked out component
    Unspecified,
}

impl fmt::Display for Component {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // pass the formatter through and let people format as they wish
            Component::Value(val) => val.fmt(f),
            Component::Unspecified => {
                // write as many digits as were requested
                let precision = f.width().unwrap_or(1);
                write!(f, "{:X<1$}", "", precision)
            }
        }
    }
}

impl Component {
    /// Get the value as an option instead of the custom `Component` type.
    pub fn value(self) -> Option<u32> {
        match self {
            Component::Value(v) => Some(v),
            Component::Unspecified => None,
        }
    }

    fn from_packed_filter(packed: PackedU8, range: std::ops::RangeInclusive<u32>) -> Option<Self> {
        let (val, flags) = packed.unpack();
        let val = val as u32;
        if flags.is_masked() {
            Some(Component::Unspecified)
        } else if range.contains(&val) {
            Some(Component::Value(val as u32))
        } else {
            None
        }
    }
    fn from_packed(packed: PackedU8) -> Self {
        let (val, flags) = packed.unpack();
        if flags.is_masked() {
            Component::Unspecified
        } else {
            Component::Value(val as u32)
        }
    }
}

impl From<Date> for Edtf {
    fn from(date: Date) -> Self {
        Self::Date(date)
    }
}

impl From<(Date, Date)> for Edtf {
    fn from((a, b): (Date, Date)) -> Self {
        Self::Interval(a, b)
    }
}

impl FromStr for Edtf {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Edtf::parse(s)
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Date {
            year,
            month,
            day,
            certainty,
        } = *self;
        let (year, yf) = year.unpack();
        let sign = helpers::sign_str_if_neg(year);
        let year = year.abs();
        match yf.mask {
            YearMask::None => write!(f, "{}{:04}", sign, year)?,
            YearMask::OneDigit => write!(f, "{}{:03}X", sign, year / 10)?,
            YearMask::TwoDigits => write!(f, "{}{:02}XX", sign, year / 100)?,
        }
        if let Some(month) = month {
            let (m, mf) = month.unpack();
            match mf.mask {
                DMMask::None => write!(f, "-{:02}", m)?,
                DMMask::Unspecified => write!(f, "-XX")?,
            }
            if let Some(day) = day {
                let (d, df) = day.unpack();
                match df.mask {
                    DMMask::None => write!(f, "-{:02}", d)?,
                    DMMask::Unspecified => write!(f, "-XX")?,
                }
            }
        }
        if let Some(cert) = match certainty {
            Certainty::Certain => None,
            Certainty::Uncertain => Some("?"),
            Certainty::Approximate => Some("~"),
            Certainty::ApproximateUncertain => Some("%"),
        } {
            write!(f, "{}", cert)?;
        }
        Ok(())
    }
}

impl fmt::Debug for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl fmt::Display for DateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let DateComplete { year, month, day } = self.date;
        let Time { hh, mm, ss, tz } = self.time;
        write!(
            f,
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
            year, month, day, hh, mm, ss
        )?;
        match tz {
            TzOffset::Unspecified => {}
            TzOffset::Utc => write!(f, "Z")?,
            TzOffset::Hours(h) => {
                let off_h = h % 24;
                write!(f, "{:+03}", off_h)?;
            }
            TzOffset::Minutes(min) => {
                let off_m = (min.abs()) % 60;
                let off_h = (min / 60) % 24;
                write!(f, "{:+03}:{:02}", off_h, off_m)?;
            }
        }
        Ok(())
    }
}

impl fmt::Display for YYear {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Y{}", self.0)
    }
}

impl fmt::Display for Terminal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open => write!(f, ".."),
            Self::Unknown => Ok(()),
        }
    }
}

impl fmt::Display for Edtf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Date(d) => write!(f, "{}", d),
            Self::Interval(d, d2) => write!(f, "{}/{}", d, d2),
            Self::IntervalFrom(d, t) => write!(f, "{}/{}", d, t),
            Self::IntervalTo(t, d) => write!(f, "{}/{}", t, d),
            Self::YYear(s) => write!(f, "{}", s),
            Self::DateTime(dt) => write!(f, "{}", dt),
        }
    }
}

impl fmt::Debug for Edtf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Self as fmt::Display>::fmt(self, f)
    }
}
