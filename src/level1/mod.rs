//! # Level 1
//!
//! ## Letter-prefixed calendar year
//!
//! > 'Y' may be used at the beginning of the date string to signify that the date is a year, when
//! (and only when) the year exceeds four digits, i.e. for years later than 9999 or earlier than
//! -9999.
//!
//! - 'Y170000002' is the year 170000002
//! - 'Y-170000002' is the year -170000002
//!
//! ## Seasons
//! ## Qualification of a date (complete)
//!
//! > The characters '?', '~' and '%' are used to mean "uncertain", "approximate", and "uncertain"
//! as well as "approximate", respectively. These characters may occur only at the end of the date
//! string and apply to the entire date.
//!
//! ## Unspecified digit(s) from the right
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
//! ## Extended Interval (L1)
//!
//! - unknown start or end: `/[date]`, `[date]/`
//! - open interval, (for example 'until date' or 'from date onwards'): `../[date]`, `[date]/..`
//!
//! ## Negative calendar year
//!
//! `-1740`

use crate::level0;
use crate::ParseError;

use self::packed::{
    DayMonthCertainty, DayMonthMask, PackedDay, PackedInt, PackedMonth, PackedYear, YearMask,
};
use self::parser::UnvalidatedDate;

mod packed;
mod parser;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Date {
    year: PackedYear,
    month: Option<PackedMonth>,
    day: Option<PackedDay>,
}

fn dm_has_mask(x: (u8, DayMonthCertainty)) -> bool {
    x.1.mask != DayMonthMask::None
}

impl Date {
    fn parse(input: &str) -> Result<Self, ParseError> {
        Self::parse_inner(input).and_then(Self::validate)
    }
    fn validate(date: UnvalidatedDate) -> Result<Self, ParseError> {
        let UnvalidatedDate { year, month, day } = date;
        let level0 =
            level0::Date::from_ymd_opt(year.0, month.map_or(0, |x| x.0), day.map_or(0, |x| x.0));
        if level0.is_none() {
            return Err(ParseError::OutOfRange);
        }
        if month.is_none() && day.is_some() {
            // this can't happen if you're parsing, but people might try to construct a date like
            // this manually
            return Err(ParseError::Invalid);
        }
        if year.1.mask != YearMask::None {
            if month.is_some() || day.is_some() {
                return Err(ParseError::Invalid);
            }
        } else if let (Some(m), Some(d)) = (month, day) {
            if dm_has_mask(m) && !dm_has_mask(d) {
                return Err(ParseError::Invalid)
            }
        }
        let date = Date {
            year: PackedYear::pack(year.0, year.1).ok_or(ParseError::OutOfRange)?,
            month: month
                .map(|x| PackedMonth::pack(x.0, x.1).ok_or(ParseError::OutOfRange))
                .transpose()?,
            day: day
                .map(|x| PackedDay::pack(x.0, x.1).ok_or(ParseError::OutOfRange))
                .transpose()?,
        };
        Ok(date)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn uncertain_dates_packed() {
        use self::packed::PackedInt;
        let d = Date::parse("2019~-07~-05%").unwrap();
        println!("{:?}", d.year.unpack());
        println!("{:?}", d.month.as_ref().map(PackedInt::unpack));
        println!("{:?}", d.day.as_ref().map(PackedInt::unpack));
        println!("{:?}", std::mem::size_of_val(&d));
        // turns out don't need quite as much packing on the month and day,
        // because in [32, x], x can be up to 32 bits before causing the whole struct's size to
        // bump up beyond 64 bits. So each of month and day has 16 bits to use, each value fits in
        // 8 bits, and so each one's flags can have 8 bits to itself.
        println!("{:?}", std::mem::size_of::<UnvalidatedDate>());
    }

    #[test]
    fn xx_rightmost_only() {
        // yes
        assert!(Date::parse("201X").is_ok());
        assert!(Date::parse("20XX").is_ok());
        assert!(Date::parse("2019-XX").is_ok());
        assert!(Date::parse("2019-XX-XX").is_ok());
        assert!(Date::parse("2019-07-XX").is_ok());
        // no
        assert!(Date::parse("2019-XX-09").is_err());
        assert!(Date::parse("201X-XX").is_err());
        assert!(Date::parse("20XX-XX").is_err());
        assert!(Date::parse("20XX-07").is_err());
        assert!(Date::parse("201X-XX-09").is_err());
        assert!(Date::parse("201X-07-09").is_err());
        assert!(Date::parse("20XX-07-09").is_err());
        assert!(Date::parse("20XX-07-XX").is_err());
        assert!(Date::parse("20XX-07-0X").is_err());
        assert!(Date::parse("2019-XX-00").is_err());
        assert!(Date::parse("2019-0X-00").is_err());
        assert!(Date::parse("2019-0X-XX").is_err());
    }
}
