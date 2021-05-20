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

use std::num::NonZeroU8;

pub(crate) type Year = i32;
pub(crate) type Month = Option<NonZeroU8>;
pub(crate) type Day = Option<NonZeroU8>;

use chrono::{DateTime, FixedOffset, TimeZone};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum EdtfDateLevel0<T: TimeZone> {
    Date(Date),
    Range(Date, Date),
    DateTime(DateTime<T>),
}

impl<T: TimeZone> EdtfDateLevel0<T> {
    pub fn from_ymd_opt(year: Year, month: u8, day: u8) -> Option<Self> {
        Date::from_ymd_opt(year, month, day).map(Self::Date)
    }
    pub fn from_ymd(year: Year, month: u8, day: u8) -> Self {
        Self::from_ymd_opt(year, month, day).expect("invalid or out-of-range date")
    }
    pub fn parse(input: &str, tz: &T) -> Result<Self, ParseError> {
        let fixed: EdtfDateLevel0<FixedOffset> = level0(input).finish().unwrap().1;
        Ok(fixed.with_timezone(tz))
    }
    fn with_timezone<Tz2: TimeZone>(&self, tz2: &Tz2) -> EdtfDateLevel0<Tz2> {
        match self {
            Self::DateTime(dt) => EdtfDateLevel0::DateTime(dt.with_timezone(tz2)),
            Self::Date(d) => EdtfDateLevel0::Date(*d),
            Self::Range(d, d2) => EdtfDateLevel0::Range(*d, *d2),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Date {
    year: Year,
    month: Month,
    day: Day,
}

impl Date {
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
    pub fn parse(input: &str) -> Result<Self, ParseError> {
        let date = match date(input) {
            Ok(("", d)) => d,
            Ok((_trailing, _)) => return Err(ParseError::TrailingCharacters),
            Err(e) => match e {
                nom::Err::Incomplete(_) => return Err(ParseError::Invalid),
                nom::Err::Error(_) => return Err(ParseError::Invalid),
                nom::Err::Failure(_) => return Err(ParseError::Invalid),
            },
        };
        date.validate()
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

// #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
// pub(crate) struct DateTime {
//     hours: u8,
//     minutes: u8,
//     seconds: u8,
//     // fixed point number of minutes UTC offset
//     tz_offset: Option<TimeZone>,
// }

// #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
// pub(crate) enum TimeZone {
//     Utc,
//     /// A number of minutes offset from UTC
//     Offset(NonZeroI16),
// }

#[allow(unused_imports)]
use nom::{
    branch as nb, bytes::complete as nbc, character as nch, character::complete as ncc,
    combinator as nc, error::ParseError as NomParseError, sequence as ns, Finish, IResult, ParseTo,
    Parser,
};

type StrResult<'a, T> = IResult<&'a str, T>;

fn level0(remain: &str) -> StrResult<EdtfDateLevel0<FixedOffset>> {
    use nb::alt;
    use nc::all_consuming;
    alt((
        all_consuming(|rem: &str| {
            DateTime::parse_from_rfc3339(rem)
                .map(|dt| ("", dt))
                .map_err(|_e| {
                    nom::Err::Error(NomParseError::from_error_kind(
                        "",
                        nom::error::ErrorKind::ParseTo,
                    ))
                })
        })
        .map(EdtfDateLevel0::DateTime),
        all_consuming(date).map(EdtfDateLevel0::Date),
    ))(remain)
}

fn hyphen(input: &str) -> StrResult<()> {
    let (remain, _) = ncc::char('-')(input)?;
    Ok((remain, ()))
}

fn maybe_hyphen(remain: &str) -> (&str, bool) {
    if remain.as_bytes().get(0).cloned() == Some(b'-') {
        (&remain[1..], true)
    } else {
        (remain, false)
    }
}

/// [date_complete] or one of the reduced precision variants
/// Level 0 only, no uncertainty etc.
pub(crate) fn date(remain: &str) -> StrResult<Date> {
    let (remain, year) = year4(remain)?;
    let (remain, is_hyphen) = maybe_hyphen(remain);
    if !is_hyphen {
        return Ok((remain, Date::new_unvalidated(year, None, None)));
    }
    let (remain, month) = two_digits(remain)?;
    let (remain, is_hyphen) = maybe_hyphen(remain);
    if !is_hyphen {
        return Ok((remain, Date::new_unvalidated(year, month, None)));
    }
    let (remain, day) = two_digits(remain)?;
    Ok((remain, Date::new_unvalidated(year, month, day)))
}

/// Level 0 only, YYYY-mm-dd only.
pub(crate) fn date_complete(remain: &str) -> StrResult<Date> {
    // ain't this neat
    let (remain, year) = year4(remain)?;
    let (remain, _) = hyphen(remain)?;
    let (remain, month) = two_digits(remain)?;
    let (remain, _) = hyphen(remain)?;
    let (remain, day) = two_digits(remain)?;
    Ok((remain, Date::new_unvalidated(year, month, day)))
}

/// [date_complete] + `T[time]` + :complete::is timezone info.
pub(crate) fn date_time() {}

fn take_n_digits(n: usize) -> impl FnMut(&str) -> StrResult<&str> {
    move |remain| nbc::take_while_m_n(n, n, |x: char| x.is_ascii_digit())(remain)
}

/// Level 0 year only, so simply exactly four digits 0-9. That's it.
fn year4(remain: &str) -> StrResult<Year> {
    let (remain, four) = take_n_digits(4)(remain)?;
    let (_, parsed) = nom::parse_to!(four, Year)?;
    Ok((remain, parsed))
}

/// Level 0 month or day. Two digits, and the range is not checked here, except that 00 is
/// rejected.
fn two_digits(remain: &str) -> StrResult<Option<NonZeroU8>> {
    let (remain, two) = take_n_digits(2)(remain)?;
    // NonZeroU8's FromStr implementation rejects 00.
    let (_, parsed) = nom::parse_to!(two, NonZeroU8)?;
    Ok((remain, Some(parsed)))
}

/// no T, HH:MM:SS
fn time(remain: &str) -> StrResult<()> {
    // let (remain, hours) = two_digits(remain)?;
    // let (remain, _) = ncc::char(':')(remain)?;
    // let (remain, minutes) = two_digits(remain)?;
    // let (remain, _) = ncc::char(':')(remain)?;
    // let (remain, seconds) = two_digits(remain)?;
    Ok((remain, ()))
}

/// `-04`
fn shift_hour() {}
/// `-04:30`
fn shift_hour_minute() {}

#[cfg(test)]
mod test {
    use super::Date;
    use nom::Finish;

    #[test]
    fn date() {
        assert_eq!(Date::parse("1985-04-12"), Ok(Date::from_ymd(1985, 4, 12)));
        assert_eq!(Date::parse("1985-04"), Ok(Date::from_ymd(1985, 4, 0)));
        assert_eq!(Date::parse("1985"), Ok(Date::from_ymd(1985, 0, 0)));
    }

    #[test]
    fn date_remain() {
        assert_eq!(
            super::date("1985-04-12T12345").finish(),
            Ok(("T12345", Date::from_ymd(1985, 4, 12)))
        );
        assert_eq!(
            super::date("1985-0489898989").finish(),
            Ok(("89898989", Date::from_ymd(1985, 4, 0)))
        );
        assert_eq!(
            super::date("1985000").finish(),
            Ok(("000", Date::from_ymd(1985, 0, 0)))
        );
    }

    #[test]
    fn date_invalid() {
        use crate::ParseError;
        assert_eq!(Date::parse("1985000"), Err(ParseError::TrailingCharacters));
        assert_eq!(Date::parse("2003-02-29"), Err(ParseError::OutOfRange));
        assert_eq!(Date::parse("2003-02-28"), Ok(Date::from_ymd(2003, 2, 28)),);
        assert_eq!(Date::parse("2004-02-29"), Ok(Date::from_ymd(2004, 2, 29)),);
    }

    use super::EdtfDateLevel0;
    use chrono::{FixedOffset, NaiveDate, TimeZone};

    #[test]
    fn date_time() {
        let tz = FixedOffset::east(0);
        assert_eq!(
            EdtfDateLevel0::parse("2004-02-29T01:47:00+00:00", &tz),
            Ok(EdtfDateLevel0::DateTime(
                tz.from_utc_datetime(&NaiveDate::from_ymd(2004, 02, 29).and_hms(01, 47, 00))
            ))
        );
        // ok, turns out we can't use RFC3339, because EDTF supports dropping the Z/tz from the
        // time.
        assert_eq!(
            EdtfDateLevel0::parse("2004-02-29T01:47:00", &tz),
            Ok(EdtfDateLevel0::DateTime(
                tz.from_utc_datetime(&NaiveDate::from_ymd(2004, 02, 29).and_hms(01, 47, 00))
            ))
        );
    }
}
