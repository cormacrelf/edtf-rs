// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright © 2021 Corporation for Digital Scholarship

#![doc = include_str!("README.md")]

/// A set of iterators for stepping through date intervals.
pub mod iter;
pub(crate) mod packed;

mod parser;
mod validate;

#[cfg(test)]
mod test;

use core::cmp::Ordering;
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

/// See [Matcher::Interval]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MatchTerminal {
    /// An actual date in an interval
    Fixed(Precision, Certainty),
    /// `..`
    Open,
    /// Null terminal: `/2020`
    Unknown,
}

/// An enum used to conveniently match on an [Edtf].
///
/// Note that the various Interval possibilities have some impossible representations, that are
/// never produced by [Edtf::as_matcher].
///
/// ```
/// use edtf::level_1::Edtf;
/// use edtf::level_1::{
///     Matcher, MatchTerminal, Precision,
///     Certainty::*,
/// };
/// assert!(match Edtf::parse("2019/..").unwrap().as_matcher() {
///     // 2019/..
///     Matcher::Interval(MatchTerminal::Fixed(Precision::Year(y), Certain), MatchTerminal::Open) => {
///         assert_eq!(y, 2019);
///         true
///     },
///     // 18XX/20XX
///     Matcher::Interval(MatchTerminal::Fixed(Precision::Century(1800), Certain), MatchTerminal::Fixed(Precision::Century(2000), Certain)) => false,
///     // 199X
///     Matcher::Date(Precision::Decade(1990), Certain) => false,
///     // 2003%
///     Matcher::Date(Precision::Year(y), ApproximateUncertain) => false,
///     Matcher::Interval(MatchTerminal::Unknown, MatchTerminal::Unknown) |
///     Matcher::Interval(MatchTerminal::Open, MatchTerminal::Open) |
///     Matcher::Interval(MatchTerminal::Open, MatchTerminal::Unknown) |
///     Matcher::Interval(MatchTerminal::Unknown, MatchTerminal::Open) => false,
///     // /2007-09-17
///     Matcher::Interval(MatchTerminal::Unknown, MatchTerminal::Fixed(Precision::Day(_y, _m, _d), Uncertain)) => false,
///     _ => panic!("not matched"),
/// });
/// ```
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Matcher {
    /// The EDTF was a single date, no interval or time. Alternative: [Edtf::as_date]
    Date(Precision, Certainty),
    /// The EDTF was a date-time stamp. Alternative: [Edtf::as_datetime]
    DateTime(DateTime),
    /// The EDTF was a `Y12345` / `Y-12345` scientific year. [Edtf::YYear], [YYear]
    Scientific(i64),
    /// in a `Matcher` returned from [Edtf::as_matcher], one of these is guaranteed to be
    /// [MatchTerminal::Fixed]
    Interval(MatchTerminal, MatchTerminal),
    // TODO: ???
    // Interval(Precision, Certainty, Precision, Certainty),
    // IntervalFrom(Precision, Certainty, Term2),
    // IntervalTo(Term2, Precision, Certainty),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Term2 {
    Open,
    Unknown,
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

/// ## Creating an [Edtf]
///
/// As EDTF is a human-readable format not specifically designed for machines speaking to each
/// other, you may find that parsing an [Edtf] and reading its contents to be enough API.
impl Edtf {
    /// Parse a Level 1 EDTF.
    pub fn parse(input: &str) -> Result<Self, ParseError> {
        ParsedEdtf::parse_inner(input).and_then(ParsedEdtf::validate)
    }
}

/// ## Getting enum variants
impl Edtf {
    /// If self is an [Edtf::Date], return it
    pub fn as_date(&self) -> Option<Date> {
        match self {
            Self::Date(d) => Some(*d),
            _ => None,
        }
    }

    /// If self is an [Edtf::DateTime], return it
    pub fn as_datetime(&self) -> Option<DateTime> {
        match self {
            Self::DateTime(d) => Some(*d),
            _ => None,
        }
    }

    /// Shorthand for matching [Matcher::Interval]
    pub fn as_interval(&self) -> Option<(MatchTerminal, MatchTerminal)> {
        match self.as_matcher() {
            Matcher::Interval(t1, t2) => Some((t1, t2)),
            _ => None,
        }
    }

    /// Return an enum that's easier to match against for generic intervals. See [Matcher] docs for
    /// details.
    pub fn as_matcher(&self) -> Matcher {
        use self::Matcher::*;
        match self {
            Self::Date(d) => Date(d.precision(), d.certainty()),
            Self::DateTime(d) => DateTime(*d),
            Self::YYear(d) => Scientific(d.value()),
            Self::Interval(d, d2) => Interval(d.as_terminal(), d2.as_terminal()),
            Self::IntervalFrom(d, t) => Interval(d.as_terminal(), t.as_match_terminal()),
            Self::IntervalTo(t, d) => Interval(t.as_match_terminal(), d.as_terminal()),
        }
    }
}

impl Terminal {
    fn as_match_terminal(&self) -> MatchTerminal {
        match self {
            Terminal::Open => MatchTerminal::Open,
            Terminal::Unknown => MatchTerminal::Unknown,
        }
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

///
/// # Creating a [Date]
///
/// ```
/// use edtf::level_1::{Date, Certainty};
/// let _ = Date::from_ymd(2019, 07, 09).and_certainty(Certainty::Uncertain);
/// ```
impl Date {
    /// Parses a Date from a string. **Note!** This is not part of the EDTF spec. It is
    /// merely a convenience, helpful for constructing proper [Edtf] values programmatically. It
    /// does not handle any of the parts of EDTF using two dates separated by a slash, or date
    /// times.
    ///
    /// ```
    /// use edtf::level_1::Date;
    /// assert_eq!(Date::parse("2019-07-09"), Ok(Date::from_ymd(2019, 07, 09)));
    /// assert_eq!(Date::parse("2019-07"), Ok(Date::from_ymd(2019, 07, 0)));
    ///
    /// assert!(Date::parse("2019-07/2020").is_err());
    /// assert!(Date::parse("2019-00-01").is_err());
    /// assert!(Date::parse("2019-01-01T00:00:00Z").is_err());
    /// ```
    pub fn parse(input: &str) -> Result<Self, ParseError> {
        Self::parse_inner(input).and_then(UnvalidatedDate::validate)
    }

    /// Construct a date with no month or day components, e.g. `2021`. Panics if out of range.
    pub fn from_year(year: i32) -> Self {
        Self::from_ymd(year, 0, 0)
    }

    /// Construct a date with no day component, e.g. `2021-04`. Panics if out of range.
    pub fn from_ym(year: i32, month: u32) -> Self {
        Self::from_ymd(year, month, 0)
    }

    /// Creates a Date from a year, month and day. If month or day are zero, this is treated as if
    /// they have not been specified at all in an EDTF string. So it is invalid to pass `month = 0`
    /// but `day != 0`. This function **panics** on invalid input, including dates that do not
    /// exist, like non-leap-year February 29.
    ///
    /// ```
    /// use edtf::level_1::Date;
    /// assert_eq!(Date::parse("2019-07-09"), Ok(Date::from_ymd(2019, 07, 09)));
    /// assert_eq!(Date::parse("2019-07"), Ok(Date::from_ymd(2019, 07, 0)));
    /// assert_eq!(Date::parse("2019"), Ok(Date::from_ymd(2019, 0, 0)));
    /// ```
    pub fn from_ymd(year: i32, month: u32, day: u32) -> Self {
        UnvalidatedDate::from_ymd(year, month, day)
            .validate()
            .unwrap_or_else(|_| panic!("date not valid: {:04}-{:02}-{:02}", year, month, day))
    }

    /// Creates a Date from a year, month and day. If month or day are zero, this is treated as if
    /// they have not been specified at all in an EDTF string. So it is invalid to pass `month=0`
    /// but `day!=0`. This function **returns None** on invalid input, including dates that do not
    /// exist, like non-leap-year February 29.
    ///
    /// Month is `1..=12` but can also be a [Season] as an integer in range `21..=24`.
    /// ```
    /// use edtf::level_1::Date;
    /// assert_eq!(Date::parse("2019-07-09"), Ok(Date::from_ymd(2019, 07, 09)));
    /// ```
    pub fn from_ymd_opt(year: i32, month: u32, day: u32) -> Option<Self> {
        UnvalidatedDate::from_ymd(year, month, day).validate().ok()
    }

    /// Checks if a year falls inside the acceptable range of years allowed by this library.
    /// This may be a value other than `i32::MIN..=i32::MAX`. It is currently `i32::MIN >> 4 ..
    /// i32::MAX >> 4` to allow for a packed representation.
    pub fn year_in_range(year: i32) -> bool {
        PackedYear::check_range_ok(year)
    }

    /// Get the year. Dates always have one.
    pub fn year(&self) -> i32 {
        let (y, _yf) = self.year.unpack();
        y
    }

    /// Get the season. Dates don't always have one.
    pub fn season(&self) -> Option<Season> {
        let (m, _mf) = self.month?.unpack();
        Season::from_u32_opt(m.into())
    }

    /// Get the month. Dates don't always have one.
    pub fn month(&self) -> Option<Component> {
        Component::from_packed_filter(self.month?, 1..=12)
    }

    /// Get the day. Dates don't always have one.
    pub fn day(&self) -> Option<Component> {
        Some(Component::from_packed(self.day?))
    }

    /// Constructs a generic date from its complete (yyyy-mm-dd) form
    pub fn from_complete(complete: DateComplete) -> Self {
        Self::from_ymd(complete.year(), complete.month(), complete.day())
    }

    /// ```
    /// use edtf::level_1::{Date, Certainty, Precision};
    /// let date = Date::from_precision(Precision::Century(1900))
    ///     .and_certainty(Certainty::Uncertain).to_string();
    /// assert_eq!(date, "19XX?");
    /// ```
    pub fn from_precision(prec: Precision) -> Self {
        Self::from_precision_opt(prec).expect("values out of range in Date::from_precision")
    }

    /// ```
    /// use edtf::level_1::{Date, Certainty, Precision};
    /// let date = Date::from_precision_opt(Precision::DayOfYear(1908));
    /// assert!(date.is_some());
    /// assert_eq!(date, Date::parse("1908-XX-XX").ok());
    /// ```
    pub fn from_precision_opt(prec: Precision) -> Option<Self> {
        use Precision as DP;
        let (y, ym, m, mf, d, df) = match prec {
            DP::Century(x) => (
                helpers::beginning_of_century(x),
                YearMask::TwoDigits,
                0,
                None,
                0,
                None,
            ),
            DP::Decade(x) => (
                helpers::beginning_of_decade(x),
                YearMask::OneDigit,
                0,
                None,
                0,
                None,
            ),
            DP::Year(x) => (x, YearMask::None, 0, None, 0, None),
            DP::Season(x, s) => (x, YearMask::None, s as u32 as u8, None, 0, None),
            DP::Month(x, m) => (x, YearMask::None, m.try_into().ok()?, None, 0, None),
            DP::Day(x, m, d) => (
                x,
                YearMask::None,
                m.try_into().ok()?,
                None,
                d.try_into().ok()?,
                None,
            ),
            DP::MonthOfYear(x) => (x, YearMask::None, 1, Some(DMMask::Unspecified), 0, None),
            DP::DayOfMonth(x, m) => (
                x,
                YearMask::None,
                m.try_into().ok()?,
                None,
                1,
                Some(DMMask::Unspecified),
            ),
            DP::DayOfYear(x) => (
                x,
                YearMask::None,
                1,
                Some(DMMask::Unspecified),
                1,
                Some(DMMask::Unspecified),
            ),
        };
        let year = PackedYear::pack(y, ym.into())?;
        let month = PackedU8::pack(m, mf.unwrap_or_else(Default::default).into());
        let day = PackedU8::pack(d, df.unwrap_or_else(Default::default).into());
        Some(Date {
            year,
            month,
            day,
            certainty: Certainty::Certain,
        })
    }

    /// Returns a new Date with the specified [Certainty]. This certainty applies to the date as a
    /// whole.
    pub fn and_certainty(&self, certainty: Certainty) -> Self {
        Date {
            year: self.year,
            month: self.month,
            day: self.day,
            certainty,
        }
    }

    /// Private.
    fn as_terminal(&self) -> MatchTerminal {
        MatchTerminal::Fixed(self.precision(), self.certainty())
    }

    /// If the date represents a specific day, this returns a [DateComplete] for it.
    ///
    /// That type has more [chrono] integration.
    pub fn complete(&self) -> Option<DateComplete> {
        let (year, yflags) = self.year.unpack();
        let ymask = yflags.mask;
        let (month, mflags) = self.month?.unpack();
        let (day, dflags) = self.day?.unpack();
        if ymask != YearMask::None
            || mflags.mask != DMMask::None
            || dflags.mask != DMMask::None
            || month > 12
        {
            return None;
        }
        Some(DateComplete {
            year,
            // these never fail
            month: NonZeroU8::new(month)?,
            day: NonZeroU8::new(day)?,
        })
    }

    /// [Date::precision] + [Date::certainty]
    pub fn precision_certainty(&self) -> (Precision, Certainty) {
        (self.precision(), self.certainty())
    }

    /// Returns the [Certainty] of this date.
    pub fn certainty(&self) -> Certainty {
        self.certainty
    }

    /// Returns an structure more suited to use with a `match` expression.
    ///
    /// The internal representation of `Date` is packed to reduce its memory footprint, hence this
    /// API.
    pub fn precision(&self) -> Precision {
        let (y, yflags) = self.year.unpack();
        let ym = yflags.mask;
        let precision = match (self.month, self.day) {
            // Only month provided. Could be a season.
            (Some(month), None) => match month.value_u32() {
                Some(m) => {
                    if (1..=12).contains(&m) {
                        Precision::Month(y, m)
                    } else if (21..=24).contains(&m) {
                        Precision::Season(y, Season::from_u32(m as u32))
                    } else {
                        unreachable!("month was out of range")
                    }
                }
                None => Precision::MonthOfYear(y),
            },
            // Both provided, but one or both might be XX (None in the match below)
            (Some(month), Some(day)) => match (month.value_u32(), day.value_u32()) {
                (None, Some(_)) => {
                    unreachable!("date should never hold a masked month with unmasked day")
                }
                (None, None) => Precision::DayOfYear(y),
                (Some(m), None) => Precision::DayOfMonth(y, m),
                (Some(m), Some(d)) => Precision::Day(y, m, d),
            },
            (None, None) => match ym {
                YearMask::None => Precision::Year(y),
                YearMask::OneDigit => Precision::Decade(y),
                YearMask::TwoDigits => Precision::Century(y),
            },
            (None, Some(_)) => unreachable!("date should never hold a day but not a month"),
        };

        precision
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

fn cmp_edtfs(a: &Edtf, b: &Edtf) -> Ordering {
    edtf_start_date(a)
        .cmp(&edtf_start_date(b))
        .then_with(|| edtf_end_date(a).cmp(&edtf_end_date(b)))
}

fn edtf_start_date(edtf: &Edtf) -> Option<Date> {
    match edtf {
        Edtf::Date(d) => Some(*d),
        Edtf::Interval(d, _) => Some(*d),
        Edtf::IntervalFrom(d, _) => Some(*d),
        // this sorts first, which makes sense
        Edtf::IntervalTo(_, _) => None,
        Edtf::DateTime(d) => Some(Date::from_complete(d.date())),
        Edtf::YYear(y) => {
            // not super important
            let yi32: i32 = y.value().try_into().ok()?;
            Date::from_ymd_opt(yi32, 0, 0)
        }
    }
}

fn edtf_end_date(edtf: &Edtf) -> Option<Date> {
    match edtf {
        Edtf::Date(_) | Edtf::DateTime(_) | Edtf::YYear(_) => edtf_start_date(edtf),
        Edtf::Interval(_, d) => Some(*d),
        Edtf::IntervalFrom(_, _) => None,
        Edtf::IntervalTo(_, d) => Some(*d),
    }
}

#[cfg(test)]
fn cmp(a: &str, b: &str) -> Ordering {
    cmp_edtfs(&Edtf::parse(a).unwrap(), &Edtf::parse(b).unwrap())
}

#[test]
fn test_cmp_single() {
    assert_eq!(cmp("2009", "2010"), Ordering::Less);
    assert_eq!(cmp("2011", "2010"), Ordering::Greater);
    assert_eq!(cmp("2010", "2010"), Ordering::Equal);
    assert_eq!(cmp("2010-08", "2010"), Ordering::Greater);
    assert_eq!(cmp("2010-08", "2010-09"), Ordering::Less);
    assert_eq!(cmp("2010-08", "2010-08"), Ordering::Equal);
}

#[test]
fn test_cmp_single_interval() {
    assert_eq!(cmp("2009", "2010/2011"), Ordering::Less);
    assert_eq!(cmp("2011", "2009/2011"), Ordering::Greater);
    assert_eq!(cmp("2010", "2010/2010"), Ordering::Equal);
    assert_eq!(cmp("2010", "2010/2011"), Ordering::Less);
    assert_eq!(cmp("2010-08", "2010/2011"), Ordering::Greater);
}

#[test]
fn test_cmp_double_interval() {
    // we compare first on the LHS terminal, and then tie break with the RHS
    // 2009    2010    2011    2012    2013
    // |---------------|
    //         |-------|
    assert_eq!(cmp("2009/2011", "2010/2011"), Ordering::Less);
    // |---------------|
    //                 |-------|
    assert_eq!(cmp("2009/2011", "2011/2012",), Ordering::Less);
    // |---------------|
    //         |---------------|
    assert_eq!(cmp("2009/2011", "2010/2012"), Ordering::Less);
    // |---------------|
    //                         |-------|
    assert_eq!(cmp("2009/2011", "2012/2013"), Ordering::Less);
    // |---------------|
    //     |-------|
    assert_eq!(cmp("2009/2011", "2009-03/2010-07"), Ordering::Less);
}

#[test]
fn test_cmp_double_interval_open() {
    // the LHS terminal being .. means it starts at the beginning of time itself, beats everything
    // 2009    2010    2011    2012    2013
    // ----------------|
    //         |-------|
    assert_eq!(cmp("../2011", "2010/2011"), Ordering::Less);
    // ----------------|
    // ----------------|
    assert_eq!(cmp("../2011", "../2011"), Ordering::Equal);
    // ----------------|
    //         |-------|
    assert_eq!(cmp("../2011", "2010/2011"), Ordering::Less);
    // and now for the RHS being open
    //
    //         |---------------
    //         |-------|
    assert_eq!(cmp("2010/..", "2010/2011",), Ordering::Greater);
}
