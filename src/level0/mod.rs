// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright © 2021 Corporation for Digital Scholarship

use crate::{common::is_valid_complete_date, ParseError};

use core::num::NonZeroU8;

mod parser;
use crate::common::{UnvalidatedTime, UnvalidatedTz};
use crate::{DateComplete, DateTime, Time, TzOffset};
use parser::ParsedEdtf;

pub mod api;
use api::*;

impl Edtf {
    fn validate(parsed: ParsedEdtf) -> Result<Self, ParseError> {
        let edtf = match parsed {
            ParsedEdtf::Date(d) => Edtf::Date(d.validate()?),
            ParsedEdtf::Interval(d, d2) => Edtf::Interval(d.validate()?, d2.validate()?),
            ParsedEdtf::DateTime(d, t) => Edtf::DateTime(DateTime::validate(d, t)?),
        };
        Ok(edtf)
    }
}

impl DateComplete {
    pub(crate) fn validate(self) -> Result<Self, ParseError> {
        let Self { year, month, day } = self;
        let v = is_valid_complete_date(year, month.get(), day.get())?;
        Ok(v)
    }
}

impl DateTime {
    pub(crate) fn validate(date: DateComplete, time: UnvalidatedTime) -> Result<Self, ParseError> {
        let date = date.validate()?;
        let time = time.validate()?;
        Ok(DateTime { date, time })
    }
}

impl UnvalidatedTz {
    fn validate(self) -> Result<TzOffset, ParseError> {
        match self {
            Self::Unspecified => Ok(TzOffset::Unspecified),
            Self::Utc => Ok(TzOffset::Utc),
            Self::Hours { positive, hh } => {
                let sign = if positive { 1 } else { -1 };
                if hh > 23 {
                    return Err(ParseError::OutOfRange);
                }
                Ok(TzOffset::Hours(sign * hh as i32))
            }
            Self::HoursMinutes { positive, hh, mm } => {
                // apparently iso8601-1 doesn't specify a limit on the number of hours offset you can be.
                // but we will stay sane and cap things at 23:59, because at >= 24h offset you need to
                // change the date.
                // We will however validate the minutes.
                if hh > 23 || mm > 59 {
                    return Err(ParseError::OutOfRange);
                }
                let sign = if positive { 1 } else { -1 };
                let mins = 60 * hh as i32 + mm as i32;
                Ok(TzOffset::Minutes(sign * mins))
            }
        }
    }
}

impl UnvalidatedTime {
    pub(crate) fn validate(self) -> Result<Time, ParseError> {
        let Self { hh, mm, ss, tz } = self;
        let tz = tz.validate()?;
        // - ISO 8601 only allows 24 as an 'end of day' or such like when used in an interval (e.g.
        //   two /-separated timestamps.) EDTF doesn't allow intervals with time of day. So hours
        //   can't be 24.
        // - Minutes can never be 60+.
        // - Seconds can top out at 58, 59 or 60 depending on whether that day adds or subtracts a
        //   leap second. But we don't know in advance and we're not an NTP server so the best we
        //   can do is check that any ss=60 leap second occurs only on a 23:59 base.
        if hh > 23 || mm > 59 || ss > 60 {
            return Err(ParseError::OutOfRange);
        }
        if ss == 60 && !(hh == 23 && mm == 59) {
            return Err(ParseError::OutOfRange);
        }
        Ok(Time { hh, mm, ss, tz })
    }
}

impl Date {
    fn new_unvalidated(year: i32, month: Option<NonZeroU8>, day: Option<NonZeroU8>) -> Self {
        Date { year, month, day }
    }
    fn validate(self) -> Result<Self, ParseError> {
        if self.year > 9999 || self.year < 0 {
            return Err(ParseError::OutOfRange);
        }
        if let Some(m) = self.month.map(NonZeroU8::get) {
            if let Some(d) = self.day.map(NonZeroU8::get) {
                let _complete = is_valid_complete_date(self.year, m, d)?;
            } else {
                if m > 12 {
                    return Err(ParseError::OutOfRange);
                }
            }
        } else {
            if self.day.is_some() {
                // Both the parser and from_ymd can accept 0 for month and nonzero for day.
                return Err(ParseError::OutOfRange);
            }
            // otherwise, both Null.
        }
        Ok(self)
    }
}

#[cfg(feature = "chrono")]
fn fixed_offset_from(positive: bool, hh: u8, mm: u8) -> Option<chrono::FixedOffset> {
    let secs = 3600 * hh as i32 + 60 * mm as i32;
    if positive {
        chrono::FixedOffset::east_opt(secs)
    } else {
        chrono::FixedOffset::west_opt(secs)
    }
}
