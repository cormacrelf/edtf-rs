use crate::common::is_valid_complete_date;
use crate::ParseError;
use ParseError::*;

pub mod api;
mod packed;
mod parser;

use self::{
    api::{Date, DateTime, Edtf},
    packed::{Certainty, DMFlags, DMMask, PackedInt, PackedU8, PackedYear, YearMask},
    parser::{ParsedEdtf, UnvalidatedDMEnum, UnvalidatedDate},
};

impl ParsedEdtf {
    fn validate(self) -> Result<Edtf, ParseError> {
        Ok(match self {
            Self::Date(d) => Edtf::Date(d.validate()?),
            Self::Scientific(scientific) => {
                // this shouldn't come from the parser, because we look for a nonzero first digit
                // but good to check?
                // if scientific < 10_000 && scientific > -10_000 {
                //     return Err(ParseError::Invalid)
                // }
                Edtf::Scientific(scientific)
            },
            Self::Range(d, d2) => Edtf::Range(d.validate()?, d2.validate()?),
            Self::DateTime(d, t) => Edtf::DateTime(DateTime::validate(d, t)?),
            Self::RangeOpenStart(start) => Edtf::RangeOpenStart(start.validate()?),
            Self::RangeOpenEnd(end) => Edtf::RangeOpenEnd(end.validate()?),
            Self::RangeUnknownStart(start) => Edtf::RangeOpenStart(start.validate()?),
            Self::RangeUnknownEnd(end) => Edtf::RangeOpenEnd(end.validate()?),
        })
    }
}

impl PackedU8 {
    fn is_masked(&self) -> bool {
        let (_, flags) = self.unpack();
        flags.is_masked()
    }
    fn certainty(&self) -> Certainty {
        let (_, flags) = self.unpack();
        flags.certainty
    }
    fn value(&self) -> Option<u8> {
        let (val, flags) = self.unpack();
        if flags.is_masked() {
            None
        } else {
            Some(val)
        }
    }
}

impl UnvalidatedDMEnum {
    pub(crate) fn validate(self) -> Result<PackedU8, ParseError> {
        let (val, flags) = match self {
            // we store 1 here, but check for the mask in PackedU8.value() and never use the 1
            Self::Masked => (1, DMFlags::new(Certainty::Certain, DMMask::Masked)),
            Self::Unmasked(v) => (v, DMFlags::new(Certainty::Certain, DMMask::None)),
        };
        PackedU8::pack(val, flags).ok_or(ParseError::OutOfRange)
    }
}

fn validate(date: UnvalidatedDate) -> Result<Date, ParseError> {
    let UnvalidatedDate {
        year,
        month,
        day,
        certainty,
    } = date;
    let month = month.as_ref().map(|m| m.validate()).transpose()?;
    let day = day.as_ref().map(|m| m.validate()).transpose()?;

    // eprintln!("\ncheck_structure: {:?}", date);
    match (month, day) {
        // this can't happen if you're parsing, but people might try to construct a date like this
        // manually with zero values
        (None, Some(_)) => return Err(OutOfRange),
        _ => {}
    }

    // mask rules
    // this is funny in 2021
    // eprintln!("    check_masks: {:?}", date);
    let year_mask = year.1.mask != YearMask::None;
    let month_mask = month.as_ref().map(|x| x.is_masked());
    let day_mask = day.as_ref().map(|x| x.is_masked());
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

    // eprintln!("   check_values: {:?}", date);
    let year_val = year.0;
    let month_val = month.as_ref().and_then(|x| x.value());
    let day_val = day.as_ref().and_then(|x| x.value());
    match (month_val, day_val) {
        // not a month (i.e. a season), but day provided
        (Some(m), Some(_)) if m > 12 => return Err(Invalid),
        (Some(m), None) if (m >= 1 && m <= 12) || (m >= 21 && m <= 24) => {}
        (Some(m), Some(d)) if m >= 1 && m <= 12 => {
            let _complete = is_valid_complete_date(year_val, m, d)?;
        }
        (None, None) => {}
        // _ => panic!("not ok: {:?}", (month_val, day_val)),
        _ => return Err(OutOfRange),
    }
    let date = Date {
        year: PackedYear::pack(date.year.0, date.year.1).ok_or(ParseError::OutOfRange)?,
        month,
        day,
        certainty,
    };
    Ok(date)
}

impl UnvalidatedDate {
    fn validate(self) -> Result<Date, ParseError> {
        validate(self)
    }
    pub(crate) fn from_ymd(year: i32, month: u8, day: u8) -> Self {
        UnvalidatedDate {
            year: (year, Default::default()),
            month: if month == 0 {
                None
            } else {
                Some(UnvalidatedDMEnum::Unmasked(month))
            },
            day: if day == 0 {
                None
            } else {
                Some(UnvalidatedDMEnum::Unmasked(day))
            },
            certainty: Default::default(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::api::Edtf;
    use super::packed::Certainty::*;
    use super::*;

    #[test]
    fn uncertain_dates_packed() {
        use self::packed::PackedInt;
        let d = Date::parse("2019-07-05%").unwrap();
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
        assert_eq!(
            Date::parse("201X").as_ref().map(Date::as_precision),
            Ok(super::api::DatePrecision::Year(2010, YearMask::OneDigit))
        );
        assert_eq!(
            Date::parse("20XX"),
            Ok(Date::from_year_masked(2000, YearMask::TwoDigits))
        );
        // same, because we round it
        assert_eq!(
            Date::parse("20XX"),
            Ok(Date::from_year_masked(2019, YearMask::TwoDigits))
        );

        assert_eq!(
            Date::parse("2019-XX"),
            Ok(Date::from_year_masked_month(2019))
        );
        assert_eq!(
            Date::parse("2019-XX-XX"),
            Ok(Date::from_year_masked_month_day(2019))
        );
        assert_eq!(
            Date::parse("2019-07-XX"),
            Ok(Date::from_ym_masked_day(2019, 7))
        );
        // no
        assert_eq!(Date::parse("2019-XX-09"), Err(Invalid));
        assert_eq!(Date::parse("201X-XX"), Err(Invalid));
        assert_eq!(Date::parse("20XX-XX"), Err(Invalid));
        assert_eq!(Date::parse("20XX-07"), Err(Invalid));
        assert_eq!(Date::parse("201X-XX-09"), Err(Invalid));
        assert_eq!(Date::parse("201X-07-09"), Err(Invalid));
        assert_eq!(Date::parse("20XX-07-09"), Err(Invalid));
        assert_eq!(Date::parse("20XX-07-XX"), Err(Invalid));
        assert_eq!(Date::parse("20XX-07-0X"), Err(Invalid));
        // Don't think you can reasonably rely on this being Invalid or OutOfRange, it's both
        assert!(Date::parse("2019-XX-00").is_err());
        assert_eq!(Date::parse("2019-0X-00"), Err(Invalid));
        assert_eq!(Date::parse("2019-0X-XX"), Err(Invalid));
    }

    #[test]
    fn no_uncertain_mid_date() {
        // yes
        assert_eq!(
            Date::parse("2019-08-08?"),
            Ok(Date::from_ymd(2019, 8, 8).and_certainty(Uncertain))
        );
        // no
        assert_eq!(Date::parse("2019?-08-08"), Err(ParseError::Invalid));
        assert_eq!(Date::parse("2019-08%-08"), Err(ParseError::Invalid));
        assert_eq!(Date::parse("2019-08?-08%"), Err(ParseError::Invalid));
        assert_eq!(Date::parse("2019?-08-08%"), Err(ParseError::Invalid));
        assert_eq!(Date::parse("2019~-08-08?"), Err(ParseError::Invalid));
        assert_eq!(Date::parse("2019~-08?-08~"), Err(ParseError::Invalid));
        assert_eq!(Date::parse("2019~-08~-08~"), Err(ParseError::Invalid));
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
        assert!(Date::parse("2019-07-XX?").is_ok());
        assert!(Date::parse("2019-07-XX~").is_ok());
        assert!(Date::parse("2019-07-XX%").is_ok());
    }

    #[test]
    fn invalid_calendar_dates() {
        // bad values
        assert_eq!(Date::parse("2019-13"), Err(ParseError::OutOfRange));
        assert_eq!(Date::parse("2019-99"), Err(ParseError::OutOfRange));
        assert_eq!(Date::parse("2019-04-40"), Err(ParseError::OutOfRange));
        assert_eq!(Date::parse("2019-99-99"), Err(ParseError::OutOfRange));
        // bad values inside range of PackedU8
        assert_eq!(Date::parse("2019-00"), Err(ParseError::OutOfRange));
        assert_eq!(Date::parse("2019-30-00"), Err(ParseError::OutOfRange));
        // more zeroes
        assert_eq!(Date::parse("2019-04-00"), Err(ParseError::OutOfRange));
        assert_eq!(Date::parse("2019-00-00"), Err(ParseError::OutOfRange));
        assert_eq!(Date::parse("2019-00-01"), Err(ParseError::OutOfRange));
        // well, year 0 is fine. It's just 1BCE.
        assert_eq!(Date::parse("0000-00-00"), Err(ParseError::OutOfRange));
        assert_eq!(Date::parse("0000-10-00"), Err(ParseError::OutOfRange));
    }

    #[test]
    fn seasons() {
        // yes
        assert!(Date::parse("2019-21").is_ok());
        assert!(Date::parse("2019-22").is_ok());
        assert!(Date::parse("2019-23").is_ok());
        assert!(Date::parse("2019-24").is_ok());
        // no
        assert_eq!(Date::parse("2019-13"), Err(ParseError::OutOfRange));
        assert_eq!(Date::parse("2019-20"), Err(ParseError::OutOfRange));
        assert_eq!(Date::parse("2019-25"), Err(ParseError::OutOfRange));
    }

    #[test]
    fn seasons_day_invalid() {
        assert_eq!(Date::parse("2019-21-05"), Err(ParseError::Invalid));
    }

    fn scientific_l1() {
        // yes - 5+ digits
        assert_eq!(
            Edtf::parse("Y157900"),
            Ok(Edtf::Scientific(157900))
        );
        assert_eq!(
            Edtf::parse("Y1234567890"),
            Ok(Edtf::Scientific(1234567890))
        );
        assert_eq!(
            Edtf::parse("Y-1234567890"),
            Ok(Edtf::Scientific(-1234567890))
        );
        // no -- <= 4 digits
        assert_eq!(Edtf::parse("Y1745"), Err(ParseError::Invalid));
    }

    #[test]
    fn negative_calendar_dates() {
        // yes
        assert_eq!(
            Edtf::parse("-1900-07-05"),
            Ok(Edtf::Date(Date::from_ymd(-1900, 7, 5)))
        );
        assert_eq!(
            Edtf::parse("-9999-07-05"),
            Ok(Edtf::Date(Date::from_ymd(-9999, 7, 5)))
        );
        // no - fewer than four digits
        assert_eq!(Edtf::parse("-999-07-05"), Err(ParseError::Invalid));
        // no - negative zero not allowed
        assert_eq!(Edtf::parse("-0000-07-05"), Err(ParseError::Invalid));
    }
}
