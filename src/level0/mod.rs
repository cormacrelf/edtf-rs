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

use crate::{common::is_valid_complete_date, ParseError};

use core::num::NonZeroU8;

mod parser;
use crate::common::{DateComplete, UnvalidatedTime, UnvalidatedTz, DateTime, Time, TzOffset};
use parser::ParsedEdtf;

pub(crate) type Year = i32;
pub(crate) type Month = Option<NonZeroU8>;
pub(crate) type Day = Option<NonZeroU8>;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Edtf {
    Date(Date),
    Range(Date, Date),
    DateTime(DateTime),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Date {
    year: Year,
    month: Month,
    day: Day,
}

impl Edtf {
    /// Parses a Level 0 EDTF.
    pub fn parse(input: &str) -> Result<Self, ParseError> {
        Self::parse_inner(input).and_then(Self::validate)
    }

    fn validate(parsed: ParsedEdtf) -> Result<Self, ParseError> {
        let edtf = match parsed {
            ParsedEdtf::Date(d) => Edtf::Date(d.validate()?),
            ParsedEdtf::Range(d, d2) => Edtf::Range(d.validate()?, d2.validate()?),
            ParsedEdtf::DateTime(d, t) => Edtf::DateTime(DateTime::validate(d, t)?),
        };
        Ok(edtf)
    }

    pub fn from_ymd_opt(year: Year, month: u8, day: u8) -> Option<Self> {
        Date::from_ymd_opt(year, month, day).map(Self::Date)
    }
    pub fn from_ymd(year: Year, month: u8, day: u8) -> Self {
        Self::from_ymd_opt(year, month, day).expect("invalid or out-of-range date")
    }
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

impl DateComplete {
    pub fn from_ymd(year: i32, month: u8, day: u8) -> Self {
        Self::from_ymd_opt(year, month, day).expect("invalid complete date")
    }
    pub fn from_ymd_opt(year: i32, month: u8, day: u8) -> Option<Self> {
        let month = NonZeroU8::new(month)?;
        let day = NonZeroU8::new(day)?;
        Self { year, month, day }.validate().ok()
    }
    pub(crate) fn validate(self) -> Result<Self, ParseError> {
        let Self { year, month, day } = self;
        let v = is_valid_complete_date(year, month.get(), day.get())?;
        Ok(v)
    }
}

impl DateTime {
    pub(crate) fn validate(date: DateComplete, time: UnvalidatedTime) -> Result<Self, ParseError> {
        let date = date.validate()?;
        let time = time.validate()?;
        Ok(DateTime { date, time })
    }
}

impl UnvalidatedTz {
    fn validate(self) -> Result<TzOffset, ParseError> {
        match self {
            Self::Utc => Ok(TzOffset::Utc),
            Self::Hours { positive, hh } => {
                let sign = if positive { 1 } else { -1 };
                if hh > 23 {
                    return Err(ParseError::OutOfRange);
                }
                Ok(TzOffset::Hours(sign * hh as i32))
            }
            Self::HoursMinutes { positive, hh, mm } => {
                // apparently iso8601-1 doesn't specify a limit on the number of hours offset you can be.
                // but we will stay sane and cap things at 23:59, because at >= 24h offset you need to
                // change the date.
                // We will however validate the minutes.
                if hh > 23 || mm > 59 {
                    return Err(ParseError::OutOfRange);
                }
                let sign = if positive { 1 } else { -1 };
                let secs = 3600 * hh as i32 + 60 * mm as i32;
                Ok(TzOffset::Seconds(sign * secs))
            }
        }
    }
}

impl UnvalidatedTime {
    fn validate(self) -> Result<Time, ParseError> {
        let Self { hh, mm, ss, tz } = self;
        let tz = tz.map(|x| x.validate()).transpose()?;
        // - ISO 8601 only allows 24 as an 'end of day' or such like when used in an interval (e.g.
        //   two /-separated timestamps.) EDTF doesn't allow intervals with time of day. So hours
        //   can't be 24.
        // - Minutes can never be 60+.
        // - Seconds can top out at 58, 59 or 60 depending on whether that day adds or subtracts a
        //   leap second. But we don't know in advance and we're not an NTP server so the best we
        //   can do is check that any ss=60 leap second occurs only on a 23:59 base.
        if hh > 23 || mm > 59 || ss > 60 {
            return Err(ParseError::OutOfRange);
        }
        if ss == 60 && !(hh == 23 && mm == 59) {
            return Err(ParseError::OutOfRange);
        }
        Ok(Time { hh, mm, ss, tz })
    }
}

impl Date {
    pub fn parse(input: &str) -> Result<Self, ParseError> {
        Self::parse_inner(input).and_then(Self::validate)
    }
    fn new_unvalidated(year: Year, month: Month, day: Day) -> Self {
        Date { year, month, day }
    }
    pub fn from_ymd_opt(year: Year, month: u8, day: u8) -> Option<Self> {
        Date {
            year,
            month: NonZeroU8::new(month),
            day: NonZeroU8::new(day),
        }
        .validate()
        .ok()
    }

    pub fn from_ymd(year: Year, month: u8, day: u8) -> Self {
        Self::from_ymd_opt(year, month, day).expect("invalid or out-of-range date")
    }

    fn validate(self) -> Result<Self, ParseError> {
        if let Some(m) = self.month.map(NonZeroU8::get) {
            if let Some(d) = self.day.map(NonZeroU8::get) {
                let _complete = is_valid_complete_date(self.year, m, d)?;
            } else {
                if m > 12 {
                    return Err(ParseError::OutOfRange);
                }
            }
        } else {
            if self.day.is_some() {
                // Both the parser and from_ymd can accept 0 for month and nonzero for day.
                return Err(ParseError::OutOfRange);
            }
            // otherwise, both Null.
        }
        Ok(self)
    }
}

#[cfg(feature = "chrono")]
fn fixed_offset_from(positive: bool, hh: u8, mm: u8) -> Option<chrono::FixedOffset> {
    let secs = 3600 * hh as i32 + 60 * mm as i32;
    if positive {
        chrono::FixedOffset::east_opt(secs)
    } else {
        chrono::FixedOffset::west_opt(secs)
    }
}

