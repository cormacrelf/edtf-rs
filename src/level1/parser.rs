#[allow(unused_imports)]
use nom::{
    branch as nb, bytes::complete as nbc, character as nch, character::complete as ncc,
    combinator as nc, error::ParseError as NomParseError, sequence as ns, Finish, IResult, ParseTo,
    Parser,
};

use crate::common::{
    date_time, hyphen, two_digits, year_n, DateComplete, StrResult, UnvalidatedTime,
};
use crate::helpers::ParserExt;
use crate::ParseError;

use super::packed::{
    Certainty::{self, *},
    YearFlags, YearMask,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ParsedEdtf {
    /// Should not contain DateTime variant
    Date(UnvalidatedDate),
    Range(UnvalidatedDate, UnvalidatedDate),
    RangeOpenEnd(UnvalidatedDate),
    RangeOpenStart(UnvalidatedDate),
    RangeUnknownStart(UnvalidatedDate),
    RangeUnknownEnd(UnvalidatedDate),
    DateTime(DateComplete, UnvalidatedTime),
}

impl ParsedEdtf {
    pub(crate) fn parse_inner(input: &str) -> Result<ParsedEdtf, ParseError> {
        level1
            .complete()
            .parse(input)
            // parser already fails on trailing chars
            .map(|(_, a)| a)
            .map_err(|_| ParseError::Invalid)
    }
}

fn level1(input: &str) -> StrResult<ParsedEdtf> {
    let dt = date_time.map(|(d, t)| ParsedEdtf::DateTime(d, t));
    let single = date_certainty.complete().map(ParsedEdtf::Date);
    let range = date_range.map(|(a, b)| ParsedEdtf::Range(a, b));

    let ru_start = range_unknown_start.map(ParsedEdtf::RangeUnknownStart);
    let ru_end = range_unknown_end.map(ParsedEdtf::RangeUnknownEnd);
    let ro_start = range_open_start.map(ParsedEdtf::RangeOpenStart);
    let ro_end = range_open_end.map(ParsedEdtf::RangeOpenEnd);

    single
        .or(dt)
        .or(range)
        .or(ru_start)
        .or(ru_end)
        .or(ro_start)
        .or(ro_end)
        .parse(input)
}

fn range_open_start(remain: &str) -> StrResult<UnvalidatedDate> {
    date_certainty
        .and_ignore(nbc::tag("/.."))
        .complete()
        .parse(remain)
}

fn range_open_end(remain: &str) -> StrResult<UnvalidatedDate> {
    ns::preceded(nbc::tag("../"), date_certainty)
        .complete()
        .parse(remain)
}

fn range_unknown_start(remain: &str) -> StrResult<UnvalidatedDate> {
    date_certainty
        .and_ignore(ncc::char('/'))
        .complete()
        .parse(remain)
}

fn range_unknown_end(remain: &str) -> StrResult<UnvalidatedDate> {
    ns::preceded(ncc::char('/'), date_certainty)
        .complete()
        .parse(remain)
}

fn date_range(remain: &str) -> StrResult<(UnvalidatedDate, UnvalidatedDate)> {
    date_certainty
        .and_ignore(ncc::char('/'))
        .and(date_certainty)
        .complete()
        .parse(remain)
}

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

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub(crate) struct UnvalidatedDate {
    pub year: (i32, YearFlags),
    pub month: Option<UnvalidatedDMEnum>,
    pub day: Option<UnvalidatedDMEnum>,
    pub certainty: Certainty,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum UnvalidatedDMEnum {
    Masked,
    Unmasked(u8),
}

pub(crate) fn date_certainty(input: &str) -> StrResult<UnvalidatedDate> {
    year_maybe_mask
        .and(
            ns::preceded(hyphen, two_digits_maybe_mask)
                .and(ns::preceded(hyphen, two_digits_maybe_mask).optional())
                .optional(),
        )
        .and(certainty)
        .map(|((year, rest), certainty)| {
            let month = rest.map(|(m, _)| m);
            let day = rest.and_then(|(_, d)| d);
            UnvalidatedDate {
                year,
                month,
                day,
                certainty,
            }
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

fn year_maybe_mask(input: &str) -> StrResult<(i32, YearFlags)> {
    let double_mask = year_n(2)
        .and_ignore(nbc::tag("XX"))
        .map(|i| (i * 100, YearMask::TwoDigits.into()));
    let single_mask = year_n(3)
        .and_ignore(ncc::char('X'))
        .map(|i| (i * 10, YearMask::OneDigit.into()));
    let dig_cert = year_n(4).map(|x| (x, Certain.into()));
    double_mask.or(single_mask).or(dig_cert).parse(input)
}

fn two_digits_maybe_mask(input: &str) -> StrResult<UnvalidatedDMEnum> {
    let masked = nbc::tag("XX").map(|_| UnvalidatedDMEnum::Masked);
    let dig_cert = two_digits.map(|x| UnvalidatedDMEnum::Unmasked(x));
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
                    month: Some(UnvalidatedDMEnum::Masked),
                    day: None,
                    certainty: Certain,
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
                    year: (2019, YearMask::None.into()),
                    month: None,
                    day: None,
                    certainty: Uncertain,
                }
            ))
        );

        assert_eq!(
            super::date_certainty("2019-05~"),
            Ok((
                "",
                UnvalidatedDate {
                    year: (2019, YearFlags::new(Certain, YearMask::None)),
                    month: Some(UnvalidatedDMEnum::Unmasked(5)),
                    day: None,
                    certainty: Approximate,
                }
            ))
        );

        assert_eq!(
            super::date_certainty("2019-05?"),
            Ok((
                "",
                UnvalidatedDate {
                    year: (2019, YearMask::None.into()),
                    month: Some(UnvalidatedDMEnum::Unmasked(5)),
                    day: None,
                    certainty: Uncertain,
                }
            ))
        );

        assert_eq!(
            super::date_certainty("2019-05-09~"),
            Ok((
                "",
                UnvalidatedDate {
                    year: (2019, YearMask::None.into()),
                    month: Some(UnvalidatedDMEnum::Unmasked(5)),
                    day: Some(UnvalidatedDMEnum::Unmasked(9)),
                    certainty: Approximate,
                }
            ))
        );

        assert_eq!(
            super::date_certainty("2019-05-09"),
            Ok((
                "",
                UnvalidatedDate {
                    year: (2019, YearFlags::new(Certain, YearMask::None)),
                    month: Some(UnvalidatedDMEnum::Unmasked(5)),
                    day: Some(UnvalidatedDMEnum::Unmasked(9)),
                    certainty: Certain,
                }
            ))
        );
    }
}
