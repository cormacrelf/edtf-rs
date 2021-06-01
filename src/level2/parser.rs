// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright © 2021 Corporation for Digital Scholarship

#[allow(unused_imports)]
use nom::{
    branch as nb, bytes::complete as nbc, character as nch, character::complete as ncc,
    combinator as nc, error::ParseError as NomParseError, sequence as ns, Finish, IResult, ParseTo,
    Parser,
};

use crate::{
    common::{signed_year_min_n, year_n_signed, StrResult},
    helpers::ParserExt,
    ParseError,
};

use super::api::ScientificYear;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ParsedEdtf {
    // Date(UnvalidatedDate),
    Scientific(ScientificYear),
    // Range(UnvalidatedDate, UnvalidatedDate),
    // RangeOpenEnd(UnvalidatedDate),
    // RangeOpenStart(UnvalidatedDate),
    // RangeUnknownStart(UnvalidatedDate),
    // RangeUnknownEnd(UnvalidatedDate),
    // DateTime(DateComplete, UnvalidatedTime),
}

impl ParsedEdtf {
    pub(crate) fn parse_inner(input: &str) -> Result<ParsedEdtf, ParseError> {
        level2
            .complete()
            .parse(input)
            // parser already fails on trailing chars
            .map(|(_, a)| a)
            .map_err(|_| ParseError::Invalid)
    }
}

fn level2(input: &str) -> StrResult<ParsedEdtf> {
    let mut sci = scientific.map(|sy| ParsedEdtf::Scientific(sy));
    // let dt = date_time.map(|(d, t)| ParsedEdtf::DateTime(d, t));
    // let single = date_certainty.complete().map(ParsedEdtf::Date);
    // let range = date_range.map(|(a, b)| ParsedEdtf::Range(a, b));
    //
    // let ru_start = range_unknown_start.map(ParsedEdtf::RangeUnknownStart);
    // let ru_end = range_unknown_end.map(ParsedEdtf::RangeUnknownEnd);
    // let ro_start = range_open_start.map(ParsedEdtf::RangeOpenStart);
    // let ro_end = range_open_end.map(ParsedEdtf::RangeOpenEnd);

    sci
        // .or(single)
        // .or(dt)
        // .or(range)
        // .or(ru_start)
        // .or(ru_end)
        // .or(ro_start)
        // .or(ro_end)
        .parse(input)
}

fn scientific(remain: &str) -> StrResult<ScientificYear> {
    // note: when we write these back out, the ScientificYear will have a `Y` prefix whenever the
    // year is more than 4 digits long. That's lossless.
    scientific_y.or(scientific_4digit).complete().parse(remain)
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
        .map(|x| x.1);
    let sig_digits = opt_s
        .map(|s| nom::parse_to!(s, u16))
        .transpose()?
        .map(|x| x.1);
    Ok((
        remain,
        ScientificYear {
            mantissa,
            exponent,
            sig_digits,
        },
    ))
}

fn scientific_4digit(remain: &str) -> StrResult<ScientificYear> {
    let s = ncc::char('S');
    let (remain, (year, sd)) = year_n_signed(4)
        .and(ns::preceded(s, ncc::digit1))
        .parse(remain)?;
    let (_, sd) = nom::parse_to!(sd, u16)?;
    Ok((
        remain,
        ScientificYear {
            mantissa: year as i64,
            exponent: None,
            sig_digits: Some(sd),
        },
    ))
}
