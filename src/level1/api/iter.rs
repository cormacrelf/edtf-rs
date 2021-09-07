use super::*;
use crate::DateComplete;
use core::num::NonZeroU8;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct Interval {
    from: Precision,
    from_certainty: Certainty,
    to: Precision,
    to_certainty: Certainty,
}

impl Interval {
    // fn iter_ignore_certainty(&self) -> Option<IntervalIter> {
    //     IntervalIter {
    //         from: self.from,
    //         to: self.to,
    //     }
    // }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum IterDate {
    Century(i32),
    Decade(i32),
    Year(i32),
    Season(i32, Season),
    Month(i32, u32),
    Day(i32, u32, u32),
}

// trait Sealed {}

trait Incrementable: Copy {
    type Input;
    type Storage: Copy + PartialOrd;
    type Output;
    fn lift(input: Self::Input) -> Self::Storage;
    fn increment(storage: Self::Storage) -> Option<Self::Storage>;
    fn output(storage: Self::Storage) -> Option<Self::Output>;
}

trait Decrementable: Incrementable {
    fn decrement(storage: Self::Storage) -> Option<Self::Storage>;
}

trait Cyclic: Copy {
    type Input;
    type Storage: Copy + PartialOrd;
    type Output;
    fn lift(input: Self::Input) -> Self::Storage;
    fn increment(storage: Self::Storage) -> Result<Self::Storage, Self::Output>;
    fn decrement(storage: Self::Storage) -> Result<Self::Storage, Self::Output>;
    fn incr_map(storage: Self::Storage) -> Result<Self::Output, Self::Output> {
        Self::increment(storage).map(|x| Self::output(x))
    }
    fn decr_map(storage: Self::Storage) -> Result<Self::Output, Self::Output> {
        Self::decrement(storage).map(|x| Self::output(x))
    }
    fn output(storage: Self::Storage) -> Self::Output;
}

macro_rules! incrementable {
    ($vis:vis struct $name:ident::<$storage:ty>($increment_expr:expr, $decrement_expr:expr)) => {
        incrementable!(
            $vis struct $name::<$storage, $storage, $storage>(
                |x| x,
                $increment_expr,
                $decrement_expr,
                Some,
            ));
    };
    (
        $vis:vis struct $name:ident::<$input:ty, $storage:ty, $output:ty>(
            $lift_expr:expr,
            $increment_expr:expr,
            $decrement_expr:expr,
            $output_expr:expr,
        )
    ) => {
        #[derive(Debug, Copy, Clone, PartialEq, Eq)]
        $vis struct $name;
        impl Incrementable for $name {
            type Input = $input;
            type Storage = $storage;
            type Output = $output;
            fn lift(storage: Self::Input) -> Self::Storage {
                $lift_expr(storage)
            }
            fn increment(storage: Self::Storage) -> Option<Self::Storage> {
                $increment_expr(storage)
            }
            fn output(storage: Self::Storage) -> Option<Self::Output> {
                $output_expr(storage)
            }
        }
        impl Decrementable for $name {
            fn decrement(storage: Self::Storage) -> Option<Self::Storage> {
                $decrement_expr(storage)
            }
        }
    }
}

macro_rules! cyclic {
    ($vis:vis struct $name:ident::<$storage:ty>($increment_expr:expr, $decrement_expr:expr)) => {
        cyclic!($vis struct $name::<$storage, $storage, $storage>(|x| x, $increment_expr, $decrement_expr, |x| x,));
    };
    ($vis:vis struct $name:ident::<$input:ty, $storage:ty, $output:ty>(
            $lift_expr:expr,
            $increment_expr:expr,
            $decrement_expr:expr,
            $output_expr:expr,
        )
    ) => {
        #[derive(Debug, Copy, Clone, PartialEq, Eq)]
        struct $name;
        impl Cyclic for $name {
            type Input = $input;
            type Storage = $storage;
            type Output = $output;
            fn lift(storage: Self::Input) -> Self::Storage {
                $lift_expr(storage)
            }
            fn increment(storage: Self::Storage) -> Result<Self::Storage, Self::Output> {
                $increment_expr(storage)
            }
            fn decrement(storage: Self::Storage) -> Result<Self::Storage, Self::Output> {
                $decrement_expr(storage)
            }
            fn output(storage: Self::Storage) -> Self::Output {
                $output_expr(storage)
            }
        }
    };
}

incrementable! {
    struct Century::<i32, i32, i32>(
        |input| input,
        |century: i32| century.checked_add(100),
        |century: i32| century.checked_sub(100),
        Some,
    )
}

incrementable! { struct Year::<i32>(|y: i32| y.checked_add(1), |y: i32| y.checked_sub(1)) }

incrementable! { struct Decade::<i32>(|dec: i32| dec.checked_add(10), |dec: i32| dec.checked_sub(10)) }
cyclic! {
    struct Month::<u32>(
        |m| if m >= 12 {
            Err(1u32)
        } else {
            Ok(m + 1)
        },
        |m| if m <= 1 {
            Err(12u32)
        } else {
            Ok(m - 1)
        }
    )
}
use crate::common::MONTH_DAYCOUNT;
use crate::common::MONTH_DAYCOUNT_LEAP;
cyclic! {
    struct DayOfMax::<(u32, u32), (u32, u32), u32>(
        |x| x,
        |(dmax, day)| {
            if day >= dmax {
                Err(1)
            } else {
                Ok((dmax, day + 1))
            }
        },
        |(dmax, day)| {
            if day <= 1 {
                // Gotta start at the last day of the previous month now
                Err(dmax)
            } else {
                Ok((0, day - 1))
            }
        },
        |(_, day)| day,
    )
}

incrementable! {
    struct YearMonthDay::<(i32, u32, u32), (i32, bool, u32, u32), DateComplete>(
        |(y, m, d)| (y, crate::common::is_leap_year(y), m, d),
        |(year, leap, month, day)| {
            let lut = if leap { MONTH_DAYCOUNT_LEAP } else { MONTH_DAYCOUNT };
            let n_days = lut[month as usize - 1] as u32;
            let next = match DayOfMax::incr_map((n_days, day)) {
                Ok(next_day) => (year, leap, month, next_day),
                // got past the last day this month
                Err(one) => match Month::incr_map(month) {
                    Ok(next_month) => (year, leap, next_month, one),
                    // got past the last month of this year
                    Err(jan) => {
                        let y = i32::checked_add(year, 1)?;
                        (y, crate::common::is_leap_year(y), jan, one)
                    },
                },
            };
            Some(next)
        },
        |(year, leap, month, day)| {
            let lut = if leap { MONTH_DAYCOUNT_LEAP } else { MONTH_DAYCOUNT };
            let prev_month_n_days = if month > 1 {
                lut[month as usize - 2] as u32
            } else {
                31
            };
            let next = match DayOfMax::decr_map((prev_month_n_days, day)) {
                Ok(prev_day) => (year, leap, month, prev_day),
                // got past the first day this month
                Err(thirty_ish) => match Month::decr_map(month) {
                    Ok(prev_month) => {
                        (year, leap, prev_month, thirty_ish)
                    },
                    // got past january of this year, go to december
                    Err(december) => {
                        let y = i32::checked_sub(year, 1)?;
                        (y, crate::common::is_leap_year(y), december, thirty_ish)
                    },
                },
            };
            Some(next)
        },
        |(year, _, month, day): (i32, bool, u32, u32)| {
            let month = month.try_into().ok().and_then(NonZeroU8::new)?;
            let day = day.try_into().ok().and_then(NonZeroU8::new)?;
            Some(DateComplete { year, month, day })
        },
    )
}

incrementable! {
    struct YearMonth::<(i32, u32)>(
        |(year, month)| {
            let x = match Month::incr_map(month) {
                Ok(next_month) => (year, next_month),
                Err(jan) => {
                    let y = i32::checked_add(year, 1)?;
                    (y, jan)
                }
            };
            Some(x)
        },
        |(year, month)| {
            let x = match Month::decr_map(month) {
                Ok(prev_month) => (year, prev_month),
                Err(dec) => {
                    let y = i32::checked_sub(year, 1)?;
                    (y, dec)
                }
            };
            Some(x)
        }
    )
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct IncrementIter<I>
where
    I: Incrementable,
{
    from: Option<I::Storage>,
    to: Option<I::Storage>,
}

impl<I: Incrementable> IncrementIter<I> {
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
         #[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    /// Iterate all centuries that have any part of them included in the date range.
    pub struct CenturyIter(IncrementIter<Century>, type Item = i32; );
}
impl_iter_inner! {
    /// Iterate all decades that have any part of them included in the date range.
    pub struct DecadeIter(IncrementIter<Decade>, type Item = i32; );
}
impl_iter_inner! {
    /// Iterate all years that have any part of them included in the date range.
    pub struct YearIter(IncrementIter<Year>, type Item = i32; );
}
impl_iter_inner! {
    /// Iterate all year-months that have any part of them included in the date range.
    ///
    /// For example, `2019-11-30/2020-01` produces `[2019-11, 2019-12, 2020-01]`.
    pub struct YearMonthIter(IncrementIter<YearMonth>, type Item = (i32, u32); );
}
impl_iter_inner! {
    /// Iterate all days in the range.
    pub struct YearMonthDayIter(IncrementIter<YearMonthDay>, type Item = DateComplete; );
}

use core::ops::RangeInclusive;

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

// impl Iterator for CenturyIter {
//     type Item = i32;
//     fn next(&mut self) -> Option<Self::Item> {
//         let Self(from, to) = *self;
//         let mut iter = from..=to;
//         let nxt = iter.next();
//     }
// }

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum IntervalIter {
    Century(CenturyIter),
    Decade(DecadeIter),
    Year(YearIter),
    Month(YearMonthIter),
    Day(YearMonthDayIter),
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Discriminant {
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
    fn discriminant(&self) -> Discriminant {
        match self {
            Self::Day(..) => Discriminant::Day,
            Self::Month(..) => Discriminant::Month,
            Self::Season(..) => Discriminant::Season,
            Self::Year(..) => Discriminant::Year,
            Self::Decade(..) => Discriminant::Decade,
            Self::Century(..) => Discriminant::Century,
        }
    }
    fn lowest_common_precision(self, other: Self) -> Discriminant {
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

    fn round_with(self, other: Self, discriminant: Discriminant) -> Option<IntervalIter> {
        let sy = self.year();
        let oy = other.year();

        Some(match discriminant {
            Discriminant::Century => IntervalIter::Century(CenturyIter::new(sy..=oy)),
            Discriminant::Decade => IntervalIter::Decade(DecadeIter::new(sy..=oy)),
            Discriminant::Year => IntervalIter::Year(YearIter::new(sy..=oy)),
            Discriminant::Month => IntervalIter::Month(YearMonthIter::new(
                (sy, self.month()? as u32)..=(oy, other.month()? as u32),
            )),
            Discriminant::Day => {
                IntervalIter::Day(YearMonthDayIter::new(self.ymd()?..=other.ymd()?))
            }
            Discriminant::Season => todo!("season iteration not implemented"),
        })
    }
}

impl Date {
    fn max_interval_precision_certain(&self) -> Option<IntervalPrecision> {
        if self.certainty != Certainty::Certain {
            return None;
        }
        let (y, yflags) = self.year.unpack();
        if yflags.certainty != Certainty::Certain {
            return None;
        }
        if let Some(m) = self.month {
            let (mu8, flags) = m.unpack();
            if flags.is_masked() || flags.certainty != Certainty::Certain {
                return None;
            }
            if let Some(d) = self.day {
                let (du8, df) = d.unpack();
                if df.is_masked() || df.certainty != Certainty::Certain {
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

    pub fn iter_certain_best(&self) -> Option<IntervalIter> {
        let (d1, d2) = self.interval()?;
        let d1 = d1.max_interval_precision_certain()?;
        let d2 = d2.max_interval_precision_certain()?;
        let disc = d1.lowest_common_precision(d2);
        d1.round_with(d2, disc)
    }

    fn iter_certain_precision(&self, discriminant: Discriminant) -> Option<IntervalIter> {
        let (d1, d2) = self.interval()?;
        let d1 = d1.max_interval_precision_certain()?;
        let d2 = d2.max_interval_precision_certain()?;
        d1.round_with(d2, discriminant)
    }

    pub fn iter_century(&self) -> Option<CenturyIter> {
        match self.iter_certain_precision(Discriminant::Century)? {
            IntervalIter::Century(c) => Some(c),
            _ => None,
        }
    }

    pub fn iter_decade(&self) -> Option<DecadeIter> {
        match self.iter_certain_precision(Discriminant::Decade)? {
            IntervalIter::Decade(c) => Some(c),
            _ => None,
        }
    }

    pub fn iter_year(&self) -> Option<YearIter> {
        match self.iter_certain_precision(Discriminant::Year)? {
            IntervalIter::Year(c) => Some(c),
            _ => None,
        }
    }

    pub fn iter_month(&self) -> Option<YearMonthIter> {
        match self.iter_certain_precision(Discriminant::Month)? {
            IntervalIter::Month(c) => Some(c),
            _ => None,
        }
    }

    pub fn iter_day(&self) -> Option<YearMonthDayIter> {
        match self.iter_certain_precision(Discriminant::Day)? {
            IntervalIter::Day(c) => Some(c),
            _ => None,
        }
    }
}

#[test]
fn test_iter_century() {
    let edtf = Edtf::parse("2021-06-28/2021-07-03").unwrap();
    let iter = edtf.iter_century().expect("couldn't make the iterator");
    let years = iter.collect_with(Vec::new());
    assert_eq!(years, vec![2000]);
    let edtf = Edtf::parse("1783-06-28/2021-07-03").unwrap();
    let iter = edtf.iter_century().expect("couldn't make the iterator");
    let years = iter.collect_with(Vec::new());
    assert_eq!(years, vec![1700, 1800, 1900, 2000]);
}

#[test]
fn test_iter_century_rev() {
    let edtf = Edtf::parse("2021-06-28/2021-07-03").unwrap();
    let iter = edtf.iter_century().expect("couldn't make the iterator");
    let iter = iter.rev();
    let years = iter.collect_with(Vec::new());
    assert_eq!(years, vec![2000]);
    let edtf = Edtf::parse("1783-06-28/2021-07-03").unwrap();
    let iter = edtf.iter_century().expect("couldn't make the iterator");
    let iter = iter.rev();
    let years = iter.collect_with(Vec::new());
    assert_eq!(years, vec![2000, 1900, 1800, 1700]);
}

#[test]
fn test_iter_decade() {
    let edtf = Edtf::parse("1783-06-28/1789-07-03").unwrap();
    let iter = edtf.iter_decade().expect("couldn't make the iterator");
    let years = iter.collect_with(Vec::new());
    assert_eq!(years, vec![1780]);
    let edtf = Edtf::parse("1783-06-28/1809-07-03").unwrap();
    let iter = edtf.iter_decade().expect("couldn't make the iterator");
    let years = iter.collect_with(Vec::new());
    assert_eq!(years, vec![1780, 1790, 1800]);
}

#[test]
fn test_iter_decade_rev() {
    let edtf = Edtf::parse("1783-06-28/1789-07-03").unwrap();
    let iter = edtf.iter_decade().expect("couldn't make the iterator");
    let iter = iter.rev();
    let years = iter.collect_with(Vec::new());
    assert_eq!(years, vec![1780]);
    let edtf = Edtf::parse("1783-06-28/1809-07-03").unwrap();
    let iter = edtf.iter_decade().expect("couldn't make the iterator");
    let iter = iter.rev();
    let years = iter.collect_with(Vec::new());
    assert_eq!(years, vec![1800, 1790, 1780]);
}

#[test]
fn test_iter_year() {
    let edtf = Edtf::parse("1783-06-28/1783-07-03").unwrap();
    let iter = edtf.iter_year().expect("couldn't make the iterator");
    let years = iter.collect_with(Vec::new());
    assert_eq!(years, vec![1783]);
    let edtf = Edtf::parse("1783-06-28/1789-07-03").unwrap();
    let iter = edtf.iter_year().expect("couldn't make the iterator");
    let years = iter.collect_with(Vec::new());
    assert_eq!(years, vec![1783, 1784, 1785, 1786, 1787, 1788, 1789]);
}

#[test]
fn test_iter_year_rev() {
    let edtf = Edtf::parse("1783-06-28/1783-07-03").unwrap();
    let iter = edtf.iter_year().expect("couldn't make the iterator");
    let iter = iter.rev();
    let years = iter.collect_with(Vec::new());
    assert_eq!(years, vec![1783]);
    let edtf = Edtf::parse("1783-06-28/1789-07-03").unwrap();
    let iter = edtf.iter_year().expect("couldn't make the iterator");
    let iter = iter.rev();
    let years = iter.collect_with(Vec::new());
    assert_eq!(years, vec![1789, 1788, 1787, 1786, 1785, 1784, 1783]);
}

#[test]
fn test_iter_year_month() {
    let edtf = Edtf::parse("1783-06-28/1783-07-03").unwrap();
    let iter = edtf.iter_month().expect("couldn't make the iterator");
    let years = iter.collect_with(Vec::new());
    assert_eq!(years, vec![(1783, 6), (1783, 7)]);
    let edtf = Edtf::parse("1783-11-28/1784-01-03").unwrap();
    let iter = edtf.iter_month().expect("couldn't make the iterator");
    let years = iter.collect_with(Vec::new());
    assert_eq!(years, vec![(1783, 11), (1783, 12), (1784, 1)]);
}

#[test]
fn test_iter_year_month_rev() {
    let edtf = Edtf::parse("1783-06/1783-09").unwrap();
    let iter = edtf.iter_month().expect("couldn't make the iterator");
    let iter = iter.rev();
    let years = iter.collect_with(Vec::new());
    assert_eq!(years, vec![(1783, 9), (1783, 8), (1783, 7), (1783, 6)]);
}

#[test]
fn test_iter_ymd() {
    let edtf = Edtf::parse("1783-06-28/1783-07-03").unwrap();
    let iter = edtf.iter_day().expect("couldn't make the iterator");
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
    let iter = edtf.iter_day().expect("couldn't make the iterator");
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
    let mut iter = edtf.iter_month().expect("couldn't make the iterator");

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
