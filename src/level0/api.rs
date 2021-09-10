// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright © 2021 Corporation for Digital Scholarship

//! # EDTF Level 0
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

use crate::helpers;
use crate::ParseError;
use core::convert::TryInto;
use core::fmt;
use core::num::NonZeroU8;

use crate::DateTime;

/// A level 0 EDTF. See [crate::level_0] module level docs for supported syntax.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Edtf {
    /// A single date, of varying precision.
    Date(Date),
    /// Two dates separated by a slash, representing an inclusive interval
    Interval(Date, Date),
    /// A time stamp with optional time zone information.
    DateTime(DateTime),
}

impl Edtf {
    /// Parses a Level 0 EDTF.
    pub fn parse(input: &str) -> Result<Self, ParseError> {
        Self::parse_inner(input).and_then(Self::validate)
    }

    // Not really necessary
    // /// Creates an [Edtf::Date]
    // pub fn from_ymd_opt(year: i32, month: u32, day: u32) -> Option<Self> {
    //     Date::from_ymd_opt(year, month, day).map(Self::Date)
    // }
    // pub fn from_ymd(year: i32, month: u32, day: u32) -> Self {
    //     Self::from_ymd_opt(year, month, day).expect("invalid or out-of-range date")
    // }

    /// If self is an [Edtf::Date], return it.
    pub fn as_date(&self) -> Option<Date> {
        match self {
            Self::Date(d) => Some(*d),
            _ => None,
        }
    }

    /// If self is an [Edtf::Interval], return it.
    pub fn as_interval(&self) -> Option<(Date, Date)> {
        match self {
            Self::Interval(d, d2) => Some((*d, *d2)),
            _ => None,
        }
    }

    /// If self is an [Edtf::DateTime], return it.
    pub fn as_datetime(&self) -> Option<DateTime> {
        match self {
            Self::DateTime(d) => Some(*d),
            _ => None,
        }
    }
}

/// An EDTF level 0 Date. Supports only YYYY, YYYY-MM, YYYY-MM-DD.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Date {
    pub(crate) year: i32,
    pub(crate) month: Option<NonZeroU8>,
    pub(crate) day: Option<NonZeroU8>,
}

impl fmt::Debug for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl Date {
    /// Returns a year in the range 0000..=9999.
    pub fn year(self) -> i32 {
        self.year
    }
    /// 0 if absent. Guaranteed to be absent if [Self::day] is also absent.
    pub fn month(self) -> u32 {
        self.month.map_or(0, |x| x.get()) as u32
    }
    /// 0 if absent.
    pub fn day(self) -> u32 {
        self.day.map_or(0, |x| x.get()) as u32
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

    ///
    /// Returns None if year < 0 or year > 9999 or the date is invalid (e.g April 31, June 400,
    /// February 29 in a non-leap-year, month=0 but day!=0, month > 12, etc.)
    ///
    /// The year would be unsigned, but chrono and this library use u32 everywhere. This allows you
    /// to simply pass i32 years around, and get `None` if it's unsupported in `level_0`.
    pub fn from_ymd_opt(year: i32, month: u32, day: u32) -> Option<Self> {
        Date {
            year,
            month: month.try_into().ok().and_then(NonZeroU8::new),
            day: day.try_into().ok().and_then(NonZeroU8::new),
        }
        .validate()
        .ok()
    }

    /// Like [Date::from_ymd_opt], but panics if the date is unsupported in `level_0`.
    pub fn from_ymd(year: i32, month: u32, day: u32) -> Self {
        Self::from_ymd_opt(year, month, day).expect("invalid or out-of-range date")
    }
}

impl core::str::FromStr for Edtf {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Edtf::parse(s)
    }
}

impl fmt::Display for Edtf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Date(d) => write!(f, "{}", d),
            Self::Interval(d, d2) => write!(f, "{}/{}", d, d2),
            Self::DateTime(dt) => write!(f, "{}", dt),
        }
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Date { year, month, day } = *self;
        let sign = helpers::sign_str_if_neg(year);
        let year = year.abs();
        write!(f, "{}{:04}", sign, year)?;
        if let Some(month) = month {
            write!(f, "-{:02}", month)?;
            if let Some(day) = day {
                write!(f, "-{:02}", day)?;
            }
        }
        Ok(())
    }
}
