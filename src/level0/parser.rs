// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright © 2021 Corporation for Digital Scholarship

use crate::helpers::ParserExt;
use crate::ParseError;

#[allow(unused_imports)]
use nom::{
    branch as nb, bytes::complete as nbc, character as nch, character::complete as ncc,
    combinator as nc, error::ParseError as NomParseError, sequence as ns, Finish, IResult, ParseTo,
    Parser,
};

use super::{Date, DateComplete, Edtf};
use crate::common::{
    date_time, maybe_hyphen, take_n_digits, two_digits, StrResult, UnvalidatedTime,
};

impl Edtf {
    pub(crate) fn parse_inner(input: &str) -> Result<ParsedEdtf, ParseError> {
        level0
            .complete()
            .parse(input)
            // parser already fails on trailing chars
            .map(|(_, a)| a)
            .map_err(|_| ParseError::Invalid)
    }
}

impl Date {
    pub(crate) fn parse_inner(input: &str) -> Result<Self, ParseError> {
        date.complete()
            .parse(input)
            .map(|(_, a)| a)
            .map_err(|_| ParseError::Invalid)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum ParsedEdtf {
    Date(Date),
    Interval(Date, Date),
    DateTime(DateComplete, UnvalidatedTime),
}

fn level0(remain: &str) -> StrResult<ParsedEdtf> {
    let dt = date_time.map(|(d, t)| ParsedEdtf::DateTime(d, t));
    let range = date_range.map(|(a, b)| ParsedEdtf::Interval(a, b));
    let single = date.map(ParsedEdtf::Date);

    dt.or(range).or(single).parse(remain)
}

fn date_range(remain: &str) -> StrResult<(Date, Date)> {
    date.and_ignore(ncc::char('/')).and(date).parse(remain)
}

/// [date_complete] or one of the reduced precision variants
/// Level 0 only, no uncertainty etc.
pub(crate) fn date(remain: &str) -> StrResult<Date> {
    let (remain, year) = year4(remain)?;
    let (remain, is_hyphen) = maybe_hyphen(remain);
    if !is_hyphen {
        return Ok((remain, Date::new_unvalidated(year, None, None)));
    }
    let (remain, month) = two_digits(remain)?;
    let (remain, is_hyphen) = maybe_hyphen(remain);
    if !is_hyphen {
        return Ok((remain, Date::new_unvalidated(year, Some(month), None)));
    }
    let (remain, day) = two_digits(remain)?;
    Ok((remain, Date::new_unvalidated(year, Some(month), Some(day))))
}

/// Level 0 year only, so simply exactly four digits 0-9. That's it.
fn year4(remain: &str) -> StrResult<i32> {
    let (remain, four) = take_n_digits(4)(remain)?;
    let (_, parsed) = nom::parse_to!(four, i32)?;
    Ok((remain, parsed))
}

#[cfg(test)]
mod test {
    use super::*;
    use core::num::NonZeroU8;

    use super::{DateComplete, ParsedEdtf};
    use crate::common::{UnvalidatedTime, UnvalidatedTz};

    #[test]
    fn date() {
        assert_eq!(Date::parse("1985-04-12"), Ok(Date::from_ymd(1985, 4, 12)));
        assert_eq!(Date::parse("1985-04"), Ok(Date::from_ymd(1985, 4, 0)));
        assert_eq!(Date::parse("1985"), Ok(Date::from_ymd(1985, 0, 0)));
    }

    #[test]
    fn date_invalid() {
        use crate::ParseError;
        assert_eq!(Date::parse("1985000"), Err(ParseError::Invalid));
        assert_eq!(Date::parse("2003-02-29"), Err(ParseError::OutOfRange));
        assert_eq!(Date::parse("2003-02-28"), Ok(Date::from_ymd(2003, 2, 28)),);
        assert_eq!(Date::parse("2004-02-29"), Ok(Date::from_ymd(2004, 2, 29)),);
    }

    #[test]
    fn date_remain() {
        assert_eq!(
            super::date("1985-04-12T12345").finish(),
            Ok(("T12345", Date::from_ymd(1985, 4, 12)))
        );
        assert_eq!(
            super::date("1985-0489898989").finish(),
            Ok(("89898989", Date::from_ymd(1985, 4, 0)))
        );
        assert_eq!(
            super::date("1985000").finish(),
            Ok(("000", Date::from_ymd(1985, 0, 0)))
        );
    }

    #[test]
    fn parse_level0() {
        assert_eq!(
            super::level0("2004-02-29T01:47:05"),
            Ok((
                "",
                ParsedEdtf::DateTime(
                    DateComplete {
                        year: 2004,
                        month: NonZeroU8::new(02).unwrap(),
                        day: NonZeroU8::new(29).unwrap(),
                    },
                    UnvalidatedTime {
                        hh: 01,
                        mm: 47,
                        ss: 05,
                        tz: UnvalidatedTz::None
                    },
                )
            ))
        );

        assert_eq!(
            super::level0("2004-02-29T01:47:00Z"),
            Ok((
                "",
                ParsedEdtf::DateTime(
                    DateComplete {
                        year: 2004,
                        month: NonZeroU8::new(02).unwrap(),
                        day: NonZeroU8::new(29).unwrap(),
                    },
                    UnvalidatedTime {
                        hh: 01,
                        mm: 47,
                        ss: 00,
                        tz: UnvalidatedTz::Utc
                    },
                )
            ))
        );

        assert_eq!(
            super::level0("2004-02-29T01:47:00+00:00"),
            Ok((
                "",
                ParsedEdtf::DateTime(
                    DateComplete {
                        year: 2004,
                        month: NonZeroU8::new(02).unwrap(),
                        day: NonZeroU8::new(29).unwrap(),
                    },
                    UnvalidatedTime {
                        hh: 01,
                        mm: 47,
                        ss: 00,
                        tz: UnvalidatedTz::HoursMinutes {
                            positive: true,
                            hh: 00,
                            mm: 00
                        }
                    },
                )
            ))
        );

        assert_eq!(
            super::level0("2004-02-29T01:47:00-04:30"),
            Ok((
                "",
                ParsedEdtf::DateTime(
                    DateComplete {
                        year: 2004,
                        month: NonZeroU8::new(02).unwrap(),
                        day: NonZeroU8::new(29).unwrap(),
                    },
                    UnvalidatedTime {
                        hh: 01,
                        mm: 47,
                        ss: 00,
                        tz: UnvalidatedTz::HoursMinutes {
                            positive: false,
                            hh: 04,
                            mm: 30
                        }
                    },
                )
            ))
        );

        assert_eq!(
            super::level0("2004-02-29/2009-07-16"),
            Ok((
                "",
                ParsedEdtf::Interval(Date::from_ymd(2004, 02, 29), Date::from_ymd(2009, 07, 16),)
            ))
        );

        assert_eq!(
            super::level0("2004-02-29/2009-07"),
            Ok((
                "",
                ParsedEdtf::Interval(Date::from_ymd(2004, 02, 29), Date::from_ymd(2009, 07, 0),)
            ))
        );

        assert_eq!(
            super::level0("2004/2009-07"),
            Ok((
                "",
                ParsedEdtf::Interval(Date::from_ymd(2004, 00, 00), Date::from_ymd(2009, 07, 00),)
            ))
        );
    }
}
