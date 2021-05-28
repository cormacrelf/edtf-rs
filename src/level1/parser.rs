#[allow(unused_imports)]
use nom::{
    branch as nb, bytes::complete as nbc, character as nch, character::complete as ncc,
    combinator as nc, error::ParseError as NomParseError, sequence as ns, Finish, IResult, ParseTo,
    Parser,
};

use crate::{
    common::{date_time, hyphen, two_digits, year_n, DateComplete, StrResult, UnvalidatedTime},
    helpers::ParserExt,
    level_1::ScientificYear,
};
use crate::{
    common::{minus_sign, take_min_n_digits},
    ParseError,
};

use super::packed::{
    Certainty::{self, *},
    YearFlags, YearMask,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ParsedEdtf {
    Date(UnvalidatedDate),
    Scientific(ScientificYear),
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
    let sci = scientific.map(|sy| ParsedEdtf::Scientific(sy));
    let dt = date_time.map(|(d, t)| ParsedEdtf::DateTime(d, t));
    let single = date_certainty.complete().map(ParsedEdtf::Date);
    let range = date_range.map(|(a, b)| ParsedEdtf::Range(a, b));

    let ru_start = range_unknown_start.map(ParsedEdtf::RangeUnknownStart);
    let ru_end = range_unknown_end.map(ParsedEdtf::RangeUnknownEnd);
    let ro_start = range_open_start.map(ParsedEdtf::RangeOpenStart);
    let ro_end = range_open_end.map(ParsedEdtf::RangeOpenEnd);

    sci.or(single)
        .or(dt)
        .or(range)
        .or(ru_start)
        .or(ru_end)
        .or(ro_start)
        .or(ro_end)
        .parse(input)
}

fn scientific(remain: &str) -> StrResult<ScientificYear> {
    // note: when we write these back out, the ScientificYear will have a `Y` prefix whenever the
    // year is more than 4 digits long. That's lossless.
    scientific_y.or(scientific_4digit).complete().parse(remain)
}

fn scientific_4digit(remain: &str) -> StrResult<ScientificYear> {
    let s = ncc::char('S');
    let (remain, (year, sd)) = year_n(4).and(ns::preceded(s, ncc::digit1)).parse(remain)?;
    let (_, sd) = nom::parse_to!(sd, u16)?;
    Ok((
        remain,
        ScientificYear {
            mantissa: year as i64,
            exponent: 0,
            significant_digits: sd,
        },
    ))
}

pub fn signed_year_min_n(n: usize) -> impl FnMut(&str) -> StrResult<i64> {
    move |remain| {
        let (remain, sign) = minus_sign(remain, -1i64, 1i64)?;
        let (remain, digs) = take_min_n_digits(n)(remain)?;
        let (_, parsed) = nom::parse_to!(digs, i64)?;
        Ok((remain, parsed * sign))
    }
}

/// Allows either Y{digit1}E7 or Y{min 5 digits}, followed by optional S3 suffix.
/// So if Y is less than 5 digits, E is mandatory.
fn scientific_y(remain: &str) -> StrResult<ScientificYear> {
    let e = ncc::char('E');
    let s = ncc::char('S');
    let (remain, ((mantissa, opt_e), opt_s)) = ns::preceded(ncc::char('Y'), signed_year_min_n(1))
        .and(ns::preceded(e, ncc::digit1).map(Some))
        .or(ns::preceded(ncc::char('Y'), signed_year_min_n(5)).map(|y| (y, None)))
        .and(ns::preceded(s, ncc::digit1).optional())
        .parse(remain)?;
    let exponent = opt_e
        .map(|e| nom::parse_to!(e, u16))
        .transpose()?
        .map_or(0, |x| x.1);
    let significant_digits = opt_s
        .map(|s| nom::parse_to!(s, u16))
        .transpose()?
        .map_or(0, |x| x.1);
    Ok((
        remain,
        ScientificYear {
            mantissa,
            exponent,
            significant_digits,
        },
    ))
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
