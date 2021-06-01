//! # Level 0
//!
//! [EDTF Specification](https://www.loc.gov/standards/datetime/), February 4, 2019
//!
//! ## Date only
//!
//! | Format                            | Example      | Name              |
//! | ------                            | -------      | ---               |
//! | `[year]-[month]-[day]`            | `1985-04-12` | `[date_complete]` |
//! | `[year]-[month]`                  | `1985-04`    |                   |
//! | `[year]`                          | `1985`       |                   |
//!
//! ## Date + Time
//!
//! | Format                                     | Example                     |
//! | ------                                     | -------                     |
//! | `[date_complete]T[time]`                   | `1985-04-12T23:20:30`       |
//! | `[date_complete]T[time]Z`                  | `1985-04-12T23:20:30Z`      |
//! | `[date_complete]T[time][shift_hour]`       | `1985-04-12T23:20:30-04`    |
//! | `[date_complete]T[time][shift_hour_minute]`| `1985-04-12T23:20:30+04:30` |
//!
//! ## Time Interval
//!
//! ISO 8601-1 specifies a "time interval" and this reflects that wording, but EDTF explicitly
//! disallows time of day in intervals. So it's more of a "date interval".
//!
//! The format is `[date]/[date]`.
//!
//! | Format          | Example |
//! | ------          | ------- |
//! | `[date]/[date]` | `1964/2008`<br> `2004-06/2006-08` <br> `2004-02-01/2005-02-08` <br> `2004-02-01/2005-02` <br> etc |
//!

pub use crate::common::DateTime;
use crate::{ParseError, common::DateComplete};
use core::num::NonZeroU8;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Edtf {
    Date(Date),
    Interval(Date, Date),
    DateTime(DateTime),
}

impl Edtf {
    /// Parses a Level 0 EDTF.
    pub fn parse(input: &str) -> Result<Self, ParseError> {
        Self::parse_inner(input).and_then(Self::validate)
    }
    pub fn from_ymd_opt(year: i32, month: u8, day: u8) -> Option<Self> {
        Date::from_ymd_opt(year, month, day).map(Self::Date)
    }
    pub fn from_ymd(year: i32, month: u8, day: u8) -> Self {
        Self::from_ymd_opt(year, month, day).expect("invalid or out-of-range date")
    }
    pub fn as_date(&self) -> Option<Date> {
        match self {
            Self::Date(d) => Some(*d),
            _ => None,
        }
    }
    pub fn as_interval(&self) -> Option<(Date, Date)> {
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

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Date {
    pub(crate) year: i32,
    pub(crate) month: Option<NonZeroU8>,
    pub(crate) day: Option<NonZeroU8>,
}

impl Date {
    pub fn year(self) -> i32 {
        self.year
    }
    pub fn month(self) -> u8 {
        self.month.map_or(0, |x| x.get())
    }
    pub fn day(self) -> u8 {
        self.day.map_or(0, |x| x.get())
    }
    /// Parses a Date from a string. **Note!** This is not part of the EDTF spec. It is
    /// merely a convenience, helpful for constructing proper [Edtf] values programmatically. It
    /// does not handle any of the parts of EDTF using two dates separated by a slash, or
    /// open/unknown ranges.
    ///
    /// ```
    /// use edtf::level_0::Date;
    /// assert_eq!(Date::parse("2019-07-09"), Ok(Date::from_ymd(2019, 07, 09)));
    /// assert_eq!(Date::parse("2019-07"), Ok(Date::from_ymd(2019, 07, 0)));
    ///
    /// assert!(Date::parse("2019-07/2020").is_err());
    /// ```
    pub fn parse(input: &str) -> Result<Self, ParseError> {
        Self::parse_inner(input).and_then(Self::validate)
    }
    pub fn from_ymd_opt(year: i32, month: u8, day: u8) -> Option<Self> {
        Date {
            year,
            month: NonZeroU8::new(month),
            day: NonZeroU8::new(day),
        }
        .validate()
        .ok()
    }

    pub fn from_ymd(year: i32, month: u8, day: u8) -> Self {
        Self::from_ymd_opt(year, month, day).expect("invalid or out-of-range date")
    }
}

impl DateComplete {
    pub fn from_ymd(year: i32, month: u8, day: u8) -> Self {
        Self::from_ymd_opt(year, month, day).expect("invalid complete date")
    }
    pub fn from_ymd_opt(year: i32, month: u8, day: u8) -> Option<Self> {
        let month = NonZeroU8::new(month)?;
        let day = NonZeroU8::new(day)?;
        Self { year, month, day }.validate().ok()
    }
}

