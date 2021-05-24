//! # Level 1
//!
//! ## Letter-prefixed calendar year ❌
//!
//! > 'Y' may be used at the beginning of the date string to signify that the date is a year, when
//! (and only when) the year exceeds four digits, i.e. for years later than 9999 or earlier than
//! -9999.
//!
//! - 'Y170000002' is the year 170000002
//! - 'Y-170000002' is the year -170000002
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
//! ## Negative calendar year ❌ (works programmatically)
//!
//! `-1740`

use core::str::FromStr;

use super::{
    packed::{PackedInt, PackedU8, PackedYear},
    parser::{ParsedEdtf, UnvalidatedDMEnum, UnvalidatedDate},
};
pub use crate::common::DateTime;
use crate::{common::TzOffset, level1::packed::YearFlags, ParseError};

pub use crate::level1::packed::Certainty;
pub use crate::level1::packed::YearMask;

#[cfg(feature = "chrono")]
mod chrono_interop;

/// A helper trait for getting timezone information from some value. (Especially [chrono::DateTime]
/// or [chrono::NaiveDateTime].)
///
/// Implementations for the `chrono` types are included with `feature = ["chrono"]`.
pub trait GetTimezone {
    /// Return the number of seconds difference from UTC.
    ///
    /// - `None` represents NO timezone information on the EDTF timestamp.
    /// - `Some(0)` represents a `Z` timezone, i.e. UTC/Zulu time.
    /// - `Some(3600)` represents `+01:00`
    /// - `Some(-16_200)` represents `-04:30`
    fn utc_offset_sec(&self) -> Option<i32>;
}

impl GetTimezone for DateTime {
    fn utc_offset_sec(&self) -> Option<i32> {
        match self.time.tz {
            Some(TzOffset::Offset(sec)) => Some(sec),
            Some(TzOffset::Utc) => Some(0),
            None => None,
        }
    }
}

// TODO: Hash everywhere
// TODO: wrap Certainty with one that doesn't expose the implementation detail
// TODO: convert to use u32 everywhere for chrono interoperability

/// A month or a day in [DatePrecision]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatePart {
    Masked,
    Normal(u8),
}

impl DatePart {
    pub fn value(&self) -> Option<u8> {
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

/// An EDTF date. Represents a standalone date or one end of a range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    /// `2018/2019`, `2019-12-31/2020-01-15`, etc
    Range(Date, Date),
    /// `2019/..`
    RangeOpenStart(Date),
    /// `../2019`
    RangeOpenEnd(Date),
    /// `2019/`
    RangeUnknownStart(Date),
    /// `/2019`
    RangeUnknownEnd(Date),
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
            Self::Range(d, d2) => Some((*d, *d2)),
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

impl ParsedEdtf {
    fn validate(self) -> Result<Edtf, ParseError> {
        Ok(match self {
            Self::Date(d) => Edtf::Date(d.validate()?),
            Self::Range(d, d2) => Edtf::Range(d.validate()?, d2.validate()?),
            Self::DateTime(d, t) => Edtf::DateTime(DateTime::validate(d, t)?),
            Self::RangeOpenStart(start) => Edtf::RangeOpenStart(start.validate()?),
            Self::RangeOpenEnd(end) => Edtf::RangeOpenEnd(end.validate()?),
            Self::RangeUnknownStart(start) => Edtf::RangeOpenStart(start.validate()?),
            Self::RangeUnknownEnd(end) => Edtf::RangeOpenEnd(end.validate()?),
        })
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

    pub fn from_ym(year: i32, month: u8) -> Self {
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
    pub fn from_ymd(year: i32, month: u8, day: u8) -> Self {
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
    pub fn from_ymd_opt(year: i32, month: u8, day: u8) -> Option<Self> {
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
    pub fn from_ym_masked_day(year: i32, month: u8) -> Self {
        UnvalidatedDate {
            year: (year, YearFlags::default()),
            month: Some(UnvalidatedDMEnum::Unmasked(month)),
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
            month: Some(UnvalidatedDMEnum::Unmasked(season as u32 as u8)),
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
            (Some(month), None) => match month.value() {
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
            (Some(month), Some(day)) => match (month.value(), day.value()) {
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
        Self::Range(a, b)
    }
}

impl FromStr for Edtf {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Edtf::parse(s)
    }
}
