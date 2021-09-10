// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright © 2021 Corporation for Digital Scholarship

use super::{
    packed::{DMFlags, DMMask, PackedInt, PackedU8},
    *,
};
use crate::common::{MONTH_DAYCOUNT, MONTH_DAYCOUNT_LEAP};

fn days_in_month(y: i32, m: u8) -> u8 {
    let leap = crate::common::is_leap_year(y);
    let lut = if leap {
        MONTH_DAYCOUNT_LEAP
    } else {
        MONTH_DAYCOUNT
    };
    lut[m as usize - 1]
}

impl Date {
    fn iter_start(&self, min_level: IterLevel) -> Option<Self> {
        self.trunc_for_iter(1, |_, _| 1, 1, min_level)
    }
    fn iter_end(&self, min_level: IterLevel) -> Option<Self> {
        self.trunc_for_iter(12, days_in_month, 31, min_level)
    }
    fn trunc_for_iter(
        &self,
        cap_month: u8,
        mday: fn(i32, u8) -> u8,
        cap_day: u8,
        min_level: IterLevel,
    ) -> Option<Self> {
        let (y, _) = self.year.unpack();
        let mut new = self.clone();
        if let Some(month) = self.month {
            let (m, mflags) = month.unpack();
            match self.day {
                Some(day) if min_level <= IterLevel::Day => {
                    let (_, dflags) = day.unpack();
                    match (mflags.mask, dflags.mask) {
                        (DMMask::None, DMMask::Unspecified) => {
                            new.day = PackedU8::pack(
                                mday(y, m),
                                DMFlags::new(dflags.certainty, DMMask::None),
                            )
                        }
                        (DMMask::Unspecified, DMMask::Unspecified) => {
                            new.month = PackedU8::pack(
                                cap_month,
                                DMFlags::new(mflags.certainty, DMMask::None),
                            );
                            new.day = PackedU8::pack(
                                cap_day,
                                DMFlags::new(mflags.certainty, DMMask::None),
                            );
                        }
                        _ => {}
                    }
                }
                None if min_level <= IterLevel::Month => {
                    if mflags.mask == DMMask::Unspecified {
                        new.month =
                            PackedU8::pack(cap_month, DMFlags::new(mflags.certainty, DMMask::None));
                    }
                }
                // min_level is too high for the date that we have
                // this means e.g. 2021-08-08.iter_possible_months returns None
                _ => return None,
            }
        } else if min_level <= IterLevel::Month {
            return None;
        }
        return Some(new);
    }

    // if these are the same day, you just get an iterator with a single Some(self) output.
    // if e.g. 2021-05-XX, it iterates through all the days in May 2021. 2021-XX-XX iterates
    // through 365 days.
    pub fn iter_possible_days(&self) -> Option<YearMonthDayIter> {
        let start = self.iter_start(IterLevel::Day)?;
        let end = self.iter_end(IterLevel::Day)?;
        Edtf::Interval(start, end).iter_days()
    }

    pub fn iter_forward_days(&self) -> Option<YearMonthDayIter> {
        let d = self.iter_start(IterLevel::Day)?.complete()?;
        let ymd = (d.year(), d.month(), d.day());
        Some(YearMonthDayIter(IncrementIter::raw(Some(ymd), None)))
    }

    pub fn iter_possible_months(&self) -> Option<YearMonthIter> {
        let start = self.iter_start(IterLevel::Month)?;
        let end = self.iter_end(IterLevel::Month)?;
        Edtf::Interval(start, end).iter_months()
    }

    pub fn iter_forward_months(&self) -> Option<YearMonthIter> {
        let start = match self.iter_start(IterLevel::Month)?.precision() {
            Precision::Month(y, m) => (y, m),
            _ => return None,
        };
        Some(YearMonthIter(IncrementIter::raw(Some(start), None)))
    }
}

#[cfg(test)]
macro_rules! date_iter_test {
    ($d:literal::$ident:ident() $( .take($take:literal) )?, vec![ $($tt:tt)*]) => {
        date_iter_test!($d::$ident()$(.take($take))?, Some(vec![$($tt)*]))
    };
    ($d:literal::$ident:ident().take($take:literal), $vec:expr) => {
        assert_eq!(
            Date::parse($d)
                .unwrap()
                .$ident()
                .map(|x| x.take($take).collect_with(Vec::new())),
            $vec
        )
    };
    ($d:literal::$ident:ident(), $vec:expr) => {
        assert_eq!(
            Date::parse($d)
                .unwrap()
                .$ident()
                // prevent unbounded recurson
                .map(|x| x.take(1000).collect_with(Vec::new())),
            $vec
        )
    };
}

#[test]
fn iter_possible_days() {
    date_iter_test!("2020-08-08"::iter_possible_days(), vec![DateComplete::from_ymd(2020, 8, 8)]);
    let date = Date::parse("2020-08-XX").unwrap();
    let days = date.iter_possible_days().unwrap().collect_with(Vec::new());
    assert_eq!(days.len(), 31);
    assert_eq!(
        days,
        Edtf::parse("2020-08-01/2020-08-31")
            .unwrap()
            .iter_days()
            .unwrap()
            .collect_with(Vec::new())
    );
}

#[test]
fn iter_possible_days_of_year() {
    let date = Date::parse("2020-XX-XX").unwrap();
    let count = date.iter_possible_days().unwrap().count();
    assert_eq!(count, 366);
    date_iter_test!("2020-XX-XX"::iter_possible_days().take(5), vec![
        DateComplete::from_ymd(2020, 1, 1),
        DateComplete::from_ymd(2020, 1, 2),
        DateComplete::from_ymd(2020, 1, 3),
        DateComplete::from_ymd(2020, 1, 4),
        DateComplete::from_ymd(2020, 1, 5),
    ]);
}

#[test]
fn iter_possible_days_of_month() {
    let date = Date::parse("2020-06-XX").unwrap();
    let count = date.iter_possible_days().unwrap().count();
    assert_eq!(count, 30);
    date_iter_test!("2020-05-XX"::iter_possible_days().take(5), vec![
        DateComplete::from_ymd(2020, 5, 1),
        DateComplete::from_ymd(2020, 5, 2),
        DateComplete::from_ymd(2020, 5, 3),
        DateComplete::from_ymd(2020, 5, 4),
        DateComplete::from_ymd(2020, 5, 5),
    ]);
}

#[test]
fn iter_possible_days_nounspec() {
    date_iter_test!("2020"::iter_possible_days(), None);
    date_iter_test!("202X"::iter_possible_days(), None);
    date_iter_test!("20XX"::iter_possible_days(), None);
}

#[test]
fn iter_forward_days() {
    date_iter_test!("2020-08-08"::iter_forward_days().take(5), vec![
        DateComplete::from_ymd(2020, 8, 8),
        DateComplete::from_ymd(2020, 8, 9),
        DateComplete::from_ymd(2020, 8, 10),
        DateComplete::from_ymd(2020, 8, 11),
        DateComplete::from_ymd(2020, 8, 12),
    ]);
}

#[test]
fn iter_possible_months() {
    date_iter_test!("20XX"::iter_possible_months(), None);
    date_iter_test!("202X"::iter_possible_months(), None);
    date_iter_test!("2020"::iter_possible_months(), None);
    date_iter_test!("2020-08-XX"::iter_possible_months(), None);
    date_iter_test!("2020-08-09"::iter_possible_months(), None);
    date_iter_test!("2020-09"::iter_possible_months(), vec![(2020, 09)]);
    date_iter_test!("2020-XX"::iter_possible_months().take(5), vec![
         (2020, 01), (2020, 02), (2020, 03), (2020, 04), (2020, 05)
    ]);
}

#[test]
fn iter_forward_months() {
    date_iter_test!("2020-05"::iter_forward_months().take(3), Some(vec![(2020, 05), (2020, 06), (2020, 07)]));
    date_iter_test!("2020-XX"::iter_forward_months().take(3), Some(vec![(2020, 01), (2020, 02), (2020, 03)]));
}

