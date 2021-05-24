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

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct DateComplete {
    pub(crate) year: i32,
    pub(crate) month: NonZeroU8,
    pub(crate) day: NonZeroU8,
}

/// Proleptic Gregorian leap year function.
/// From RFC3339 Appendix C.
pub(crate) fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

#[test]
fn leap_year() {
    // yes
    assert!(is_leap_year(2004));
    assert!(is_leap_year(-400));
    assert!(is_leap_year(-204));
    assert!(is_leap_year(0));
    // no
    assert!(!is_leap_year(1));
    assert!(!is_leap_year(100));
    assert!(!is_leap_year(1900));
    assert!(!is_leap_year(1901));
    assert!(!is_leap_year(2005));
    assert!(!is_leap_year(-1));
    assert!(!is_leap_year(-100));
    assert!(!is_leap_year(-200));
    assert!(!is_leap_year(-300));
    assert!(!is_leap_year(-309));
}

pub(crate) fn is_valid_complete_date(
    year: i32,
    month: u8,
    day: u8,
) -> Result<DateComplete, ParseError> {
    const MONTH_DAYCOUNT: [u8; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    const MONTH_DAYCOUNT_LEAP: [u8; 12] = [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let month = NonZeroU8::new(month).ok_or(ParseError::OutOfRange)?;
    let day = NonZeroU8::new(day).ok_or(ParseError::OutOfRange)?;
    let m = month.get();
    let d = day.get();
    if m > 12 || m < 1 || d < 1 || d > 31 {
        return Err(ParseError::OutOfRange);
    }
    let max = if is_leap_year(year) {
        MONTH_DAYCOUNT_LEAP[m as usize - 1]
    } else {
        MONTH_DAYCOUNT[m as usize - 1]
    };
    if d > max {
        return Err(ParseError::OutOfRange);
    }
    Ok(DateComplete { year, month, day })
}

pub type StrResult<'a, T> = IResult<&'a str, T>;

pub fn hyphen(input: &str) -> StrResult<()> {
    let (remain, _) = ncc::char('-')(input)?;
    Ok((remain, ()))
}

pub fn maybe_hyphen(remain: &str) -> (&str, bool) {
    if remain.as_bytes().get(0).cloned() == Some(b'-') {
        (&remain[1..], true)
    } else {
        (remain, false)
    }
}

pub fn take_n_digits(n: usize) -> impl FnMut(&str) -> StrResult<&str> {
    move |remain| nbc::take_while_m_n(n, n, |x: char| x.is_ascii_digit())(remain)
}

/// Level 0 month or day. Two digits, and the range is not checked here except for the range of T.
pub fn two_digits<T: FromStr>(remain: &str) -> StrResult<T> {
    let (remain, two) = take_n_digits(2)(remain)?;
    // NonZeroU8's FromStr implementation rejects 00.
    let (_, parsed) = nom::parse_to!(two, T)?;
    Ok((remain, parsed))
}

/// Level 0 month or day. Two digits, and the range is not checked here.
pub fn two_digits_zero_none(remain: &str) -> StrResult<Option<NonZeroU8>> {
    let (remain, two) = take_n_digits(2).parse(remain)?;
    // NonZeroU8's FromStr implementation rejects 00.
    let (_, parsed) = nom::parse_to!(two, u8)?;
    Ok((remain, NonZeroU8::new(parsed)))
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct UnvalidatedTime {
    pub hh: u8,
    pub mm: u8,
    pub ss: u8,
    pub tz: Option<UnvalidatedTz>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum UnvalidatedTz {
    Utc,
    Offset { positive: bool, hh: u8, mm: u8 },
}

pub fn year_n(n: usize) -> impl FnMut(&str) -> StrResult<i32> {
    move |remain| {
        let (remain, four) = take_n_digits(n)(remain)?;
        let (_, parsed) = nom::parse_to!(four, i32)?;
        Ok((remain, parsed))
    }
}

/// Level 0 only, YYYY-mm-dd only.
pub fn date_complete(remain: &str) -> StrResult<DateComplete> {
    year_n(4)
        .and_ignore(hyphen)
        .and(two_digits)
        .and_ignore(hyphen)
        .and(two_digits)
        .map(|((year, month), day)| DateComplete { year, month, day })
        .parse(remain)
}

/// [date_complete] + `T[time]` + :complete::is timezone info.
pub fn date_time(remain: &str) -> StrResult<(DateComplete, UnvalidatedTime)> {
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

pub fn sign(remain: &str) -> StrResult<bool> {
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
