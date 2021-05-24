use chrono::{Datelike, Timelike};

use crate::common::{DateComplete, TzOffset, Time};
use super::*;

/// This implementation maps to an EDTF timestamp without any timezone information attached.
impl GetTimezone for chrono::NaiveDate {
    fn utc_offset_sec(&self) -> Option<i32> {
        None
    }
}

/// This implementation maps to an EDTF timestamp with a `Z` on the end.
impl GetTimezone for chrono::DateTime<chrono::Utc> {
    fn utc_offset_sec(&self) -> Option<i32> {
        Some(0)
    }
}

/// This implementation maps to an EDTF timestamp with a timezone offset like `+04:00`.
impl GetTimezone for chrono::DateTime<chrono::FixedOffset> {
    fn utc_offset_sec(&self) -> Option<i32> {
        let offset = self.offset();
        Some(offset.local_minus_utc())
    }
}

fn get_timezone(getter: &impl GetTimezone) -> Option<TzOffset> {
    let sec = getter.utc_offset_sec();
    if sec == Some(0) {
        Some(TzOffset::Utc)
    } else {
        sec.map(TzOffset::Offset)
    }
}

impl<DT> From<DT> for DateTime
where
    DT: Datelike,
    DT: Timelike,
    DT: GetTimezone,
{
    fn from(chrono_dt: DT) -> DateTime {
        use core::num::NonZeroU8;
        let year = chrono_dt.year();
        let month = NonZeroU8::new(chrono_dt.month() as u8).unwrap();
        let day = NonZeroU8::new(chrono_dt.day() as u8).unwrap();
        let hh = chrono_dt.hour() as u8;
        let mm = chrono_dt.minute() as u8;
        let ss = chrono_dt.second() as u8;
        let date = DateComplete { year, month, day };
        let date = date.validate().expect("chrono::Datelike should return valid values");
        let tz = get_timezone(&chrono_dt);
        let time = Time { hh, mm, ss, tz };
        DateTime { date, time }
    }
}

impl<DT> From<DT> for Edtf
where
    DT: Datelike,
    DT: Timelike,
    DT: GetTimezone,
{
    fn from(chrono_dt: DT) -> Edtf {
        Edtf::DateTime(chrono_dt.into())
    }
}
