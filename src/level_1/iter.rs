// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright © 2021 Corporation for Digital Scholarship

mod incrementable;
use crate::common::{MONTH_DAYCOUNT, MONTH_DAYCOUNT_LEAP};

use super::{
    packed::{DMFlags, DMMask, PackedInt, PackedU8},
    *,
};
use core::ops::RangeInclusive;
use incrementable::*;

#[derive(Debug, Clone, PartialEq, Eq)]
struct IncrementIter<I>
where
    I: Incrementable,
{
    from: Option<I::Storage>,
    to: Option<I::Storage>,
}

impl<I: Incrementable> IncrementIter<I> {
    pub fn raw(from: Option<I::Input>, to: Option<I::Input>) -> Self {
        Self {
            from: from.map(I::lift),
            to: to.map(I::lift),
        }
    }
    pub fn new(from: I::Input, to: I::Input) -> Self {
        Self {
            from: Some(I::lift(from)),
            to: Some(I::lift(to)),
        }
    }
}

impl<I: Incrementable> Iterator for IncrementIter<I> {
    type Item = I::Output;

    fn next(&mut self) -> Option<Self::Item> {
        let from = self.from?;
        let next = I::output(from)?;
        self.from = I::increment(from).filter(|&new_f| self.to.map_or(true, |t| new_f <= t));
        if self.from.is_none() {
            self.to = None;
        }
        Some(next)
    }
}

impl<D: Decrementable> DoubleEndedIterator for IncrementIter<D> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let to = self.to?;
        let next = D::output(to)?;
        self.to = D::decrement(to).filter(|&new_to| self.from.map_or(true, |f| f <= new_to));
        if self.to.is_none() {
            self.from = None;
        }
        Some(next)
    }
}

macro_rules! impl_iter_inner {
    ($(#[$attr:meta])* $vis:vis struct $name:ident($iterable:ty, type Item = $item:ty; );) => {
        $(#[$attr])*
         #[derive(Debug, Clone, PartialEq, Eq)]
        $vis struct $name($iterable);
        impl Iterator for $name {
            type Item = $item;
            fn next(&mut self) -> Option<Self::Item> {
                self.0.next()
            }
        }

        impl DoubleEndedIterator for $name {
            fn next_back(&mut self) -> Option<Self::Item> {
                self.0.next_back()
            }
        }
    };
}

impl_iter_inner! {
    /// Iterate all centuries that have any part of them included in the date range. See
    /// [Edtf::iter_centuries]
    pub struct CenturyIter(IncrementIter<Century>, type Item = i32; );
}
impl_iter_inner! {
    /// Iterate all decades that have any part of them included in the date range. See
    /// [Edtf::iter_decades]
    pub struct DecadeIter(IncrementIter<Decade>, type Item = i32; );
}
impl_iter_inner! {
    /// Iterate all years that have any part of them included in the date range.
    /// See [Edtf::iter_years]
    pub struct YearIter(IncrementIter<Year>, type Item = i32; );
}
impl_iter_inner! {
    /// Iterate all year-months that have any part of them included in the date range.
    /// See [Edtf::iter_months]
    ///
    /// For example, `2019-11-30/2020-01` produces `[2019-11, 2019-12, 2020-01]`.
    pub struct YearMonthIter(IncrementIter<YearMonth>, type Item = (i32, u32); );
}
impl_iter_inner! {
    /// Iterate all days in the range.
    /// See [Edtf::iter_days]
    pub struct YearMonthDayIter(IncrementIter<YearMonthDay>, type Item = DateComplete; );
}

impl From<RangeInclusive<i32>> for CenturyIter {
    fn from(range: RangeInclusive<i32>) -> Self {
        let start = *range.start();
        let end = *range.end();
        let from = start - num_integer::mod_floor(start, 100);
        let to = end - num_integer::mod_floor(end, 100);
        CenturyIter(IncrementIter::new(from, to))
    }
}

impl CenturyIter {
    /// Create using a range, like `1905..=2005 => [1900, 2000]`
    pub fn new(range: RangeInclusive<i32>) -> Self {
        range.into()
    }
}

impl From<RangeInclusive<i32>> for DecadeIter {
    fn from(range: RangeInclusive<i32>) -> Self {
        let from = *range.start() - num_integer::mod_floor(*range.start(), 10);
        let to = *range.end() - num_integer::mod_floor(*range.end(), 10);
        DecadeIter(IncrementIter::new(from, to))
    }
}

impl DecadeIter {
    /// Create using a range, like `1905..=1939 => [1900, 1910, 1920, 1930]`
    pub fn new(range: RangeInclusive<i32>) -> Self {
        range.into()
    }
}

impl From<RangeInclusive<i32>> for YearIter {
    fn from(range: RangeInclusive<i32>) -> Self {
        let from = *range.start();
        let to = *range.end();
        YearIter(IncrementIter::new(from, to))
    }
}

impl YearIter {
    /// Create using a range, like `1905..=1939 => [1900, 1910, 1920, 1930]`
    pub fn new(range: RangeInclusive<i32>) -> Self {
        range.into()
    }
}

impl From<RangeInclusive<(i32, u32)>> for YearMonthIter {
    fn from(range: RangeInclusive<(i32, u32)>) -> Self {
        let from = *range.start();
        let to = *range.end();
        YearMonthIter(IncrementIter::new(from, to))
    }
}

impl YearMonthIter {
    pub fn new(range: RangeInclusive<(i32, u32)>) -> Self {
        range.into()
    }
}

impl From<RangeInclusive<(i32, u32, u32)>> for YearMonthDayIter {
    fn from(range: RangeInclusive<(i32, u32, u32)>) -> Self {
        let from = *range.start();
        let to = *range.end();
        YearMonthDayIter(IncrementIter::new(from, to))
    }
}

impl YearMonthDayIter {
    pub fn new(range: RangeInclusive<(i32, u32, u32)>) -> Self {
        range.into()
    }
}

#[test]
fn test_century_iter() {
    macro_rules! test_century {
        ($from:literal..=$to:literal, $expected:expr) => {
            let century = CenturyIter::new($from..=$to);
            let centuries: Vec<_> = century.collect();
            assert_eq!(centuries, $expected);
        };
    }

    // we want to iterate all centuries that have any part of them included in the range

    test_century!(1905..=2000, vec![1900, 2000]);
    test_century!(1899..=2000, vec![1800, 1900, 2000]);
    test_century!(1905..=2005, vec![1900, 2000]);
    test_century!(1905..=1906, vec![1900]);

    // negative
    test_century!(
        -1905..=-1000,
        vec![-2000, -1900, -1800, -1700, -1600, -1500, -1400, -1300, -1200, -1100, -1000]
    );
    // crossover
    test_century!(-97..=14, vec![-100, 0]);
}

/// [Iterator::collect_into issue](https://github.com/rust-lang/rust/pull/48597#issuecomment-842083688)
///
/// This is just for trying it out.
trait IteratorExt: Iterator + Sized {
    fn collect_into<E>(self, collection: &mut E)
    where
        E: Extend<Self::Item>,
    {
        collection.extend(self);
    }

    fn collect_with<E>(self, mut collection: E) -> E
    where
        E: Extend<Self::Item>,
    {
        collection.extend(self);
        collection
    }
}
impl<T> IteratorExt for T where T: Iterator {}

#[test]
fn test_ymd_iter() {
    let iter = YearMonthDayIter(IncrementIter::new((2019, 7, 28), (2019, 8, 2)));
    assert_eq!(
        iter.collect_with(Vec::new()),
        vec![
            DateComplete::from_ymd(2019, 7, 28),
            DateComplete::from_ymd(2019, 7, 29),
            DateComplete::from_ymd(2019, 7, 30),
            DateComplete::from_ymd(2019, 7, 31),
            DateComplete::from_ymd(2019, 8, 1),
            DateComplete::from_ymd(2019, 8, 2),
        ],
    );
}

#[test]
fn test_ymd_iter_new_year() {
    let iter = YearMonthDayIter(IncrementIter::new((2012, 12, 29), (2013, 1, 2)));
    assert_eq!(
        iter.collect_with(Vec::new()),
        vec![
            DateComplete::from_ymd(2012, 12, 29),
            DateComplete::from_ymd(2012, 12, 30),
            DateComplete::from_ymd(2012, 12, 31),
            DateComplete::from_ymd(2013, 1, 1),
            DateComplete::from_ymd(2013, 1, 2),
        ],
    );
}

#[test]
fn test_ymd_iter_leap() {
    let nonleap_wholeyear = YearMonthDayIter(IncrementIter::new((2011, 1, 1), (2011, 12, 31)));
    assert_eq!(nonleap_wholeyear.count(), 365);
    let leap_wholeyear = YearMonthDayIter(IncrementIter::new((2012, 1, 1), (2012, 12, 31)));
    assert_eq!(leap_wholeyear.count(), 366);
    let iter = YearMonthDayIter(IncrementIter::new((2019, 2, 27), (2019, 3, 2)));
    assert_eq!(
        iter.collect_with(Vec::new()),
        vec![
            DateComplete::from_ymd(2019, 2, 27),
            DateComplete::from_ymd(2019, 2, 28),
            DateComplete::from_ymd(2019, 3, 1),
            DateComplete::from_ymd(2019, 3, 2),
        ],
    );
    let iter = YearMonthDayIter(IncrementIter::new((2012, 2, 27), (2012, 3, 2)));
    assert_eq!(
        iter.collect_with(Vec::new()),
        vec![
            DateComplete::from_ymd(2012, 2, 27),
            DateComplete::from_ymd(2012, 2, 28),
            DateComplete::from_ymd(2012, 2, 29),
            DateComplete::from_ymd(2012, 3, 1),
            DateComplete::from_ymd(2012, 3, 2),
        ],
    );
}

// Hmm. We're in an unspecified hemisphere. Seasons don't match up with years. Summer in northern
// hemisphere is all within one year, but summer in the southern hemisphere is spread over two
// years.
// Even if you do know the hemisphere, seasons are not easy to match with years, unless there's
// some convention people use that I'm missing. Point is, there's no obvious way to iterate them.
// #[derive(Debug, Copy, Clone)]
// pub struct YearSeasonIter(i32, Season, i32, Season);

/// See [Edtf::iter_smallest]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmallestStep {
    Century(CenturyIter),
    Decade(DecadeIter),
    Year(YearIter),
    Month(YearMonthIter),
    Day(YearMonthDayIter),
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum IterLevel {
    Day,
    Month,
    Season,
    Year,
    Decade,
    Century,
}

#[derive(Copy, Clone)]
enum IntervalPrecision {
    Day(i32, u8, u8),
    Month(i32, u8),
    Season(i32, u8),
    Year(i32),
    Decade(i32),
    Century(i32),
}

impl IntervalPrecision {
    fn discriminant(&self) -> IterLevel {
        match self {
            Self::Day(..) => IterLevel::Day,
            Self::Month(..) => IterLevel::Month,
            Self::Season(..) => IterLevel::Season,
            Self::Year(..) => IterLevel::Year,
            Self::Decade(..) => IterLevel::Decade,
            Self::Century(..) => IterLevel::Century,
        }
    }
    fn lowest_common_precision(self, other: Self) -> IterLevel {
        self.discriminant().max(other.discriminant())
    }
    fn year(&self) -> i32 {
        match *self {
            Self::Day(y, ..)
            | Self::Month(y, ..)
            | Self::Season(y, ..)
            | Self::Year(y, ..)
            | Self::Decade(y, ..)
            | Self::Century(y, ..) => y,
        }
    }
    fn month(&self) -> Option<u8> {
        match *self {
            Self::Day(_, m, ..) | Self::Month(_, m, ..) => Some(m),
            _ => None,
        }
    }
    fn day(&self) -> Option<u8> {
        match *self {
            Self::Day(_, _, d, ..) => Some(d),
            _ => None,
        }
    }
    fn ymd(&self) -> Option<(i32, u32, u32)> {
        let year = self.year();
        let month = self.month()?;
        let day = self.day()?;
        Some((year, month as u32, day as u32))
    }

    // fn round(self, discriminant: Discriminant) -> Option<Self> {
    //     Some(match discriminant {
    //         Discriminant::Century => Self::Century(self.year()),
    //         Discriminant::Decade => Self::Decade(self.year()),
    //         Discriminant::Year => Self::Year(self.year()),
    //         Discriminant::Month => Self::Month(self.year(), self.month()?),
    //         Discriminant::Day => Self::Day(self.year(), self.month()?, self.day()?),
    //         Discriminant::Season => todo!(),
    //     })
    // }

    // fn open_to(self) -> Option<IntervalIter> {
    //     Some(match self {
    //         Self::Century(c) => IntervalIter::Century(Century::new(c..)),
    //     })
    // }

    fn round_with(self, other: Self, discriminant: IterLevel) -> Option<SmallestStep> {
        let sy = self.year();
        let oy = other.year();

        Some(match discriminant {
            IterLevel::Century => SmallestStep::Century(CenturyIter::new(sy..=oy)),
            IterLevel::Decade => SmallestStep::Decade(DecadeIter::new(sy..=oy)),
            IterLevel::Year => SmallestStep::Year(YearIter::new(sy..=oy)),
            IterLevel::Month => SmallestStep::Month(YearMonthIter::new(
                (sy, self.month()? as u32)..=(oy, other.month()? as u32),
            )),
            IterLevel::Day => SmallestStep::Day(YearMonthDayIter::new(self.ymd()?..=other.ymd()?)),
            IterLevel::Season => todo!("season iteration not implemented"),
        })
    }
}

impl Date {
    fn max_interval_precision(&self) -> Option<IntervalPrecision> {
        let (y, yflags) = self.year.unpack();
        if let Some(m) = self.month {
            let (mu8, flags) = m.unpack();
            if flags.is_masked() {
                return None;
            }
            if let Some(d) = self.day {
                let (du8, df) = d.unpack();
                if df.is_masked() {
                    return None;
                }
                return Some(IntervalPrecision::Day(y, mu8, du8));
            }
            if mu8 <= 12 {
                return Some(IntervalPrecision::Month(y, mu8));
            } else {
                return Some(IntervalPrecision::Season(y, mu8));
            }
        }
        Some(match yflags.mask {
            YearMask::None => IntervalPrecision::Year(y),
            YearMask::OneDigit => IntervalPrecision::Decade(y),
            YearMask::TwoDigits => IntervalPrecision::Century(y),
        })
    }
}

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
macro_rules! iterator_test {
    ($d:literal.$ident:ident(), $take:literal, $vec:expr) => {
        assert_eq!(
            Date::parse($d)
                .unwrap()
                .$ident()
                .map(|x| x.take($take).collect_with(Vec::new())),
            $vec
        )
    };
    ($d:literal.$ident:ident(), $vec:expr) => {
        assert_eq!(
            Date::parse($d)
                .unwrap()
                .$ident()
                .map(|x| x.collect_with(Vec::new())),
            $vec
        )
    };
}

#[test]
fn iter_possible_days() {
    let date = Date::parse("2020-08-08").unwrap();
    let days = date
        .iter_possible_days()
        .unwrap()
        .take(5)
        .collect_with(Vec::new());
    assert_eq!(days, vec![DateComplete::from_ymd(2020, 8, 8),]);
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
}

#[test]
fn iter_possible_days_of_month() {
    let date = Date::parse("2020-06-XX").unwrap();
    let count = date.iter_possible_days().unwrap().count();
    assert_eq!(count, 30);
}

#[test]
fn iter_possible_days_nounspec() {
    assert_eq!(Date::parse("2020").unwrap().iter_possible_days(), None);
    assert_eq!(Date::parse("202X").unwrap().iter_possible_days(), None);
    assert_eq!(Date::parse("20XX").unwrap().iter_possible_days(), None);
}

#[test]
fn iter_forward_days() {
    let date = Date::parse("2020-08-08").unwrap();
    let days = date
        .iter_forward_days()
        .unwrap()
        .take(5)
        .collect_with(Vec::new());
    assert_eq!(
        days,
        vec![
            DateComplete::from_ymd(2020, 8, 8),
            DateComplete::from_ymd(2020, 8, 9),
            DateComplete::from_ymd(2020, 8, 10),
            DateComplete::from_ymd(2020, 8, 11),
            DateComplete::from_ymd(2020, 8, 12),
        ]
    );
}

#[test]
fn iter_dayofmonth() {
    let date = Date::parse("2020-05-XX").unwrap();
    let iter = date.iter_possible_days().unwrap();
    let days = iter.take(5).collect_with(Vec::new());
    assert_eq!(
        days,
        vec![
            DateComplete::from_ymd(2020, 5, 1),
            DateComplete::from_ymd(2020, 5, 2),
            DateComplete::from_ymd(2020, 5, 3),
            DateComplete::from_ymd(2020, 5, 4),
            DateComplete::from_ymd(2020, 5, 5),
        ]
    );
}

#[test]
fn iter_dayofyear() {
    let date = Date::parse("2020-XX-XX").unwrap();
    let iter = date.iter_possible_days().unwrap();
    let days = iter.take(5).collect_with(Vec::new());
    assert_eq!(
        days,
        vec![
            DateComplete::from_ymd(2020, 1, 1),
            DateComplete::from_ymd(2020, 1, 2),
            DateComplete::from_ymd(2020, 1, 3),
            DateComplete::from_ymd(2020, 1, 4),
            DateComplete::from_ymd(2020, 1, 5),
        ]
    );
}

#[test]
fn iter_possible_months() {
    assert_eq!(Date::parse("20XX").unwrap().iter_possible_months(), None);
    assert_eq!(Date::parse("202X").unwrap().iter_possible_months(), None);
    assert_eq!(Date::parse("2020").unwrap().iter_possible_months(), None);
    assert_eq!(
        Date::parse("2020-08-XX").unwrap().iter_possible_months(),
        None
    );
    assert_eq!(
        Date::parse("2020-08-09").unwrap().iter_possible_months(),
        None
    );
    assert_eq!(
        Date::parse("2020-09")
            .unwrap()
            .iter_possible_months()
            .unwrap()
            .collect_with(Vec::new()),
        vec![(2020, 09)]
    );
    assert_eq!(
        Date::parse("2020-XX")
            .unwrap()
            .iter_possible_months()
            .unwrap()
            .take(5)
            .collect_with(Vec::new()),
        vec![(2020, 01), (2020, 02), (2020, 03), (2020, 04), (2020, 05)]
    );
}

#[test]
fn iter_forward_months() {
    iterator_test!("2020-05".iter_forward_months(), 3, Some(vec![(2020, 05), (2020, 06), (2020, 07)]));
    iterator_test!("2020-05".iter_forward_months(), 3, Some(vec![(2020, 05), (2020, 06), (2020, 07)]));
}

impl Edtf {
    // TODO: make iterators for OpenFrom/UnknownFrom that simply produce no output unless you
    // reverse them.

    fn interval(&self) -> Option<(Date, Date)> {
        match self {
            // These should work probably
            Self::IntervalOpenTo(_d)
            | Self::IntervalUnknownFrom(_d)
            | Self::IntervalUnknownTo(_d)
            | Self::IntervalOpenFrom(_d) => None,
            Self::Interval(d1, d2) => Some((*d1, *d2)),
            Self::Date(_) => None,
            Self::DateTime(_) => None,
            Self::YYear(_) => None,
        }
    }

    /// If self is an finite interval, returns an enum containing the variant which iterates at
    /// the smallest sized step supported by both ends of the interval.
    ///
    /// Open/unknown ranges return None. So do any unspecified digits in either terminal.
    pub fn iter_smallest(&self) -> Option<SmallestStep> {
        let (d1, d2) = self.interval()?;
        let d1 = d1.max_interval_precision()?;
        let d2 = d2.max_interval_precision()?;
        let disc = d1.lowest_common_precision(d2);
        d1.round_with(d2, disc)
    }

    fn iter_precision(&self, discriminant: IterLevel) -> Option<SmallestStep> {
        let (d1, d2) = self.interval()?;
        let d1 = d1.max_interval_precision()?;
        let d2 = d2.max_interval_precision()?;
        d1.round_with(d2, discriminant)
    }

    pub fn iter_centuries(&self) -> Option<CenturyIter> {
        match self.iter_precision(IterLevel::Century)? {
            SmallestStep::Century(c) => Some(c),
            _ => None,
        }
    }

    pub fn iter_decades(&self) -> Option<DecadeIter> {
        match self.iter_precision(IterLevel::Decade)? {
            SmallestStep::Decade(c) => Some(c),
            _ => None,
        }
    }

    pub fn iter_years(&self) -> Option<YearIter> {
        match self.iter_precision(IterLevel::Year)? {
            SmallestStep::Year(c) => Some(c),
            _ => None,
        }
    }

    pub fn iter_months(&self) -> Option<YearMonthIter> {
        match self.iter_precision(IterLevel::Month)? {
            SmallestStep::Month(c) => Some(c),
            _ => None,
        }
    }

    pub fn iter_days(&self) -> Option<YearMonthDayIter> {
        match self.iter_precision(IterLevel::Day)? {
            SmallestStep::Day(c) => Some(c),
            _ => None,
        }
    }
}

#[test]
fn test_iter_century() {
    let edtf = Edtf::parse("2021-06-28/2021-07-03").unwrap();
    let iter = edtf.iter_centuries().expect("couldn't make the iterator");
    let years = iter.collect_with(Vec::new());
    assert_eq!(years, vec![2000]);
    let edtf = Edtf::parse("1783-06-28/2021-07-03").unwrap();
    let iter = edtf.iter_centuries().expect("couldn't make the iterator");
    let years = iter.collect_with(Vec::new());
    assert_eq!(years, vec![1700, 1800, 1900, 2000]);
}

#[test]
fn test_iter_century_rev() {
    let edtf = Edtf::parse("2021-06-28/2021-07-03").unwrap();
    let iter = edtf.iter_centuries().expect("couldn't make the iterator");
    let iter = iter.rev();
    let years = iter.collect_with(Vec::new());
    assert_eq!(years, vec![2000]);
    let edtf = Edtf::parse("1783-06-28/2021-07-03").unwrap();
    let iter = edtf.iter_centuries().expect("couldn't make the iterator");
    let iter = iter.rev();
    let years = iter.collect_with(Vec::new());
    assert_eq!(years, vec![2000, 1900, 1800, 1700]);
}

#[test]
fn test_iter_decade() {
    let edtf = Edtf::parse("1783-06-28/1789-07-03").unwrap();
    let iter = edtf.iter_decades().expect("couldn't make the iterator");
    let years = iter.collect_with(Vec::new());
    assert_eq!(years, vec![1780]);
    let edtf = Edtf::parse("1783-06-28/1809-07-03").unwrap();
    let iter = edtf.iter_decades().expect("couldn't make the iterator");
    let years = iter.collect_with(Vec::new());
    assert_eq!(years, vec![1780, 1790, 1800]);
}

#[test]
fn test_iter_decade_rev() {
    let edtf = Edtf::parse("1783-06-28/1789-07-03").unwrap();
    let iter = edtf.iter_decades().expect("couldn't make the iterator");
    let iter = iter.rev();
    let years = iter.collect_with(Vec::new());
    assert_eq!(years, vec![1780]);
    let edtf = Edtf::parse("1783-06-28/1809-07-03").unwrap();
    let iter = edtf.iter_decades().expect("couldn't make the iterator");
    let iter = iter.rev();
    let years = iter.collect_with(Vec::new());
    assert_eq!(years, vec![1800, 1790, 1780]);
}

#[test]
fn test_iter_year() {
    let edtf = Edtf::parse("1783-06-28/1783-07-03").unwrap();
    let iter = edtf.iter_years().expect("couldn't make the iterator");
    let years = iter.collect_with(Vec::new());
    assert_eq!(years, vec![1783]);
    let edtf = Edtf::parse("1783-06-28/1789-07-03").unwrap();
    let iter = edtf.iter_years().expect("couldn't make the iterator");
    let years = iter.collect_with(Vec::new());
    assert_eq!(years, vec![1783, 1784, 1785, 1786, 1787, 1788, 1789]);
}

#[test]
fn test_iter_year_rev() {
    let edtf = Edtf::parse("1783-06-28/1783-07-03").unwrap();
    let iter = edtf.iter_years().expect("couldn't make the iterator");
    let iter = iter.rev();
    let years = iter.collect_with(Vec::new());
    assert_eq!(years, vec![1783]);
    let edtf = Edtf::parse("1783-06-28/1789-07-03").unwrap();
    let iter = edtf.iter_years().expect("couldn't make the iterator");
    let iter = iter.rev();
    let years = iter.collect_with(Vec::new());
    assert_eq!(years, vec![1789, 1788, 1787, 1786, 1785, 1784, 1783]);
}

#[test]
fn test_iter_year_month() {
    let edtf = Edtf::parse("1783-06-28/1783-07-03").unwrap();
    let iter = edtf.iter_months().expect("couldn't make the iterator");
    let years = iter.collect_with(Vec::new());
    assert_eq!(years, vec![(1783, 6), (1783, 7)]);
    let edtf = Edtf::parse("1783-11-28/1784-01-03").unwrap();
    let iter = edtf.iter_months().expect("couldn't make the iterator");
    let years = iter.collect_with(Vec::new());
    assert_eq!(years, vec![(1783, 11), (1783, 12), (1784, 1)]);
}

#[test]
fn test_iter_year_month_rev() {
    let edtf = Edtf::parse("1783-06/1783-09").unwrap();
    let iter = edtf.iter_months().expect("couldn't make the iterator");
    let iter = iter.rev();
    let years = iter.collect_with(Vec::new());
    assert_eq!(years, vec![(1783, 9), (1783, 8), (1783, 7), (1783, 6)]);
}

#[test]
fn test_iter_ymd() {
    let edtf = Edtf::parse("1783-06-28/1783-07-03").unwrap();
    let iter = edtf.iter_days().expect("couldn't make the iterator");
    let years = iter.collect_with(Vec::new());
    assert_eq!(
        years,
        vec![
            DateComplete::from_ymd(1783, 6, 28),
            DateComplete::from_ymd(1783, 6, 29),
            DateComplete::from_ymd(1783, 6, 30),
            DateComplete::from_ymd(1783, 7, 1),
            DateComplete::from_ymd(1783, 7, 2),
            DateComplete::from_ymd(1783, 7, 3),
        ]
    );
}

#[test]
fn test_iter_ymd_rev() {
    let edtf = Edtf::parse("1783-06-28/1783-07-03").unwrap();
    let iter = edtf.iter_days().expect("couldn't make the iterator");
    let iter = iter.rev();
    let years = iter.collect_with(Vec::new());
    assert_eq!(
        years,
        vec![
            DateComplete::from_ymd(1783, 7, 3),
            DateComplete::from_ymd(1783, 7, 2),
            DateComplete::from_ymd(1783, 7, 1),
            DateComplete::from_ymd(1783, 6, 30),
            DateComplete::from_ymd(1783, 6, 29),
            DateComplete::from_ymd(1783, 6, 28),
        ]
    );
}

#[test]
fn test_iter_year_month_rev_rev() {
    let edtf = Edtf::parse("1783-06/1783-09").unwrap();
    let mut iter = edtf.iter_months().expect("couldn't make the iterator");

    let rr = iter.clone().rev().rev();
    let years = rr.collect_with(Vec::new());
    assert_eq!(years, vec![(1783, 6), (1783, 7), (1783, 8), (1783, 9)]);

    println!("{:?}", iter);
    assert_eq!(iter.next(), Some((1783, 6)));
    assert_eq!(iter.next_back(), Some((1783, 9)));
    println!("{:?}", iter);
    assert_eq!(iter.next(), Some((1783, 7)));
    assert_eq!(iter.next(), Some((1783, 8)));
    assert_eq!(iter.next(), None);
    assert_eq!(iter.next_back(), None);

    println!("{:?}", iter);
}
