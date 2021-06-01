// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright © 2021 Corporation for Digital Scholarship

//! # EDTF Level 1
//!
//! ## Letter-prefixed calendar year ✅
//!
//! > 'Y' may be used at the beginning of the date string to signify that the date is a year, when
//! (and only when) the year exceeds four digits, i.e. for years later than 9999 or earlier than
//! -9999.
//!
//! - 'Y170000002' is the year 170000002
//! - 'Y-170000002' is the year -170000002
//!
//! ### Notes:
//!
//! It is unclear how many other features should be supported in `Y`-years. The spec is pretty
//! quiet on this. The main reason in favour of adding a bunch of features is that `Y`-years are
//! called "date", and the "date" concept is reused all over the place. Here's some pro/con
//! analysis of adding features:
//!
//! - Can they be followed by a month and day/season?
//!   - Probably not, because the spec says '*to signify that the date is a year*'. Also who cares
//!   whether 10,000BC was a Thursday?
//! - Can they take `X/XX` unspecified digits?
//!   - In Level 2 there is already the significant digits functionality, which kinda covers this
//!   via `S1`/`S2`. So probably not.
//! - Can they have a `?~%` uncertainty attached?
//!   - If you're talking about 10,000BC, it is rare that you could actually be certain. But that
//!   only makes it logical that the additional uncertainty flags are not actually necessary.
//! - Can they be put in ranges?
//!   - Absolutely no reason why not. In fact this is probably *the* most useful feature for them.
//!   Plus, years in L2 can have significant digits, which is shorthand for making a special kind
//!   of range with an estimate. **Leaning yes.**
//! - or L2 sets?
//!   - No great reasons for/against. But those sets are designed for enumerating specific
//!   years/months/days, which is not useful for Y-years because they are typically so inaccurate.
//!
//!
//! This table lists compatibility with other implementations as of 2021-05-26.
//!

//! | Implementation                   | Rust    | [validator][v] | [PHP][php] | [Dart][dart] | [edtf.js][js] | [edtf-ruby][rb] | [python-edtf][py] |
//! | ---                              | --      | --             | --         | --           | --            | --              | --                |
//! | Last Updated                     |         | 2020-11        | 2021-05    | 2019         | 2020-11       | 2020-11         | 2018-06           |
//! | Draft version supported          | 2019-02 | 2019-02        | 2019-02    | 2019-02      | 2019-02       | 2012 ⚠️          | 2012 ⚠️            |
//! | More info                        |         | [info][vh]     |            |              |               |                 |                   |
//! | Rejects 4-digit `Y1234`          | ✅      | ✅             | ❌         | ❌           | ✅            | ✅              | ✅                |
//! | `Y17000`, `Y-17000` (base)       | ✅      | ✅             | ✅         | ✅           | ✅            | ✅              | ✅                |
//! | `Y17000-08-18`                   | ❌      | ❌             | ✅         | ✅           | ❌            | ❌              | ❌                |
//! | `Y1700X`                         | 🧐      | ❌             | ✅         | ✅           | ❌            | ❌              | ❌                |
//! | `Y17000?`                        | 🧐      | ❌             | ✅         | ✅           | ❌            | ❌              | ❌                |
//! | `Y-17000/2003`, `Y17000/..` etc. | 🧐      | ❌             | ✅         | ✅           | ❌            | ❌              | ❌                |
//! | `[Y17000..]`, etc.               | 🧐      | ❌             | ✅         | ✅           | ❌            | ❌              | ❌                |

//!
//! [v]: https://digital2.library.unt.edu/edtf/
//! [vh]: https://library.unt.edu/digital-projects-unit/metadata/fields/date/
//! [php]: https://github.com/ProfessionalWiki/EDTF
//! [dart]: https://github.com/maalexy/edtf
//! [js]: https://npmjs.com/package/edtf/
//! [rb]: https://rubygems.org/gems/edtf/
//! [py]: https://pypi.org/project/edtf/
//!
//! Test suites: [php](https://github.com/ProfessionalWiki/EDTF/blob/c0f54c0c8dff3c00f9b32ea3e773315d6a5f2c9e/tests/Functional/Level1/PrefixedYearTest.php),
//! [js]()
//! [rb](https://github.com/inukshuk/edtf-ruby/blob/7ee86d81ddb7d6503d5b282a409eb43e51f27186/spec/edtf/parser_spec.rb#L74-L80),
//! [py](https://github.com/ixc/python-edtf/blob/3bff48427b9f1452fcc030e1cc30e4e6808febc5/edtf/parser/tests.py#L101-L103) but [considers `y17e7-12-26` to be "not implemented"](https://github.com/ixc/python-edtf/blob/3bff48427b9f1452fcc030e1cc30e4e6808febc5/edtf/parser/tests.py#L195) rather than not part of the spec.
//!
//! *⚠️: The 2012 draft uses the old `y12345` syntax.*
//!
//! ## Seasons ✅
//!
//! Using Spring=21, Summer=22, Autumn=23, Winter=24.
//!
//! ## Qualification of a date (complete) ✅
//!
//! > The characters '?', '~' and '%' are used to mean "uncertain", "approximate", and "uncertain"
//! as well as "approximate", respectively. These characters may occur only at the end of the date
//! string and apply to the entire date.
//!
//! ## Unspecified digit(s) from the right ✅
//!
//! > The character 'X' may be used in place of one or more rightmost digits to indicate that the
//! value of that digit is unspecified, for the following cases:
//!
//! - `201X`, `20XX`: Year only, one or two digits: `201X`, `20XX`
//! - `2004-XX`: Year specified, *month unspecified*, month precision: `2004-XX` (different from `2004`, as
//!   it has month precision but no actual month, whereas `2004` has year precision only)
//! - `2004-07-XX`: Year and month specified, *day unspecified* in a year-month-day expression (day precision)
//! - `2004-XX-XX`: Year specified, *day and month unspecified* in a year-month-day expression  (day precision)
//!
//! ## Extended Interval (L1) ✅
//!
//! - unknown start or end: `/[date]`, `[date]/`
//! - open interval, (for example 'until date' or 'from date onwards'): `../[date]`, `[date]/..`
//!
//! ## Negative calendar year ✅
//!
//! `-1740`

use core::convert::TryInto;
use core::str::FromStr;

use crate::helpers;
use crate::{DateComplete, DateTime, ParseError, Time, TzOffset};

use super::{
    packed::{DMMask, PackedInt, PackedU8, PackedYear, YearFlags},
    parser::{ParsedEdtf, UnvalidatedDMEnum, UnvalidatedDate},
};

pub use crate::level1::packed::Certainty;
pub use crate::level1::packed::YearMask;

use core::fmt;

// TODO: Hash everywhere
// TODO: wrap Certainty with one that doesn't expose the implementation detail
// TODO: convert to use u32 everywhere for chrono interoperability

/// A month or a day in [DatePrecision]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatePart {
    Masked,
    Normal(u32),
}

impl DatePart {
    pub fn value(&self) -> Option<u32> {
        match *self {
            Self::Normal(v) => Some(v),
            Self::Masked => None,
        }
    }
}

/// A season in [DatePrecision] and constructor methods on [Date] e.g. [Date::from_year_season]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Season {
    Spring = 21,
    Summer = 22,
    Autumn = 23,
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
}

/// An enum used to conveniently match on an EDTF [Date].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatePrecision {
    Year(i32, YearMask),
    Month(i32, DatePart),
    Day(i32, DatePart, DatePart),
    Season(i32, Season),
}

/// An EDTF date. Represents a standalone date or one end of a interval.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Date {
    pub(crate) year: PackedYear,
    pub(crate) month: Option<PackedU8>,
    pub(crate) day: Option<PackedU8>,
    pub(crate) certainty: Certainty,
}

/// Fully represents EDTF Level 1. The representation is lossless.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    /// `2019/..`
    IntervalOpenFrom(Date),
    /// `../2019`
    IntervalOpenTo(Date),
    /// `2019/`
    IntervalUnknownFrom(Date),
    /// `/2019`
    IntervalUnknownTo(Date),
}

/// Represents a 5+ digit, signed year like `Y12345`, `Y-17000`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct YYear(i64);

impl YYear {
    pub fn year(&self) -> i64 {
        self.0
    }
    pub(crate) fn raw(y: i64) -> Self {
        Self(y)
    }
    /// If the value is a four-digit date (invalid for `Y`-years), returns None.
    pub fn new_opt(value: i64) -> Option<Self> {
        if helpers::inside_9999(value) {
            return None;
        }
        Some(Self(value))
    }

    /// If the value is a four-digit date, returns an [Edtf::Date] calendar date instead.
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
    pub fn as_date(&self) -> Option<Date> {
        match self {
            Self::Date(d) => Some(*d),
            _ => None,
        }
    }
    pub fn as_range(&self) -> Option<(Date, Date)> {
        match self {
            Self::Interval(d, d2) => Some((*d, *d2)),
            _ => None,
        }
    }
    pub fn as_datetime(&self) -> Option<DateTime> {
        match self {
            Self::DateTime(d) => Some(*d),
            _ => None,
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
    /// does not handle any of the parts of EDTF using two dates separated by a slash, or
    /// open/unknown ranges.
    ///
    /// ```
    /// use edtf::level_1::Date;
    /// assert_eq!(Date::parse("2019-07-09"), Ok(Date::from_ymd(2019, 07, 09)));
    /// assert_eq!(Date::parse("2019-07"), Ok(Date::from_ymd(2019, 07, 0)));
    ///
    /// assert!(Date::parse("2019-07/2020").is_err());
    /// ```
    pub fn parse(input: &str) -> Result<Self, ParseError> {
        Self::parse_inner(input).and_then(UnvalidatedDate::validate)
    }

    pub fn from_year(year: i32) -> Self {
        Self::from_ymd(year, 0, 0)
    }

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
    ///
    /// assert!(Date::parse("2019-00-01").is_err());
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

    /// Returns the [Certainty] of this date.
    pub fn certainty(&self) -> Certainty {
        self.certainty
    }

    /// Checks if a year falls inside the acceptable range of years allowed by this library.
    /// This may be a value other than `i32::MIN..=i32::MAX`. It is currently `i32::MIN >> 4 ..
    /// i32::MAX >> 4` to allow for a packed representation.
    pub fn year_in_range(year: i32) -> bool {
        PackedYear::check_range_ok(year)
    }

    /// Equivalent to `201X` or `20XX`. The year has its masked digits replaced with zeroes.
    ///
    /// Panics if year is out of range.
    pub fn from_year_masked(year: i32, mask: YearMask) -> Self {
        let year = match mask {
            YearMask::None => year,
            YearMask::OneDigit => year - year % 10,
            YearMask::TwoDigits => year - year % 100,
        };
        UnvalidatedDate {
            year: (year, YearFlags::new(Default::default(), mask)),
            ..Default::default()
        }
        .validate()
        .expect("year out of range")
    }

    /// Equivalent to e.g. `2019-XX`.
    ///
    /// Panics if year is out of range.
    pub fn from_year_masked_month(year: i32) -> Self {
        UnvalidatedDate {
            year: (year, YearFlags::default()),
            month: Some(UnvalidatedDMEnum::Masked),
            ..Default::default()
        }
        .validate()
        .expect("year out of range")
    }

    /// Equivalent to e.g. `2019-XX-XX`.
    ///
    /// Panics if year is out of range.
    pub fn from_year_masked_month_day(year: i32) -> Self {
        UnvalidatedDate {
            year: (year, YearFlags::default()),
            month: Some(UnvalidatedDMEnum::Masked),
            day: Some(UnvalidatedDMEnum::Masked),
            ..Default::default()
        }
        .validate()
        .expect("year out of range")
    }

    /// Equivalent to e.g. `2019-07-XX`
    /// Panics if year or month is out of range.
    /// Month accepts a (1-12) month or (21-24) season.
    ///
    /// Panics if year is out of range.
    pub fn from_ym_masked_day(year: i32, month: u32) -> Self {
        UnvalidatedDate {
            year: (year, YearFlags::default()),
            month: month.try_into().ok().map(UnvalidatedDMEnum::Unmasked),
            day: Some(UnvalidatedDMEnum::Masked),
            ..Default::default()
        }
        .validate()
        .expect("year or month out of range")
    }

    /// Equivalent to e.g. `2019-21`
    ///
    /// Panics if year is out of range.
    pub fn from_year_season(year: i32, season: Season) -> Self {
        UnvalidatedDate {
            year: (year, Default::default()),
            month: (season as u32)
                .try_into()
                .ok()
                .map(UnvalidatedDMEnum::Unmasked),
            ..Default::default()
        }
        .validate()
        .expect("year or month out of range")
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

    /// Returns an enum more suited to use with a `match` expression.
    /// Note that the [DatePrecision] type does not contain a date-as-a-whole Certainty field. You
    /// can easily fetch it separately with [Date::certainty].
    ///
    /// The internal representation of `Date` is packed to reduce its memory footprint, hence this
    /// API.
    pub fn as_precision(&self) -> DatePrecision {
        let (
            y,
            YearFlags {
                // year's certainty not used in level 1
                certainty: _,
                mask: ym,
            },
        ) = self.year.unpack();
        match (self.month, self.day) {
            (Some(month), None) => match month.value_u32() {
                Some(m) => {
                    if m >= 21 && m <= 24 {
                        DatePrecision::Season(y, Season::from_u32(m as u32))
                    } else if m >= 1 && m <= 12 {
                        DatePrecision::Month(y, DatePart::Normal(m))
                    } else {
                        unreachable!("month was out of range")
                    }
                }
                None => DatePrecision::Month(y, DatePart::Masked),
            },
            (Some(month), Some(day)) => match (month.value_u32(), day.value_u32()) {
                (None, Some(_)) => {
                    unreachable!("date should never hold a masked month with unmasked day")
                }
                (None, None) => DatePrecision::Day(y, DatePart::Masked, DatePart::Masked),
                (Some(m), None) => DatePrecision::Day(y, DatePart::Normal(m), DatePart::Masked),
                (Some(m), Some(d)) => {
                    DatePrecision::Day(y, DatePart::Normal(m), DatePart::Normal(d))
                }
            },
            (None, None) => DatePrecision::Year(y, ym),
            (None, Some(_)) => unreachable!("date should never hold a day but not a month"),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::level_1::*;
    use Certainty::*;

    #[test]
    fn match_precision() {
        let date = Date::parse("2019-09?").unwrap();
        assert_eq!(
            date.as_precision(),
            DatePrecision::Month(2019, DatePart::Normal(9))
        );
    }

    #[test]
    fn masking_with_uncertain() {
        assert_eq!(
            Date::parse("201X?").unwrap().as_precision(),
            DatePrecision::Year(2010, YearMask::OneDigit)
        );
        assert_eq!(
            Date::parse("2019-XX?").unwrap().as_precision(),
            DatePrecision::Month(2019, DatePart::Masked)
        );
    }

    #[test]
    fn ranges() {
        assert_eq!(
            Edtf::parse("2020/2021"),
            Ok((Date::from_year(2020), Date::from_year(2021)).into())
        );
        assert_eq!(
            Edtf::parse("2020?/2021"),
            Ok((
                Date::from_year(2020).and_certainty(Uncertain),
                Date::from_year(2021)
            )
                .into())
        );
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
                DMMask::Masked => write!(f, "-XX")?,
            }
            if let Some(day) = day {
                let (d, df) = day.unpack();
                match df.mask {
                    DMMask::None => write!(f, "-{:02}", d)?,
                    DMMask::Masked => write!(f, "-XX")?,
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

impl fmt::Display for DateComplete {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let DateComplete { year, month, day } = *self;
        write!(f, "{:04}-{:02}-{:02}", year, month, day)?;
        Ok(())
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
            None => {}
            Some(TzOffset::Utc) => write!(f, "Z")?,
            Some(TzOffset::Hours(h)) => {
                let off_h = h % 24;
                write!(f, "{:+03}", off_h)?;
            }
            Some(TzOffset::Minutes(min)) => {
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

impl fmt::Display for Edtf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Date(d) => write!(f, "{}", d),
            Self::Interval(d, d2) => write!(f, "{}/{}", d, d2),
            Self::IntervalOpenFrom(d) => write!(f, "{}/..", d),
            Self::IntervalOpenTo(d) => write!(f, "../{}", d),
            Self::IntervalUnknownFrom(d) => write!(f, "{}/", d),
            Self::IntervalUnknownTo(d) => write!(f, "/{}", d),
            Self::YYear(s) => write!(f, "{}", s),
            Self::DateTime(dt) => write!(f, "{}", dt),
        }
    }
}

#[cfg(test)]
macro_rules! test_roundtrip {
    ($x:literal) => {
        assert_eq!(Edtf::parse($x).unwrap().to_string(), $x);
    };
    ($x:literal, $y:literal) => {
        assert_eq!(Edtf::parse($x).unwrap().to_string(), $y);
    };
}

#[test]
fn test_lossless_roundtrip() {
    // dates and uncertainties
    test_roundtrip!("2019-08-17");
    test_roundtrip!("2019-08");
    test_roundtrip!("2019");
    test_roundtrip!("2019-08-17?");
    test_roundtrip!("2019-08?");
    test_roundtrip!("2019?");
    test_roundtrip!("2019-08-17~");
    test_roundtrip!("2019-08%");
    test_roundtrip!("2019%");
    // funky years
    test_roundtrip!("0043-08");
    test_roundtrip!("-0043-08");
    // timezones
    test_roundtrip!("2019-08-17T23:59:30");
    test_roundtrip!("2019-08-17T23:59:30Z");
    test_roundtrip!("2019-08-17T01:56:00+04:30");
    test_roundtrip!("2019-08-17T23:59:30+00");
    test_roundtrip!("2019-08-17T23:59:30+04");
    test_roundtrip!("2019-08-17T23:59:30-04");
    test_roundtrip!("2019-08-17T23:59:30+00:00");
    test_roundtrip!("2019-08-17T23:59:30+00:05");
    test_roundtrip!("2019-08-17T23:59:30+23:59");
    test_roundtrip!("2019-08-17T23:59:30-10:00");
    test_roundtrip!("2019-08-17T23:59:30-10:19");
}

#[test]
fn leap_second() {
    test_roundtrip!("2019-08-17T23:59:60Z");
    // no, leap seconds are always inserted at 23:59:60.
    // (unless they're removed, in which case 23:59:59 is removed.)
    assert_eq!(
        Edtf::parse("2019-08-17T22:59:60Z"),
        Err(ParseError::OutOfRange),
    );
    assert_eq!(
        Edtf::parse("2019-08-17T23:58:60Z"),
        Err(ParseError::OutOfRange),
    );
}
