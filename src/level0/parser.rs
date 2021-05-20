use crate::helpers::ParserExt;
use crate::ParseError;
use core::num::NonZeroU8;
use core::str::FromStr;

#[allow(unused_imports)]
use nom::{
    branch as nb, bytes::complete as nbc, character as nch, character::complete as ncc,
    combinator as nc, error::ParseError as NomParseError, sequence as ns, Finish, IResult, ParseTo,
    Parser,
};

use super::{Date, DateComplete, Edtf};

impl Edtf {
    pub(crate) fn parse_inner(input: &str) -> Result<ParsedEdtf, ParseError> {
        level0
            .complete()
            .parse(input)
            // parser already fails on trailing chars
            .map(|(_, a)| a)
            .map_err(|_| ParseError::Invalid)
    }
}

impl Date {
    pub(crate) fn parse_inner(input: &str) -> Result<Self, ParseError> {
        date
            .complete()
            .parse(input)
            .map(|(_, a)| a)
            .map_err(|_| ParseError::Invalid)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum ParsedEdtf {
    Date(Date),
    Range(Date, Date),
    DateTime(DateComplete, UnvalidatedTime),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) struct UnvalidatedTime {
    pub hh: u8,
    pub mm: u8,
    pub ss: u8,
    pub tz: Option<UnvalidatedTz>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) enum UnvalidatedTz {
    Utc,
    Offset { positive: bool, hh: u8, mm: u8 },
}

type StrResult<'a, T> = IResult<&'a str, T>;

fn level0(remain: &str) -> StrResult<ParsedEdtf> {
    let dt = date_time.map(|(d, t)| ParsedEdtf::DateTime(d, t));
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
fn year4(remain: &str) -> StrResult<i32> {
    let (remain, four) = take_n_digits(4)(remain)?;
    let (_, parsed) = nom::parse_to!(four, i32)?;
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
    use super::*;

    #[test]
    fn date() {
        assert_eq!(Date::parse("1985-04-12"), Ok(Date::from_ymd(1985, 4, 12)));
        assert_eq!(Date::parse("1985-04"), Ok(Date::from_ymd(1985, 4, 0)));
        assert_eq!(Date::parse("1985"), Ok(Date::from_ymd(1985, 0, 0)));
    }

    #[test]
    fn date_invalid() {
        use crate::ParseError;
        assert_eq!(Date::parse("1985000"), Err(ParseError::Invalid));
        assert_eq!(Date::parse("2003-02-29"), Err(ParseError::OutOfRange));
        assert_eq!(Date::parse("2003-02-28"), Ok(Date::from_ymd(2003, 2, 28)),);
        assert_eq!(Date::parse("2004-02-29"), Ok(Date::from_ymd(2004, 2, 29)),);
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
}
