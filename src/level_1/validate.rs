use crate::common::is_valid_complete_date;
use crate::ParseError::{self, *};
use core::convert::TryInto;

use super::{
    packed::{Certainty, DMFlags, DMMask, PackedInt, PackedU8, PackedYear, YearMask},
    parser::{ParsedEdtf, UnvalidatedDMEnum, UnvalidatedDate},
    Date, Edtf, YYear,
};

#[cfg(test)]
use super::Precision;

use crate::DateTime;

impl ParsedEdtf {
    pub(crate) fn validate(self) -> Result<Edtf, ParseError> {
        Ok(match self {
            Self::Date(d) => Edtf::Date(d.validate()?),
            Self::YYear(y) => {
                // this shouldn't come from the parser, because we look for a nonzero first digit
                // but good to check?
                // if scientific < 10_000 && scientific > -10_000 {
                //     return Err(ParseError::Invalid)
                // }
                Edtf::YYear(YYear::new_opt(y).ok_or(ParseError::Invalid)?)
            }
            Self::Interval(d, d2) => Edtf::Interval(d.validate()?, d2.validate()?),
            Self::DateTime(d, t) => Edtf::DateTime(DateTime::validate(d, t)?),
            Self::IntervalOpenFrom(start) => Edtf::IntervalOpenFrom(start.validate()?),
            Self::IntervalOpenTo(end) => Edtf::IntervalOpenTo(end.validate()?),
            Self::IntervalUnknownFrom(start) => Edtf::IntervalOpenFrom(start.validate()?),
            Self::IntervalUnknownTo(end) => Edtf::IntervalOpenTo(end.validate()?),
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
    pub(crate) fn value_u32(&self) -> Option<u32> {
        self.value().map(|x| x as u32)
    }
}

impl UnvalidatedDMEnum {
    pub(crate) fn validate(self) -> Result<PackedU8, ParseError> {
        let (val, flags) = match self {
            // we store 1 here, but check for the mask in PackedU8.value() and never use the 1
            Self::Unspecified => (1, DMFlags::new(Certainty::Certain, DMMask::Unspecified)),
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
    pub(crate) fn validate(self) -> Result<Date, ParseError> {
        validate(self)
    }
    pub(crate) fn from_ymd(year: i32, month: u32, day: u32) -> Self {
        UnvalidatedDate {
            year: (year, Default::default()),
            month: if month == 0 {
                None
            } else {
                month.try_into().ok().map(UnvalidatedDMEnum::Unmasked)
            },
            day: if day == 0 {
                None
            } else {
                day.try_into().ok().map(UnvalidatedDMEnum::Unmasked)
            },
            certainty: Default::default(),
        }
    }
}

#[cfg(test)]
macro_rules! test_roundtrip {
    ($x:literal) => {
        assert_eq!(Edtf::parse($x).unwrap().to_string(), $x);
    };
    ($x:literal, $y:literal) => {
        assert_eq!(Edtf::parse($x).unwrap().to_string(), $y);
    };
}

#[test]
fn test_lossless_roundtrip() {
    // dates and uncertainties
    test_roundtrip!("2019-08-17");
    test_roundtrip!("2019-08");
    test_roundtrip!("2019");
    test_roundtrip!("2019-08-17?");
    test_roundtrip!("2019-08?");
    test_roundtrip!("2019?");
    test_roundtrip!("2019-08-17~");
    test_roundtrip!("2019-08%");
    test_roundtrip!("2019%");
    // funky years
    test_roundtrip!("0043-08");
    test_roundtrip!("-0043-08");
    // timezones
    test_roundtrip!("2019-08-17T23:59:30");
    test_roundtrip!("2019-08-17T23:59:30Z");
    test_roundtrip!("2019-08-17T01:56:00+04:30");
    test_roundtrip!("2019-08-17T23:59:30+00");
    test_roundtrip!("2019-08-17T23:59:30+04");
    test_roundtrip!("2019-08-17T23:59:30-04");
    test_roundtrip!("2019-08-17T23:59:30+00:00");
    test_roundtrip!("2019-08-17T23:59:30+00:05");
    test_roundtrip!("2019-08-17T23:59:30+23:59");
    test_roundtrip!("2019-08-17T23:59:30-10:00");
    test_roundtrip!("2019-08-17T23:59:30-10:19");
}

#[test]
fn leap_second() {
    test_roundtrip!("2019-08-17T23:59:60Z");
    // no, leap seconds are always inserted at 23:59:60.
    // (unless they're removed, in which case 23:59:59 is removed.)
    assert_eq!(
        Edtf::parse("2019-08-17T22:59:60Z"),
        Err(ParseError::OutOfRange),
    );
    assert_eq!(
        Edtf::parse("2019-08-17T23:58:60Z"),
        Err(ParseError::OutOfRange),
    );
}

#[test]
fn match_precision() {
    let date = Date::parse("2019-09?").unwrap();
    assert_eq!(
        date.as_matcher(),
        (Precision::Month(2019, 9), Certainty::Uncertain)
    );
}

#[test]
fn masking_with_uncertain() {
    assert_eq!(
        Date::parse("201X?").unwrap().as_matcher(),
        (Precision::Decade(2010), Certainty::Uncertain)
    );
    assert_eq!(
        Date::parse("2019-XX?").unwrap().as_matcher(),
        (Precision::MonthOfYear(2019), Certainty::Uncertain)
    );
}

#[test]
fn ranges() {
    assert_eq!(
        Edtf::parse("2020/2021"),
        Ok((Date::from_year(2020), Date::from_year(2021)).into())
    );
    assert_eq!(
        Edtf::parse("2020?/2021"),
        Ok((
            Date::from_year(2020).and_certainty(Certainty::Uncertain),
            Date::from_year(2021)
        )
            .into())
    );
}
