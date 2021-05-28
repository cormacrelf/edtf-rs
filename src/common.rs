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

/// A DateTime object.
///
/// This has minimal introspection methods. Prefer to use its implementation of
/// [chrono::Datelike] and [chrono::Timelike] or simply the [DateTime::to_chrono] method to use a
/// specific [chrono::TimeZone], all available with `features = ["chrono"]`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct DateTime {
    pub(crate) date: DateComplete,
    pub(crate) time: Time,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Time {
    pub hh: u8,
    pub mm: u8,
    pub ss: u8,
    pub tz: Option<TzOffset>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) enum TzOffset {
    Utc,
    /// A number of seconds offset from UTC
    Offset(i32),
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
    /// ```
    /// use edtf::level_1::Edtf;
    /// use chrono::TimeZone;
    ///
    /// let utc = chrono::Utc;
    /// assert_eq!(
    ///     Edtf::parse("2004-02-29T01:47:00+00:00")
    ///         .unwrap()
    ///         .as_datetime()
    ///         .unwrap()
    ///         .to_chrono(&utc),
    ///     utc.ymd(2004, 02, 29).and_hms(01, 47, 00)
    /// );
    /// ```
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
        use crate::level_1::Edtf;
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

/// Has a sanity check cap of 100 digits. Because cmon.
pub fn take_min_n_digits(n: usize) -> impl FnMut(&str) -> StrResult<&str> {
    move |remain| nbc::take_while_m_n(n, 100, |x: char| x.is_ascii_digit())(remain)
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

pub fn year_n_signed(n: usize) -> impl FnMut(&str) -> StrResult<i32> {
    move |remain| {
        let (remain, sign) = minus_sign(-1i32, 1)(remain)?;
        let (remain, four) = take_n_digits(n)(remain)?;
        let (_, parsed) = nom::parse_to!(four, i32)?;
        if sign == -1 && parsed == 0 {
            return Err(nom::Err::Error(NomParseError::from_error_kind(remain, nom::error::ErrorKind::Digit)));
        }
        Ok((remain, parsed * sign))
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
        .complete()
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

pub fn minus_sign<T: Copy>(neg_one: T, one: T) -> impl FnMut(&str) -> StrResult<T> {
    move |remain| {
        let (remain, minus) = ncc::char('-')
            .map(|_| ())
            .optional()
            .parse(remain)?;
        let val = if let Some(_) = minus {
            neg_one
        } else {
            one
        };
        Ok((remain, val))
    }
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
