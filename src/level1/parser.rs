#[allow(unused_imports)]
use nom::{
    branch as nb, bytes::complete as nbc, character as nch, character::complete as ncc,
    combinator as nc, error::ParseError as NomParseError, sequence as ns, Finish, IResult, ParseTo,
    Parser,
};

use crate::common::{hyphen, two_digits, year_n, StrResult};
use crate::helpers::ParserExt;
use crate::ParseError;

use super::packed::{
    Certainty::{self, *},
    DMEnum, YearFlags, YearMask,
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
    pub year: (i32, YearFlags),
    pub month: Option<DMEnum>,
    pub day: Option<DMEnum>,
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
        .map(|i| (i * 100, YearMask::Two.into()));
    let single_mask = year_n(3)
        .and_ignore(ncc::char('X'))
        .map(|i| (i * 10, YearMask::One.into()));
    let dig_cert = year_n(4).and(certainty.map(|c| c.into()));
    double_mask.or(single_mask).or(dig_cert).parse(input)
}

fn two_digits_certainty(input: &str) -> StrResult<DMEnum> {
    let masked = nbc::tag("XX").map(|_| DMEnum::Masked);
    let dig_cert = two_digits
        .and(certainty)
        .map(|x| DMEnum::Unmasked(x.0, x.1));
    masked.or(dig_cert).parse(input)
}

#[cfg(test)]
mod test {
    use std::num::NonZeroU8;

    use crate::level1::packed::{YearFlags, YearMask};

    use super::*;

    #[test]
    fn unspecified_date() {
        assert_eq!(
            super::date_certainty("2019-XX"),
            Ok((
                "",
                UnvalidatedDate {
                    year: (2019, Certain.into()),
                    month: Some(DMEnum::Masked),
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
                    month: Some(DMEnum::Unmasked(NonZeroU8::new(5).unwrap(), Approximate)),
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
                    month: Some(DMEnum::Unmasked(NonZeroU8::new(5).unwrap(), Uncertain)),
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
                    month: Some(DMEnum::Unmasked(NonZeroU8::new(5).unwrap(), Uncertain)),
                    day: Some(DMEnum::Unmasked(NonZeroU8::new(9).unwrap(), Approximate)),
                }
            ))
        );

        assert_eq!(
            super::date_certainty("2019-05-09"),
            Ok((
                "",
                UnvalidatedDate {
                    year: (2019, YearFlags::new(Certain, YearMask::None)),
                    month: Some(DMEnum::Unmasked(NonZeroU8::new(5).unwrap(), Certain)),
                    day: Some(DMEnum::Unmasked(NonZeroU8::new(9).unwrap(), Certain)),
                }
            ))
        );
    }
}
