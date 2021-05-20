#![allow(dead_code)]

//! # Level 0
//!
//! [EDTF Specification](https://www.loc.gov/standards/datetime/), February 4, 2019
//!
//! Level 0 is also described in ISO 8601-1, but that costs a few hundred dollars to read.
//!
//! ## Date only
//!
//! API: [date]
//!
//! | Format                            | Example      | API                                 |
//! | ------                            | -------      | ---                                 |
//! | `[year]-[month]-[day]`            | `1985-04-12` | [date_complete]                     |
//! | `[year]-[month]`                  | `1985-04`    |                                     |
//! | `[year]`                          | `1985`       |                                     |
//!
//! ## Date + Time [date_time]
//!
//! API: [date_time]
//!
//! | Format                                     | Example                     |
//! | ------                                     | -------                     |
//! | `[date_complete]T[time]`                   | `1985-04-12T23:20:30`       |
//! | `[date_complete]T[time]Z`                  | `1985-04-12T23:20:30Z`      |
//! | `[date_complete]T[time][shift_hour]`       | `1985-04-12T23:20:30-04`    |
//! | `[date_complete]T[time][shift_hour_minute]`| `1985-04-12T23:20:30+04:30` |
//!
//! ## Time Interval (should probably be called 'Date interval'!)
//!
//! The format is `[date]/[date]`.
//!
//! | Format          | Example |
//! | ------          | ------- |
//! | `[date]/[date]` | `1964/2008`<br> `2004-06/2006-08` <br> `2004-02-01/2005-02-08` <br> `2004-02-01/2005-02` <br> etc |
//!

use crate::helpers::is_leap_year;
use crate::ParseError;

use core::num::NonZeroU8;

mod parser;
use parser::{UnvalidatedTime, UnvalidatedTz, ParsedEdtf};

pub(crate) type Year = i32;
pub(crate) type Month = Option<NonZeroU8>;
pub(crate) type Day = Option<NonZeroU8>;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) enum Edtf {
    Date(Date),
    Range(Date, Date),
    DateTime(DateTime),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Date {
    year: Year,
    month: Month,
    day: Day,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) struct DateComplete {
    year: Year,
    month: NonZeroU8,
    day: NonZeroU8,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct DateTime {
    date: DateComplete,
    time: Time,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct Time {
    hh: u8,
    mm: u8,
    ss: u8,
    tz: Option<TzOffset>,
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
    fn validate(self) -> Result<Self, ParseError> {
        let Self { year, month, day } = self;
        let date = Date::new_unvalidated(year, Some(month), Some(day));
        let v = date.validate()?;
        Ok(Self {
            year: v.year,
            month: v.month.unwrap(),
            day: v.day.unwrap(),
        })
    }
}

impl DateTime {
    fn validate(date: DateComplete, time: UnvalidatedTime) -> Result<Self, ParseError> {
        let date = date.validate()?;
        let time = time.validate()?;
        Ok(DateTime { date, time })
    }
}

impl UnvalidatedTz {
    fn validate(self) -> Result<TzOffset, ParseError> {
        match self {
            Self::Utc => Ok(TzOffset::Utc),
            Self::Offset { positive, hh, mm } => {
                // 24:00 offsets are apparently fine, according to rfc3339
                if hh > 24 || mm >= 60 {
                    return Err(ParseError::OutOfRange);
                }
                let sign = if positive { 1 } else { -1 };
                let secs = 3600 * hh as i32 + 60 * mm as i32;
                Ok(TzOffset::Offset(sign * secs))
            }
        }
    }
}

impl UnvalidatedTime {
    fn validate(self) -> Result<Time, ParseError> {
        let Self { hh, mm, ss, tz } = self;
        let tz = tz.map(|x| x.validate()).transpose()?;
        if hh > 24 || mm >= 60 || ss >= 60 {
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

    fn validate(mut self) -> Result<Self, ParseError> {
        self.month = self.month.and_then(nullify_invalid_month_level0);
        self.day = self.day.and_then(nullify_invalid_day);
        const MONTH_DAYCOUNT: [u8; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        const MONTH_DAYCOUNT_LEAP: [u8; 12] = [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        if let Some(m) = self.month.map(NonZeroU8::get) {
            if let Some(d) = self.day.map(NonZeroU8::get) {
                let max = if is_leap_year(self.year) {
                    MONTH_DAYCOUNT_LEAP[m as usize - 1]
                } else {
                    MONTH_DAYCOUNT[m as usize - 1]
                };
                if d > max {
                    return Err(ParseError::OutOfRange);
                }
            }
        } else {
            // after nullify_invaliding, either could suddenly be zero.
            // make sure if month is None, so is Day.
            self.day = None;
        }
        Ok(self)
    }
}

fn nullify_invalid_month_level0(m: NonZeroU8) -> Option<NonZeroU8> {
    let m = m.get();
    let val = if m > 12 { 0 } else { m };
    NonZeroU8::new(val)
}

fn nullify_invalid_day(m: NonZeroU8) -> Option<NonZeroU8> {
    let m = m.get();
    let val = if m > 31 { 0 } else { m };
    NonZeroU8::new(val)
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) enum TzOffset {
    Utc,
    /// A number of seconds offset from UTC
    Offset(i32),
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

#[cfg(feature = "chrono")]
fn chrono_tz_datetime<Tz: chrono::TimeZone>(
    tz: &Tz,
    date: &DateComplete,
    time: &Time,
) -> chrono::DateTime<Tz> {
    tz.ymd(date.year, date.month.get() as u32, date.day.get() as u32)
        .and_hms(time.hh as u32, time.mm as u32, time.ss as u32)
}

#[cfg(feature = "chrono")]
impl DateTime {
    pub fn to_chrono<Tz>(&self, tz: &Tz) -> chrono::DateTime<Tz>
    where
        Tz: chrono::TimeZone,
    {
        let DateTime { date, time } = self;
        match time.tz {
            None => chrono_tz_datetime(tz, date, time),
            Some(TzOffset::Utc) => {
                let utc = chrono_tz_datetime(&chrono::Utc, date, time);
                utc.with_timezone(tz)
            }
            Some(TzOffset::Offset(signed_seconds)) => {
                let fixed_zone = chrono::FixedOffset::east_opt(signed_seconds)
                    .expect("time zone offset out of bounds");
                let fixed_dt = chrono_tz_datetime(&fixed_zone, date, time);
                fixed_dt.with_timezone(tz)
            }
        }
    }
}

#[cfg(test)]
mod test {

    #[cfg(feature = "chrono")]
    #[test]
    fn to_chrono() {
        use super::Edtf;
        use chrono::TimeZone;
        let utc = chrono::Utc;
        assert_eq!(
            Edtf::parse("2004-02-29T01:47:00+00:00")
                .unwrap()
                .as_datetime()
                .unwrap()
                .to_chrono(&utc),
            utc.ymd(2004, 02, 29).and_hms(01, 47, 00)
        );
        assert_eq!(
            Edtf::parse("2004-02-29T01:47:00")
                .unwrap()
                .as_datetime()
                .unwrap()
                .to_chrono(&utc),
            utc.ymd(2004, 02, 29).and_hms(01, 47, 00)
        );
    }

}
