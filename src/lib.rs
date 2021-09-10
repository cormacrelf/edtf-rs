// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright © 2021 Corporation for Digital Scholarship

#![allow(dead_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

// use `bacon docs-rs` to review missing documentation
#![cfg_attr(docsrs, warn(missing_docs))]

#[cfg(all(doc, feature = "chrono"))]
use chrono::NaiveDate;

pub(crate) mod common;
pub(crate) mod helpers;
mod level0;
#[allow(missing_docs)]
mod level2;
pub mod level_1;
use common::{UnvalidatedTime, UnvalidatedTz};
pub use level0::api as level_0;
#[doc(hidden)]
pub use level2::api as level_2;

#[cfg(feature = "chrono")]
#[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
mod chrono_interop;

use core::convert::TryInto;
use core::num::NonZeroU8;

/// The error type for all the `parse` methods in this crate.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseError {
    /// A field is out of the permitted range.
    OutOfRange,

    /// The input string has some invalid character sequence.
    Invalid,
}

impl std::error::Error for ParseError {}

use core::fmt;
impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[allow(rustdoc::broken_intra_doc_links)]
/// A DateTime object.
///
/// This has minimal introspection methods. It is not an attempt to build a complete DateTime API.
/// Prefer to use its implementation of [chrono::Datelike] and [chrono::Timelike] or simply the
/// [DateTime::to_chrono] method to use a specific [chrono::TimeZone], all available with `features
/// = ["chrono"]`.
///
/// Also, its Display implementation is geared towards lossless EDTF parse-format roundtrips. It
/// does not always produce valid RFC3339 timestamps, in particular [TzOffset::Hours] is rendered
/// as `+04` instead of `+04:00`. This is best considered a problem with the EDTF specification for
/// allowing a useless extra timestamp format.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct DateTime {
    pub(crate) date: DateComplete,
    pub(crate) time: Time,
}

#[cfg(feature = "chrono")]
fn chrono_tz_datetime<Tz: chrono::TimeZone>(
    tz: &Tz,
    date: &DateComplete,
    time: &Time,
) -> chrono::DateTime<Tz> {
    tz.ymd(date.year, date.month.get() as u32, date.day.get() as u32)
        .and_hms(time.hh as u32, time.mm as u32, time.ss as u32)
}

impl DateTime {
    /// Gets the date portion
    pub fn date(&self) -> DateComplete {
        self.date
    }
    /// Gets the time portion
    pub fn time(&self) -> Time {
        self.time
    }

    #[cfg_attr(not(feature = "chrono"), allow(rustdoc::broken_intra_doc_links))]
    /// Get the `TzOffset`. If `None` is returned, this represents a timestamp which did not
    /// specify a timezone.
    ///
    /// If using the `chrono` interop, None means you should attempt to convert to a [chrono::NaiveDate]
    pub fn offset(&self) -> TzOffset {
        self.time.offset()
    }

    #[cfg(feature = "chrono")]
    #[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
    #[cfg_attr(not(feature = "chrono"), allow(rustdoc::broken_intra_doc_links))]
    /// Convert to a [chrono::NaiveDate]
    pub fn to_chrono_naive(&self) -> chrono::NaiveDateTime {
        let date = self.date.to_chrono();
        let time = self.time.to_chrono_naive();
        date.and_time(time)
    }

    /// ```
    /// use edtf::level_1::Edtf;
    /// use chrono::TimeZone;
    ///
    /// let utc = chrono::Utc;
    /// assert_eq!(
    ///     Edtf::parse("2004-02-29T01:47:00+05:00")
    ///         .unwrap()
    ///         .as_datetime()
    ///         .unwrap()
    ///         .to_chrono(&utc),
    ///     utc.ymd(2004, 02, 28).and_hms(20, 47, 00)
    /// );
    /// ```
    #[cfg(feature = "chrono")]
    #[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
    pub fn to_chrono<Tz>(&self, tz: &Tz) -> chrono::DateTime<Tz>
    where
        Tz: chrono::TimeZone,
    {
        let DateTime { date, time } = self;
        match time.tz {
            None => chrono_tz_datetime(tz, date, time),
            Some(TzOffset::Utc) => {
                let utc = chrono_tz_datetime(&chrono::Utc, date, time);
                utc.with_timezone(tz)
            }
            Some(TzOffset::Hours(hours)) => {
                let fixed_zone = chrono::FixedOffset::east_opt(hours * 3600)
                    .expect("time zone offset out of bounds");
                let fixed_dt = chrono_tz_datetime(&fixed_zone, date, time);
                fixed_dt.with_timezone(tz)
            }
            Some(TzOffset::Minutes(signed_min)) => {
                let fixed_zone = chrono::FixedOffset::east_opt(signed_min * 60)
                    .expect("time zone offset out of bounds");
                let fixed_dt = chrono_tz_datetime(&fixed_zone, date, time);
                fixed_dt.with_timezone(tz)
            }
        }
    }
}

/// A structure to hold the date portion of a [DateTime]. It contains a valid date in the proleptic
/// Gregorian calendar.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct DateComplete {
    pub(crate) year: i32,
    pub(crate) month: NonZeroU8,
    pub(crate) day: NonZeroU8,
}

impl fmt::Debug for DateComplete {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Self as fmt::Display>::fmt(self, f)
    }
}
impl fmt::Display for DateComplete {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let DateComplete { year, month, day } = *self;
        let sign = helpers::sign_str_if_neg(year);
        let year = year.abs();
        write!(f, "{}{:04}", sign, year)?;
        write!(f, "-{:02}", month)?;
        write!(f, "-{:02}", day)?;
        Ok(())
    }
}

impl DateComplete {
    /// Create a complete date. Panics if the date is invalid.
    pub fn from_ymd(year: i32, month: u32, day: u32) -> Self {
        Self::from_ymd_opt(year, month, day).expect("invalid complete date")
    }
    /// Create a complete date. Returns None if the date is invalid. The only way a date can be
    /// invalid is if it isn't a real date. There are otherwise no limitations on the range of
    /// acceptable years.
    pub fn from_ymd_opt(year: i32, month: u32, day: u32) -> Option<Self> {
        let month = month.try_into().ok().map(NonZeroU8::new)??;
        let day = day.try_into().ok().map(NonZeroU8::new)??;
        Self { year, month, day }.validate().ok()
    }

    /// Gets the year
    pub fn year(&self) -> i32 {
        self.year
    }
    /// Gets the month
    pub fn month(&self) -> u32 {
        self.month.get() as u32
    }
    /// Gets the day
    pub fn day(&self) -> u32 {
        self.day.get() as u32
    }
}

/// The time portion of a [DateTime].
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Time {
    pub(crate) hh: u8,
    pub(crate) mm: u8,
    pub(crate) ss: u8,
    pub(crate) tz: TzOffset,
}

impl Time {
    /// Create a time. Panics if it's invalid.
    pub fn from_hmsz(hh: u32, mm: u32, ss: u32, tz: TzOffset) -> Self {
        Self::from_hmsz_opt(hh, mm, ss, tz).expect("out of range in Time::from_hmsz")
    }
    /// Create a time. Returns None if it's invalid for any reason.
    pub fn from_hmsz_opt(hh: u32, mm: u32, ss: u32, tz: TzOffset) -> Option<Self> {
        let unval = UnvalidatedTime {
            hh: hh.try_into().ok()?,
            mm: mm.try_into().ok()?,
            ss: ss.try_into().ok()?,
            tz: UnvalidatedTz::Unspecified,
        };
        let mut time = unval.validate().ok()?;
        let tz = match tz {
            TzOffset::Unspecified => tz,
            TzOffset::Hours(x) if x.abs() < 24 => tz,
            TzOffset::Minutes(x) if x.abs() < 24 * 60 => tz,
            TzOffset::Utc => tz,
            _ => return None,
        };
        time.tz = tz;
        Some(time)
    }
    /// 0..=23
    pub fn hour(&self) -> u32 {
        self.hh as u32
    }
    /// 0..=59
    pub fn minute(&self) -> u32 {
        self.mm as u32
    }
    /// 0..=60 (because of leap seconds; only if hour==23 and minute=59)
    pub fn second(&self) -> u32 {
        self.ss as u32
    }
    #[cfg_attr(not(feature = "chrono"), allow(rustdoc::broken_intra_doc_links))]
    /// Get the `TzOffset`. If `None` is returned, this represents a timestamp which did not
    /// specify a timezone.
    ///
    /// If using the `chrono` interop, None means you should attempt to convert to a [chrono::NaiveDate]
    pub fn offset(&self) -> TzOffset {
        self.tz
    }
    #[cfg(feature = "chrono")]
    #[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
    /// Strips out the timezone and returns a [chrono::NaiveTime].
    pub fn to_chrono_naive(&self) -> chrono::NaiveTime {
        chrono::NaiveTime::from_hms(self.hour(), self.minute(), self.second())
    }
}

#[cfg_attr(not(feature = "chrono"), allow(rustdoc::broken_intra_doc_links))]
/// A parsed EDTF timezone.
///
/// If `features = ["chrono"]` is enabled, then this can act as a [chrono::TimeZone]. This can be
/// used to preserve the level of TZ brevity i.e. `TzOffset::Hours(_)` ends up as `+04` instead of
/// `+04:00`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum TzOffset {
    /// An EDTF with no timezone information at all.
    Unspecified,
    /// `Z`
    Utc,
    /// `+04`
    /// A number of hours only. Not RFC3339-compliant.
    ///
    /// In order to provide lossless parse-format roundtrips,  this will be formatted without the
    /// `:00`, so if you want timestamps to be RFC3339-compliant, do not use this. Because of this,
    /// you may wish to use the `chrono` interop to format an RFC3339 timestamp instead of the
    /// Display implementation on [DateTime].
    Hours(i32),
    /// `+04:30`
    /// A number of minutes offset from UTC.
    Minutes(i32),
}

#[cfg_attr(not(feature = "chrono"), allow(rustdoc::broken_intra_doc_links))]
/// A helper trait for getting timezone information from some value. (Especially [chrono::DateTime]
/// or [chrono::NaiveDateTime].)
///
/// Implementations for the `chrono` types are included with `feature = ["chrono"]`.
///
/// Not implemented on [DateTime] because this is only used as a bound on `impl<T> From<T> for DateTime`
/// implementations.
pub trait GetTimezone {
    /// Return the number of seconds difference from UTC.
    ///
    /// - `TzOffset::None` represents NO timezone information on the EDTF timestamp.
    /// - `TzOffset::Utc` represents a `Z` timezone, i.e. UTC/Zulu time.
    /// - `TzOffset::Hours(1)` represents `+01
    /// - `TzOffset::Minutes(-16_200)` represents `-04:30`
    fn tz_offset(&self) -> TzOffset;
}
