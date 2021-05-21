#[allow(unused_imports)]
use nom::{
    branch as nb, bytes::complete as nbc, character as nch, character::complete as ncc,
    combinator as nc, error::ParseError as NomParseError, sequence as ns, Finish, IResult, ParseTo,
    Parser,
};

use crate::ParseError;
use crate::common::{hyphen, take_n_digits, two_digits, StrResult};
use crate::helpers::ParserExt;

use super::packed::{
    Certainty::{self, *},
    DayMonthCertainty as DMCertainty, DayMonthMask as DMMask, YearCertainty, YearMask,
};

impl super::Date {
    pub(crate) fn parse_inner(input: &str) -> Result<UnvalidatedDate, ParseError> {
        date_certainty
            .complete()
            .parse(input)
            // parser already fails on trailing chars
            .map(|(_, a)| a)
            .map_err(|_| ParseError::Invalid)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) struct UnvalidatedDate {
    pub year: (i32, YearCertainty),
    pub month: Option<(u8, DMCertainty)>,
    pub day: Option<(u8, DMCertainty)>,
}

pub(crate) fn date_certainty(input: &str) -> StrResult<UnvalidatedDate> {
    year_certainty
        .and(
            ns::preceded(hyphen, two_digits_certainty)
                .and(ns::preceded(hyphen, two_digits_certainty).optional())
                .optional(),
        )
        .map(|(year, rest)| {
            let month = rest.map(|(m, _)| m);
            let day = rest.and_then(|(_, d)| d);
            UnvalidatedDate { year, month, day }
        })
        .parse(input)
}

fn certainty(input: &str) -> StrResult<Certainty> {
    let present = ncc::char('?').map(|_| Uncertain);
    let present = present.or(ncc::char('~').map(|_| Approximate));
    let present = present.or(ncc::char('%').map(|_| ApproximateUncertain));
    present
        .optional()
        .map(|o| o.map_or(Certain, |x| x))
        .parse(input)
}

fn year_n(n: usize) -> impl FnMut(&str) -> StrResult<i32> {
    move |remain| {
        let (remain, four) = take_n_digits(n)(remain)?;
        let (_, parsed) = nom::parse_to!(four, i32)?;
        Ok((remain, parsed))
    }
}

fn year_certainty(input: &str) -> StrResult<(i32, YearCertainty)> {
    let double_mask = year_n(2)
        .and_ignore(nbc::tag("XX"))
        .map(|i| (i * 100, YearMask::Two.into()));
    let single_mask = year_n(3)
        .and_ignore(ncc::char('X'))
        .map(|i| (i * 10, YearMask::One.into()));
    let dig_cert = year_n(4).and(certainty.map(|c| c.into()));
    double_mask.or(single_mask).or(dig_cert).parse(input)
}

fn two_digits_certainty(input: &str) -> StrResult<(u8, DMCertainty)> {
    let masked = nbc::tag("XX").map(|_| (0, DMMask::Masked.into()));
    let dig_cert = two_digits::<u8>.and(certainty.map(DMCertainty::from));
    masked.or(dig_cert).parse(input)
}

#[cfg(test)]
mod test {
    use crate::level1::packed::{
        Certainty::*, DayMonthCertainty as DMCertainty, DayMonthMask as DMMask, YearCertainty,
        YearMask,
    };

    use super::UnvalidatedDate;

    #[test]
    fn uncertain_date() {
        assert_eq!(
            super::date_certainty("2019?"),
            Ok((
                "",
                UnvalidatedDate {
                    year: (2019, YearCertainty::new(Uncertain, YearMask::None)),
                    month: None,
                    day: None,
                }
            ))
        );

        assert_eq!(
            super::date_certainty("2019?-05~"),
            Ok((
                "",
                UnvalidatedDate {
                    year: (2019, YearCertainty::new(Uncertain, YearMask::None)),
                    month: Some((5, DMCertainty::new(Approximate, DMMask::None))),
                    day: None,
                }
            ))
        );

        assert_eq!(
            super::date_certainty("2019%-05?"),
            Ok((
                "",
                UnvalidatedDate {
                    year: (2019, YearCertainty::new(ApproximateUncertain, YearMask::None)),
                    month: Some((5, DMCertainty::new(Uncertain, DMMask::None))),
                    day: None,
                }
            ))
        );

        assert_eq!(
            super::date_certainty("2019-05?-09~"),
            Ok((
                "",
                UnvalidatedDate {
                    year: (2019, YearCertainty::new(Certain, YearMask::None)),
                    month: Some((5, DMCertainty::new(Uncertain, DMMask::None))),
                    day: Some((9, DMCertainty::new(Approximate, DMMask::None))),
                }
            ))
        );

        assert_eq!(
            super::date_certainty("2019-05-09"),
            Ok((
                "",
                UnvalidatedDate {
                    year: (2019, YearCertainty::new(Certain, YearMask::None)),
                    month: Some((5, DMCertainty::new(Certain, DMMask::None))),
                    day: Some((9, DMCertainty::new(Certain, DMMask::None))),
                }
            ))
        );

    }
}
