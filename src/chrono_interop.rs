// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright © 2021 Corporation for Digital Scholarship

use core::convert::TryFrom;
use core::num::NonZeroU8;

use chrono::{Datelike, NaiveDate, Offset, TimeZone, Timelike};

use crate::{DateComplete, DateTime, Time, TzOffset, GetTimezone};
use crate::level1::packed::{Certainty, PackedYear, PackedU8, PackedInt};

/// This implementation maps to an EDTF timestamp without any timezone information attached.
impl GetTimezone for chrono::NaiveDate {
    fn tz_offset(&self) -> Option<TzOffset> {
        None
    }
}

/// This implementation maps to an EDTF timestamp with a `Z` on the end.
impl GetTimezone for chrono::DateTime<chrono::Utc> {
    fn tz_offset(&self) -> Option<TzOffset> {
        Some(TzOffset::Utc)
    }
}

/// This implementation maps to an EDTF timestamp with a timezone offset like `+04:00`.
impl GetTimezone for chrono::DateTime<chrono::FixedOffset> {
    fn tz_offset(&self) -> Option<TzOffset> {
        let offset = self.offset();
        Some(TzOffset::Minutes(offset.local_minus_utc() / 60))
    }
}

/// This implementation maps to an EDTF timestamp with a timezone offset like `+04:00`.
impl GetTimezone for chrono::DateTime<TzOffset> {
    fn tz_offset(&self) -> Option<TzOffset> {
        Some(*self.offset())
    }
}

impl<DT> From<DT> for DateTime
where
    DT: Datelike,
    DT: Timelike,
    DT: GetTimezone,
{
    fn from(chrono_dt: DT) -> DateTime {
        let year = chrono_dt.year();
        let month = NonZeroU8::new(chrono_dt.month() as u8).unwrap();
        let day = NonZeroU8::new(chrono_dt.day() as u8).unwrap();
        let hh = chrono_dt.hour() as u8;
        let mm = chrono_dt.minute() as u8;
        let ss = chrono_dt.second() as u8;
        let date = DateComplete { year, month, day };
        let date = date
            .validate()
            .expect("chrono::Datelike should return valid values");
        let tz = chrono_dt.tz_offset();
        let time = Time { hh, mm, ss, tz };
        DateTime { date, time }
    }
}

impl<DT> From<DT> for crate::level_0::Edtf
where
    DT: Datelike,
    DT: Timelike,
    DT: GetTimezone,
{
    fn from(chrono_dt: DT) -> crate::level_0::Edtf {
        crate::level_0::Edtf::DateTime(chrono_dt.into())
    }
}

impl<DT> From<DT> for crate::level_1::Edtf
where
    DT: Datelike,
    DT: Timelike,
    DT: GetTimezone,
{
    fn from(chrono_dt: DT) -> crate::level_1::Edtf {
        crate::level_1::Edtf::DateTime(chrono_dt.into())
    }
}

impl DateTime {
    fn with_date(&self, date: DateComplete) -> Self {
        let Self { date: _, time } = *self;
        Self { date, time }
    }
    fn with_time(&self, time: Time) -> Self {
        let Self { date, time: _ } = *self;
        Self { date, time }
    }
}

impl DateComplete {
    pub fn to_chrono(&self) -> NaiveDate {
        NaiveDate::from_ymd(self.year, self.month.get() as u32, self.day.get() as u32)
    }
}

/// Converts from [chrono::NaiveDate].
impl From<NaiveDate> for DateComplete {
    fn from(naive: NaiveDate) -> Self {
        Self {
            year: naive.year(),
            month: NonZeroU8::new(naive.month() as u8).unwrap(),
            day: NonZeroU8::new(naive.day() as u8).unwrap(),
        }
    }
}

impl crate::level_0::Date {
    /// If this date is complete, i.e. it has a month and a day, produces a [chrono::NaiveDate].
    /// Also available via an [core::convert::TryFrom] implementation on [chrono::NaiveDate].
    pub fn to_chrono(&self) -> Option<NaiveDate> {
        if let (Some(month), Some(day)) = (self.month, self.day) {
            return Some(NaiveDate::from_ymd(
                self.year,
                month.get() as u32,
                day.get() as u32,
            ));
        }
        None
    }
}

impl crate::level_1::Date {
    /// If this date is complete, i.e. it has a month and a day, produces a [chrono::NaiveDate].
    /// Also available via an [core::convert::TryFrom] implementation on [chrono::NaiveDate].
    pub fn to_chrono(&self) -> Option<NaiveDate> {
        if let (Some(month), Some(day)) = (self.month, self.day) {
            return Some(NaiveDate::from_ymd(
                self.year.unpack().0,
                month.unpack().0 as u32,
                day.unpack().0 as u32,
            ));
        }
        None
    }
}

/// Attempts conversion via [Date::to_chrono].
impl TryFrom<crate::level_1::Date> for NaiveDate {
    type Error = ();
    fn try_from(value: crate::level_1::Date) -> Result<Self, Self::Error> {
        value.to_chrono().ok_or(())
    }
}

/// Converts from [chrono::NaiveDate], into a Date with day precision, and with no uncertainty
/// flags set.
impl From<NaiveDate> for crate::level_1::Date {
    fn from(naive: NaiveDate) -> Self {
        Self {
            year: PackedYear::pack(naive.year(), Default::default()).unwrap(),
            month: PackedU8::pack(naive.month() as u8, Default::default()),
            day: PackedU8::pack(naive.day() as u8, Default::default()),
            certainty: Certainty::Certain,
        }
    }
}

/// Convenience [chrono::Datelike] implementation, which mostly relies on internal conversion to
/// [chrono::NaiveDate].
impl Datelike for DateComplete {
    fn year(&self) -> i32 {
        self.year
    }

    fn month(&self) -> u32 {
        self.month.get() as u32
    }

    fn month0(&self) -> u32 {
        self.month.get() as u32 - 1
    }

    fn day(&self) -> u32 {
        self.day.get() as u32
    }

    fn day0(&self) -> u32 {
        self.day() - 1
    }

    fn ordinal(&self) -> u32 {
        self.to_chrono().ordinal()
    }
    fn ordinal0(&self) -> u32 {
        self.to_chrono().ordinal0()
    }

    fn weekday(&self) -> chrono::Weekday {
        self.to_chrono().weekday()
    }

    fn iso_week(&self) -> chrono::IsoWeek {
        self.to_chrono().iso_week()
    }

    fn with_year(&self, year: i32) -> Option<Self> {
        self.to_chrono().with_year(year).map(Self::from)
    }

    fn with_month(&self, month: u32) -> Option<Self> {
        self.to_chrono().with_month(month).map(Self::from)
    }

    fn with_month0(&self, month0: u32) -> Option<Self> {
        self.to_chrono().with_month0(month0).map(Self::from)
    }

    fn with_day(&self, day: u32) -> Option<Self> {
        self.to_chrono().with_day(day).map(Self::from)
    }

    fn with_day0(&self, day0: u32) -> Option<Self> {
        self.to_chrono().with_day0(day0).map(Self::from)
    }

    fn with_ordinal(&self, ordinal: u32) -> Option<Self> {
        self.to_chrono().with_ordinal(ordinal).map(Self::from)
    }

    fn with_ordinal0(&self, ordinal0: u32) -> Option<Self> {
        self.to_chrono().with_ordinal0(ordinal0).map(Self::from)
    }
}

/// Convenience [chrono::Datelike] implementation, which mostly relies on internal conversion to
/// [chrono::NaiveDate].
impl Datelike for DateTime {
    fn year(&self) -> i32 {
        self.date.year()
    }

    fn month(&self) -> u32 {
        self.date.month()
    }

    fn month0(&self) -> u32 {
        self.date.month0()
    }

    fn day(&self) -> u32 {
        self.date.day()
    }

    fn day0(&self) -> u32 {
        self.date.day0()
    }

    fn ordinal(&self) -> u32 {
        self.date.ordinal()
    }

    fn ordinal0(&self) -> u32 {
        self.date.ordinal0()
    }

    fn weekday(&self) -> chrono::Weekday {
        self.date.weekday()
    }

    fn iso_week(&self) -> chrono::IsoWeek {
        self.date.iso_week()
    }

    fn with_year(&self, year: i32) -> Option<Self> {
        self.date.with_year(year).map(|date| self.with_date(date))
    }

    fn with_month(&self, month: u32) -> Option<Self> {
        self.date.with_month(month).map(|date| self.with_date(date))
    }

    fn with_month0(&self, month0: u32) -> Option<Self> {
        self.date
            .with_month0(month0)
            .map(|date| self.with_date(date))
    }

    fn with_day(&self, day: u32) -> Option<Self> {
        self.date.with_day(day).map(|date| self.with_date(date))
    }

    fn with_day0(&self, day0: u32) -> Option<Self> {
        self.date.with_day0(day0).map(|date| self.with_date(date))
    }

    fn with_ordinal(&self, ordinal: u32) -> Option<Self> {
        self.date
            .with_ordinal(ordinal)
            .map(|date| self.with_date(date))
    }

    fn with_ordinal0(&self, ordinal0: u32) -> Option<Self> {
        self.date
            .with_ordinal0(ordinal0)
            .map(|date| self.with_date(date))
    }
}

impl Timelike for Time {
    fn hour(&self) -> u32 {
        self.hh as u32
    }

    fn minute(&self) -> u32 {
        self.mm as u32
    }

    fn second(&self) -> u32 {
        self.ss as u32
    }

    fn nanosecond(&self) -> u32 {
        0
    }

    fn with_hour(&self, hour: u32) -> Option<Self> {
        if hour > 23 {
            return None;
        }
        Some(Self {
            hh: hour as u8,
            ..*self
        })
    }

    fn with_minute(&self, min: u32) -> Option<Self> {
        if min > 59 {
            return None;
        }
        Some(Self {
            mm: min as u8,
            ..*self
        })
    }

    fn with_second(&self, sec: u32) -> Option<Self> {
        if sec > 60 {
            return None;
        }
        if sec == 60 && !(self.hh == 23 && self.mm == 59) {
            return None;
        }
        Some(Self {
            ss: sec as u8,
            ..*self
        })
    }

    fn with_nanosecond(&self, _nano: u32) -> Option<Self> {
        Some(*self)
    }
}

impl Timelike for DateTime {
    fn hour(&self) -> u32 {
        self.time.hour()
    }

    fn minute(&self) -> u32 {
        self.time.minute()
    }

    fn second(&self) -> u32 {
        self.time.second()
    }

    fn nanosecond(&self) -> u32 {
        self.time.nanosecond()
    }

    fn with_hour(&self, hour: u32) -> Option<Self> {
        self.time.with_hour(hour).map(|t| self.with_time(t))
    }

    fn with_minute(&self, min: u32) -> Option<Self> {
        self.time.with_minute(min).map(|t| self.with_time(t))
    }

    fn with_second(&self, sec: u32) -> Option<Self> {
        self.time.with_second(sec).map(|t| self.with_time(t))
    }

    fn with_nanosecond(&self, _nano: u32) -> Option<Self> {
        Some(*self)
    }
}

impl Offset for TzOffset {
    fn fix(&self) -> chrono::FixedOffset {
        match *self {
            TzOffset::Utc => chrono::FixedOffset::east(0),
            TzOffset::Hours(h) => chrono::FixedOffset::east(h * 3600),
            TzOffset::Minutes(min) => chrono::FixedOffset::east(min * 60),
        }
    }
}

impl TimeZone for TzOffset {
    type Offset = Self;

    fn from_offset(offset: &Self::Offset) -> Self {
        *offset
    }

    fn offset_from_local_date(&self, _local: &NaiveDate) -> chrono::LocalResult<Self::Offset> {
        chrono::LocalResult::Single(*self)
    }

    fn offset_from_local_datetime(
        &self,
        _local: &chrono::NaiveDateTime,
    ) -> chrono::LocalResult<Self::Offset> {
        chrono::LocalResult::Single(*self)
    }

    fn offset_from_utc_date(&self, _utc: &NaiveDate) -> Self::Offset {
        *self
    }

    fn offset_from_utc_datetime(&self, _utc: &chrono::NaiveDateTime) -> Self::Offset {
        *self
    }
}

#[test]
fn timezone_impl() {
    let off = TzOffset::Hours(4);
    let ch = off.ymd(2019, 8, 7).and_hms(19, 7, 56);
    let edtf = crate::level_1::Edtf::from(ch).as_datetime();
    assert_eq!(
        edtf,
        Some(DateTime {
            date: DateComplete::from_ymd(2019, 8, 7),
            time: Time {
                hh: 19,
                mm: 7,
                ss: 56,
                tz: Some(TzOffset::Hours(4))
            }
        })
    );
}

#[cfg(test)]
mod test {
    #[test]
    fn to_chrono() {
        use crate::level_1::Edtf;
        use chrono::TimeZone;
        let utc = chrono::Utc;
        assert_eq!(
            Edtf::parse("2004-02-29T01:47:00+00:00")
                .unwrap()
                .as_datetime()
                .unwrap()
                .to_chrono(&utc),
            utc.ymd(2004, 02, 29).and_hms(01, 47, 00)
        );
        assert_eq!(
            Edtf::parse("2004-02-29T01:47:00")
                .unwrap()
                .as_datetime()
                .unwrap()
                .to_chrono(&utc),
            utc.ymd(2004, 02, 29).and_hms(01, 47, 00)
        );
    }
}
