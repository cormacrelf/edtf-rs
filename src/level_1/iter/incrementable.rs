// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright © 2021 Corporation for Digital Scholarship

use crate::DateComplete;
use core::num::NonZeroU8;
use std::convert::TryInto;

pub(crate) trait Incrementable: Copy {
    type Input;
    type Storage: Copy + PartialOrd;
    type Output;
    fn lift(input: Self::Input) -> Self::Storage;
    fn increment(storage: Self::Storage) -> Option<Self::Storage>;
    fn output(storage: Self::Storage) -> Option<Self::Output>;
}

pub(crate) trait Decrementable: Incrementable {
    fn decrement(storage: Self::Storage) -> Option<Self::Storage>;
}

pub(crate) trait Cyclic: Copy {
    type Input;
    type Storage: Copy + PartialOrd;
    type Output;
    fn lift(input: Self::Input) -> Self::Storage;
    fn increment(storage: Self::Storage) -> Result<Self::Storage, Self::Output>;
    fn decrement(storage: Self::Storage) -> Result<Self::Storage, Self::Output>;
    fn incr_map(storage: Self::Storage) -> Result<Self::Output, Self::Output> {
        Self::increment(storage).map(Self::output)
    }
    fn decr_map(storage: Self::Storage) -> Result<Self::Output, Self::Output> {
        Self::decrement(storage).map(Self::output)
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
    pub(crate) struct Century::<i32, i32, i32>(
        |input| input,
        |century: i32| century.checked_add(100),
        |century: i32| century.checked_sub(100),
        Some,
    )
}

incrementable! { pub(crate) struct Year::<i32>(|y: i32| y.checked_add(1), |y: i32| y.checked_sub(1)) }

incrementable! { pub(crate) struct Decade::<i32>(|dec: i32| dec.checked_add(10), |dec: i32| dec.checked_sub(10)) }
cyclic! {
    pub(crate) struct Month::<u32>(
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
    pub(crate) struct DayOfMax::<(u32, u32), (u32, u32), u32>(
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
    pub(crate) struct YearMonthDay::<(i32, u32, u32), (i32, bool, u32, u32), DateComplete>(
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
    pub(crate) struct YearMonth::<(i32, u32)>(
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
