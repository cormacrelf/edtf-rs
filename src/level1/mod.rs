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
use ParseError::*;
use crate::common::is_valid_complete_date;

use self::packed::{DMEnum, PackedInt, PackedYear, YearMask};
use self::parser::UnvalidatedDate;

mod packed;
mod parser;
pub mod api;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Date {
    pub(crate) year: PackedYear,
    pub(crate) month: Option<DMEnum>,
    pub(crate) day: Option<DMEnum>,
}

fn value_of(opt_dmenum: Option<DMEnum>) -> Option<u8> {
    opt_dmenum.as_ref().and_then(DMEnum::value)
}

fn validate(date: UnvalidatedDate) -> Result<Date, ParseError> {
    let UnvalidatedDate { year, month, day } = date;
    let month = month.as_ref().map(|m| m.validate()).transpose()?;
    let day = day.as_ref().map(|m| m.validate()).transpose()?;

    eprintln!("\ncheck_structure: {:?}", date);
    match (month, day) {
        // this can't happen if you're parsing, but people might try to construct a date like this
        // manually with zero values
        (None, Some(_)) => return Err(OutOfRange),
        _ => {}
    }

    // mask rules
    // this is funny in 2021
    eprintln!("    check_masks: {:?}", date);
    let year_mask = year.1.mask != YearMask::None;
    let month_mask = month.as_ref().map(DMEnum::is_masked);
    let day_mask = day.as_ref().map(DMEnum::is_masked);
    match (year_mask, month_mask, day_mask) {
        // no masks is fine
        (false, None, None) => {}
        (false, Some(false), None) => {}
        (false, Some(false), Some(false)) => {}
        // the four valid cases in the spec
        (true, None, None) => {}
        (false, Some(true), None) => {}
        (false, Some(true), Some(true)) => {}
        (false, Some(false), Some(true)) => {}
        _ => return Err(Invalid),
    }

    eprintln!("   check_values: {:?}", date);
    let year_val = year.0;
    let month_val = month.as_ref().and_then(DMEnum::value);
    let day_val = day.as_ref().and_then(DMEnum::value);
    match (month_val, day_val) {
        // not a month (i.e. a season), but day provided
        (Some(m), Some(_)) if m > 12 => return Err(Invalid),
        (Some(m), None) if (m >= 1 && m <= 12) || (m >= 21 && m <= 24) => {}
        (Some(m), Some(d)) if m >= 1 && m <= 12 => {
            let _complete = is_valid_complete_date(year_val, m, d)?;
        }
        (None, None) => {}
        _ => return Err(OutOfRange),
    }
    let date = Date {
        year: PackedYear::pack(date.year.0, date.year.1).ok_or(ParseError::OutOfRange)?,
        month: month,
        day: day,
    };
    Ok(date)
}

impl Date {
    fn validate(date: UnvalidatedDate) -> Result<Self, ParseError> {
        validate(date)
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
        println!("{:?}", d.month);
        println!("{:?}", d.day);
        println!("{:?}", std::mem::size_of_val(&d));
        println!("{:?}", std::mem::size_of::<UnvalidatedDate>());
        assert!(std::mem::size_of_val(&d) <= 8);
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

    #[test]
    fn xx_with_uncertainty() {
        // yes
        assert!(Date::parse("201X?").is_ok());
        assert!(Date::parse("20XX~").is_ok());
        assert!(Date::parse("20XX%").is_ok());
        assert!(Date::parse("2019-XX?").is_ok());
        assert!(Date::parse("2019-XX~").is_ok());
        assert!(Date::parse("2019-XX%").is_ok());
        assert!(Date::parse("2019-XX-XX?").is_ok());
        assert!(Date::parse("2019-XX-XX~").is_ok());
        assert!(Date::parse("2019-XX-XX%").is_ok());
        assert!(Date::parse("2019~-XX?-XX~").is_ok());
        assert!(Date::parse("2019-XX?-XX%").is_ok());
        assert!(Date::parse("2019?-XX-XX%").is_ok());
        assert!(Date::parse("2019-07-XX?").is_ok());
        assert!(Date::parse("2019-07-XX~").is_ok());
        assert!(Date::parse("2019-07-XX%").is_ok());
        assert!(Date::parse("2019~-07-XX?").is_ok());
        assert!(Date::parse("2019~-07~-XX~").is_ok());
        assert!(Date::parse("2019-07~-XX%").is_ok());
    }

    #[test]
    fn invalid_calendar_dates() {
        assert_eq!(Date::parse("2019-13"), Err(ParseError::OutOfRange));
        assert_eq!(Date::parse("2019-99"), Err(ParseError::OutOfRange));
        assert_eq!(Date::parse("2019-04-40"), Err(ParseError::OutOfRange));

        assert_eq!(Date::parse("2019-04-00"), Err(ParseError::OutOfRange));
        assert_eq!(Date::parse("2019-00-00"), Err(ParseError::OutOfRange));
        assert_eq!(Date::parse("2019-00-01"), Err(ParseError::OutOfRange));
        // well, year 0 is fine. It's just 1BCE.
        assert_eq!(Date::parse("0000-00-00"), Err(ParseError::OutOfRange));
        assert_eq!(Date::parse("0000-10-00"), Err(ParseError::OutOfRange));
    }

    #[test]
    fn seasons() {
        assert!(Date::parse("2019-21").is_ok());
        assert!(Date::parse("2019-22").is_ok());
        assert!(Date::parse("2019-23").is_ok());
        assert!(Date::parse("2019-24").is_ok());

        assert!(Date::parse("2019-20").is_err());
        assert!(Date::parse("2019-25").is_err());
    }

    #[test]
    fn seasons_day_invalid() {
        assert_eq!(Date::parse("2019-21-05"), Err(ParseError::Invalid));
    }
}
