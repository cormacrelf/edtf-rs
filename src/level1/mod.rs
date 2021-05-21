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

use crate::ParseError;
use crate::level0;

use self::packed::{PackedInt, PackedDay, PackedMonth, PackedYear};
use self::parser::UnvalidatedDate;

mod packed;
mod parser;

#[derive(Debug, Clone, Copy)]
pub struct Date {
    year: PackedYear,
    month: Option<PackedMonth>,
    day: Option<PackedDay>,
}

impl Date {
    fn parse(input: &str) -> Result<Self, ParseError> {
        Self::parse_inner(input).and_then(Self::validate)
    }
    fn validate(date: UnvalidatedDate) -> Result<Self, ParseError> {
        let UnvalidatedDate { year, month, day } = date;
        let level0 = level0::Date::from_ymd_opt(year.0, month.map_or(0, |x| x.0), day.map_or(0, |x| x.0));
        if level0.is_some() {
            let date = Date {
                year: PackedYear::pack(year.0, year.1).ok_or(ParseError::OutOfRange)?,
                month: month.map(|x| PackedMonth::pack(x.0, x.1).ok_or(ParseError::OutOfRange)).transpose()?,
                day: day.map(|x| PackedDay::pack(x.0, x.1).ok_or(ParseError::OutOfRange)).transpose()?,
            };
            Ok(date)
        } else {
            Err(ParseError::OutOfRange)
        }
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
    }
}
