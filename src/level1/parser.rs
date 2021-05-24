#[allow(unused_imports)]
use nom::{
    branch as nb, bytes::complete as nbc, character as nch, character::complete as ncc,
    combinator as nc, error::ParseError as NomParseError, sequence as ns, Finish, IResult, ParseTo,
    Parser,
};

use crate::common::{hyphen, two_digits, year_n, StrResult};
use crate::helpers::ParserExt;
use crate::ParseError;

use super::packed::{Certainty::{self, *}, YearFlags, YearMask};

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
    pub year: (i32, YearFlags),
    pub month: Option<UnvalidatedDMEnum>,
    pub day: Option<UnvalidatedDMEnum>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum UnvalidatedDMEnum {
    Masked(Certainty),
    Unmasked(u8, Certainty),
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

fn year_certainty(input: &str) -> StrResult<(i32, YearFlags)> {
    let double_mask = year_n(2)
        .and_ignore(nbc::tag("XX"))
        .and(certainty)
        .map(|(i, c)| (i * 100, YearFlags::new(c, YearMask::TwoDigits)));
    let single_mask = year_n(3)
        .and_ignore(ncc::char('X'))
        .and(certainty)
        .map(|(i, c)| (i * 10, YearFlags::new(c, YearMask::OneDigit)));
    let dig_cert = year_n(4).and(certainty.map(|c| c.into()));
    double_mask.or(single_mask).or(dig_cert).parse(input)
}

fn two_digits_certainty(input: &str) -> StrResult<UnvalidatedDMEnum> {
    let masked = nbc::tag("XX").and(certainty).map(|(_, c)| UnvalidatedDMEnum::Masked(c));
    let dig_cert = two_digits
        .and(certainty)
        .map(|x| UnvalidatedDMEnum::Unmasked(x.0, x.1));
    masked.or(dig_cert).parse(input)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::level1::packed::{YearFlags, YearMask};

    #[test]
    fn unspecified_date() {
        assert_eq!(
            super::date_certainty("2019-XX"),
            Ok((
                "",
                UnvalidatedDate {
                    year: (2019, Certain.into()),
                    month: Some(UnvalidatedDMEnum::Masked(Certainty::Certain)),
                    day: None,
                }
            ))
        );
    }

    #[test]
    fn uncertain_date() {
        assert_eq!(
            super::date_certainty("2019?"),
            Ok((
                "",
                UnvalidatedDate {
                    year: (2019, YearFlags::new(Uncertain, YearMask::None)),
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
                    year: (2019, YearFlags::new(Uncertain, YearMask::None)),
                    month: Some(UnvalidatedDMEnum::Unmasked(5, Approximate)),
                    day: None,
                }
            ))
        );

        assert_eq!(
            super::date_certainty("2019%-05?"),
            Ok((
                "",
                UnvalidatedDate {
                    year: (2019, YearFlags::new(ApproximateUncertain, YearMask::None)),
                    month: Some(UnvalidatedDMEnum::Unmasked(5, Uncertain)),
                    day: None,
                }
            ))
        );

        assert_eq!(
            super::date_certainty("2019-05?-09~"),
            Ok((
                "",
                UnvalidatedDate {
                    year: (2019, YearFlags::new(Certain, YearMask::None)),
                    month: Some(UnvalidatedDMEnum::Unmasked(5, Uncertain)),
                    day: Some(UnvalidatedDMEnum::Unmasked(9, Approximate)),
                }
            ))
        );

        assert_eq!(
            super::date_certainty("2019-05-09"),
            Ok((
                "",
                UnvalidatedDate {
                    year: (2019, YearFlags::new(Certain, YearMask::None)),
                    month: Some(UnvalidatedDMEnum::Unmasked(5, Certain)),
                    day: Some(UnvalidatedDMEnum::Unmasked(9, Certain)),
                }
            ))
        );
    }
}
