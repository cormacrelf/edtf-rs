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
use crate::helpers::ParserExt;
use crate::ParseError;

use core::num::NonZeroU8;
use core::str::FromStr;

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

impl DateTime {
    fn validate(date: DateComplete, time: UnvalidatedTime) -> Result<Self, ParseError> {
        let date = date.validate()?;
        let time = time.validate()?;
        Ok(DateTime { date, time })
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
    fn to_chrono<Tz>(&self, tz: &Tz) -> chrono::DateTime<Tz>
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

impl Edtf {
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

    pub fn parse(input: &str) -> Result<Self, ParseError> {
        let parsed: ParsedEdtf = level0
            .complete()
            .parse(input)
            .finish()
            // parser already fails on trailing chars
            .map(|(_, a)| a)
            .map_err(|_| ParseError::Invalid)?;
        let edtf = match parsed {
            ParsedEdtf::Date(d) => Edtf::Date(d.validate()?),
            ParsedEdtf::Range(d, d2) => Edtf::Range(d.validate()?, d2.validate()?),
            ParsedEdtf::DateTime(d, t) => Edtf::DateTime(DateTime::validate(d, t)?),
        };
        Ok(edtf)
    }
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

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) struct UnvalidatedTime {
    hh: u8,
    mm: u8,
    ss: u8,
    tz: Option<UnvalidatedTz>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) enum UnvalidatedTz {
    Utc,
    Offset { positive: bool, hh: u8, mm: u8 },
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) enum TzOffset {
    Utc,
    /// A number of seconds offset from UTC
    Offset(i32),
}

#[allow(unused_imports)]
use nom::{
    branch as nb, bytes::complete as nbc, character as nch, character::complete as ncc,
    combinator as nc, error::ParseError as NomParseError, sequence as ns, Finish, IResult, ParseTo,
    Parser,
};

type StrResult<'a, T> = IResult<&'a str, T>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ParsedEdtf {
    Date(Date),
    Range(Date, Date),
    DateTime(DateComplete, UnvalidatedTime),
}

fn level0(remain: &str) -> StrResult<ParsedEdtf> {
    let dt = date_time
        .map(|(d, t)| ParsedEdtf::DateTime(d, t));
    let range = date_range.map(|(a, b)| ParsedEdtf::Range(a, b));
    let single = date.map(ParsedEdtf::Date);

    dt.or(range).or(single).parse(remain)
}

fn date_range(remain: &str) -> StrResult<(Date, Date)> {
    date.and(ncc::char('/'))
        .map(|(a, _)| a)
        .and(date)
        .parse(remain)
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
    let (remain, month) = two_digits::<NonZeroU8>(remain)?;
    let (remain, is_hyphen) = maybe_hyphen(remain);
    if !is_hyphen {
        return Ok((remain, Date::new_unvalidated(year, Some(month), None)));
    }
    let (remain, day) = two_digits::<NonZeroU8>(remain)?;
    Ok((remain, Date::new_unvalidated(year, Some(month), Some(day))))
}

/// Level 0 only, YYYY-mm-dd only.
pub(crate) fn date_complete(remain: &str) -> StrResult<DateComplete> {
    // ain't this neat
    let (remain, year) = year4(remain)?;
    let (remain, _) = hyphen(remain)?;
    let (remain, month) = two_digits(remain)?;
    let (remain, _) = hyphen(remain)?;
    let (remain, day) = two_digits(remain)?;
    Ok((remain, DateComplete { year, month, day }))
}

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
fn two_digits<T: FromStr>(remain: &str) -> StrResult<T> {
    let (remain, two) = take_n_digits(2)(remain)?;
    // NonZeroU8's FromStr implementation rejects 00.
    let (_, parsed) = nom::parse_to!(two, T)?;
    Ok((remain, parsed))
}

// /// no T, HH:MM:SS and an optional offset
// fn time(remain: &str) -> StrResult<(u8, u8, u8, Option<TzOffset>)> {
//     let (remain, hours) = two_digits(remain)?;
//     let (remain, _) = ncc::char(':')(remain)?;
//     let (remain, minutes) = two_digits(remain)?;
//     let (remain, _) = ncc::char(':')(remain)?;
//     let (remain, seconds) = two_digits(remain)?;
//     Ok((remain, (hours, minutes, seconds, offset)))
// }

/// [date_complete] + `T[time]` + :complete::is timezone info.
fn date_time(remain: &str) -> StrResult<(DateComplete, UnvalidatedTime)> {
    date_complete
        .and_ignore(ncc::char('T'))
        .and(time)
        .parse(remain)
}

/// no T, HH:MM:SS and an optional offset
fn time(remain: &str) -> StrResult<UnvalidatedTime> {
    two_digits
        .and_ignore(ncc::char(':'))
        .and(two_digits::<u8>)
        .and_ignore(ncc::char(':'))
        .and(two_digits::<u8>)
        .and(tz_offset.optional())
        .map(|(((hh, mm), ss), tz)| UnvalidatedTime { hh, mm, ss, tz })
        .parse(remain)
}

fn tz_offset(remain: &str) -> StrResult<UnvalidatedTz> {
    let utc = ncc::char('Z').map(|_| UnvalidatedTz::Utc);
    utc.or(shift_hour_minute).or(shift_hour).parse(remain)
}

fn sign(remain: &str) -> StrResult<bool> {
    ncc::char('+')
        .or(ncc::char('-'))
        .map(|x| x == '+')
        .parse(remain)
}

/// `-04`, `+04`
fn shift_hour(remain: &str) -> StrResult<UnvalidatedTz> {
    sign.and(two_digits::<u8>)
        .map(|(positive, hh)| UnvalidatedTz::Offset {
            positive,
            hh,
            mm: 0,
        })
        .parse(remain)
}
/// `-04:30`
fn shift_hour_minute(remain: &str) -> StrResult<UnvalidatedTz> {
    sign.and(two_digits::<u8>)
        .and_ignore(ncc::char(':'))
        .and(two_digits::<u8>)
        .map(|((positive, hh), mm)| UnvalidatedTz::Offset { positive, hh, mm })
        .parse(remain)
}

#[cfg(test)]
mod test {
    use std::num::NonZeroU8;

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

    use super::{DateComplete, ParsedEdtf, UnvalidatedTime, UnvalidatedTz};

    #[test]
    fn parse_level0() {
        assert_eq!(
            super::level0("2004-02-29T01:47:05"),
            Ok((
                "",
                ParsedEdtf::DateTime(
                    DateComplete {
                        year: 2004,
                        month: NonZeroU8::new(02).unwrap(),
                        day: NonZeroU8::new(29).unwrap(),
                    },
                    UnvalidatedTime {
                        hh: 01,
                        mm: 47,
                        ss: 05,
                        tz: None
                    },
                )
            ))
        );

        assert_eq!(
            super::level0("2004-02-29T01:47:00Z"),
            Ok((
                "",
                ParsedEdtf::DateTime(
                    DateComplete {
                        year: 2004,
                        month: NonZeroU8::new(02).unwrap(),
                        day: NonZeroU8::new(29).unwrap(),
                    },
                    UnvalidatedTime {
                        hh: 01,
                        mm: 47,
                        ss: 00,
                        tz: Some(UnvalidatedTz::Utc)
                    },
                )
            ))
        );

        assert_eq!(
            super::level0("2004-02-29T01:47:00+00:00"),
            Ok((
                "",
                ParsedEdtf::DateTime(
                    DateComplete {
                        year: 2004,
                        month: NonZeroU8::new(02).unwrap(),
                        day: NonZeroU8::new(29).unwrap(),
                    },
                    UnvalidatedTime {
                        hh: 01,
                        mm: 47,
                        ss: 00,
                        tz: Some(UnvalidatedTz::Offset {
                            positive: true,
                            hh: 00,
                            mm: 00
                        })
                    },
                )
            ))
        );

        assert_eq!(
            super::level0("2004-02-29T01:47:00-04:30"),
            Ok((
                "",
                ParsedEdtf::DateTime(
                    DateComplete {
                        year: 2004,
                        month: NonZeroU8::new(02).unwrap(),
                        day: NonZeroU8::new(29).unwrap(),
                    },
                    UnvalidatedTime {
                        hh: 01,
                        mm: 47,
                        ss: 00,
                        tz: Some(UnvalidatedTz::Offset {
                            positive: false,
                            hh: 04,
                            mm: 30
                        })
                    },
                )
            ))
        );

        assert_eq!(
            super::level0("2004-02-29/2009-07-16"),
            Ok((
                "",
                ParsedEdtf::Range(Date::from_ymd(2004, 02, 29), Date::from_ymd(2009, 07, 16),)
            ))
        );

        assert_eq!(
            super::level0("2004-02-29/2009-07"),
            Ok((
                "",
                ParsedEdtf::Range(Date::from_ymd(2004, 02, 29), Date::from_ymd(2009, 07, 0),)
            ))
        );

        assert_eq!(
            super::level0("2004/2009-07"),
            Ok((
                "",
                ParsedEdtf::Range(Date::from_ymd(2004, 00, 00), Date::from_ymd(2009, 07, 00),)
            ))
        );
    }

    #[cfg(feature = "chrono")]
    #[test]
    fn date_time() {
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
