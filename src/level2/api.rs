// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright © 2021 Corporation for Digital Scholarship

use crate::helpers::{inside_9999, outside_9999};
use crate::ParseError;

/// A year equal to `mantissa * 10^exponent`, to a precision of `sig_digits`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ScientificYear {
    /// Includes the sign bit for convenience of representation.
    pub(crate) mantissa: i64,
    pub(crate) exponent: Option<u16>,
    pub(crate) sig_digits: Option<u16>,
}

// placeholder for the doctests below
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Edtf {
    Date(Date),
    Scientific(ScientificYear),
}

// placeholder for the doctests below
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Date {
    Placeholder,
}

use super::parser::ParsedEdtf;
impl Edtf {
    pub fn parse(input: &str) -> Result<Self, ParseError> {
        ParsedEdtf::parse_inner(input).and_then(ParsedEdtf::validate)
    }
}

impl ScientificYear {
    /// If the year is <= 4 digits long, we ought to throw a parse error before this
    /// validator, so this is not checked here. Instead, this validator can check any of the
    /// three forms of scientific year for overflow and mathematical sense, namely
    /// `1500S2`/`-1500S2`, `Y15000`/`Y-15000` and `Y-17E7`. ('Negative calendar
    /// year'/`-1985` is not included as one of these.)
    #[cfg_attr(all(test, not(debug_assertions)), no_panic::no_panic)]
    pub(crate) fn validate(self) -> Result<Self, ParseError> {
        // if the value overflows an i64, it's frankly too big. The universe is only 13.77 billion
        // years old.
        let v = self.value_opt().ok_or(ParseError::Invalid)?;

        // E0 is invalid to parse
        if let Some(0) = self.exponent {
            return Err(ParseError::Invalid);
        }
        if let Some(sd) = self.sig_digits {
            // Now deal with e.g. 15000S44 -- this is nonsensical. We don't allow 'decimal points' of
            // precision on years.
            let num_digits = n_base10_digits(v);
            if sd == 0 || sd > num_digits {
                return Err(ParseError::Invalid);
            }
        }
        Ok(self)
    }
}

#[cfg_attr(all(test, not(debug_assertions)), no_panic::no_panic)]
fn n_base10_digits(n: i64) -> u16 {
    let mut n = n.abs() as u64;
    let mut count = 0;
    while n != 0 {
        n /= 10;
        count += 1;
    }
    // 0 is 1 digit long
    core::cmp::max(count, 1)
}

#[test]
fn test_n_base10_digits() {
    assert_eq!(n_base10_digits(0), 1);
    assert_eq!(n_base10_digits(5), 1);
    assert_eq!(n_base10_digits(52), 2);
    assert_eq!(n_base10_digits(50), 2);
    assert_eq!(n_base10_digits(1000), 4);
    assert_eq!(n_base10_digits(1204), 4);
    // neg
    assert_eq!(n_base10_digits(-0), 1);
    assert_eq!(n_base10_digits(-5), 1);
    assert_eq!(n_base10_digits(-52), 2);
    assert_eq!(n_base10_digits(-50), 2);
    assert_eq!(n_base10_digits(-1000), 4);
    assert_eq!(n_base10_digits(-1204), 4);
}

fn some_if_nonzero(n: u16) -> Option<u16> {
    Some(n).filter(|&x| x > 0)
}

impl ScientificYear {
    /// Gets the value of a scientific year, by the formula `mantissa * 10 ^ exponent`
    ///
    /// Note that this library validates that the value does not overflow after parsing, and it is
    /// impossible to programmatically create a `ScientificYear`, so this function will never panic
    /// due to overflow.
    pub fn value(&self) -> i64 {
        self.value_opt()
            .expect("ScientificYear::value() overflowed i64")
    }

    /// Gets the range of values covered by the use of significant digits.
    ///
    /// Note that the 'estimate' in the EDTF spec is returned by [ScientificYear::value].
    ///
    /// ```
    /// use edtf::level_2::ScientificYear;
    ///
    /// let year = ScientificYear::new(1950, 0, 2);
    /// assert_eq!(year.range(), 1900..=1999);
    /// assert_eq!(year.value(), 1950);
    ///
    /// assert_eq!(ScientificYear::new(195, 1, 0).range(), 1950..=1950);
    /// assert_eq!(ScientificYear::new(1950, 0, 1).range(), 1000..=1999);
    /// assert_eq!(ScientificYear::new(1950, 0, 2).range(), 1900..=1999);
    /// assert_eq!(ScientificYear::new(1950, 0, 3).range(), 1950..=1959);
    /// assert_eq!(ScientificYear::new(1950, 0, 4).range(), 1950..=1950);
    ///
    /// let year = ScientificYear::new(171_010_000, 0, 3);
    /// assert_eq!(year.range(), 171_000_000..=171_999_999);
    /// assert_eq!(year.value(), 171_010_000);
    ///
    /// let year = ScientificYear::new(3388, 2, 3);
    /// assert_eq!(year.range(), 338_000..=338_999);
    /// assert_eq!(year.value(), 338_800);
    /// ```
    ///
    /// Note that [core::ops::RangeInclusive::contains] exists and can tell you if a year is within
    /// the range, or iterate over the years in the range.
    ///
    /// ```
    /// use edtf::level_2::ScientificYear;
    ///
    /// let year = ScientificYear::new(1950, 0, 2);
    /// assert!( year.range().contains(&1999));
    /// assert!( year.range().contains(&1940));
    /// assert!(!year.range().contains(&1800));
    /// ```
    pub fn range(&self) -> RangeInclusive<i64> {
        let val = self.value();
        // let's just be extra safe about nonzero mod
        if let Some(sd) = self.sig_digits.filter(|&x| x != 0) {
            let n_dig = n_base10_digits(val);
            let nines_width = n_dig.saturating_sub(sd);
            let tens = 10i64.pow(nines_width as u32);
            let start = val - val % tens;
            let end = start + tens - 1;
            start..=end
        } else {
            val..=val
        }
    }

    /// Creates a new ScientificYear with the given mantissa, exponent, and significant digits.
    ///
    /// The mantissa contains the sign.
    ///
    /// Zero for either exponent or sig_digits with 4-digit years can also result in
    /// the year being unrepresentable as a ScientificYear, i.e. you should be creating a calendar
    /// [Date] instead. This function also panics in that case.
    ///
    /// See details on `exponent` and `sig_digits` in [ScientificYear::new_or_cal].
    ///
    /// ```
    /// use edtf::level_2::{Edtf, ScientificYear};
    /// let edtf = Edtf::parse("Y17E7S3").unwrap();
    /// let year = ScientificYear::new(17, 7, 3);
    /// assert_eq!(edtf, Edtf::Scientific(year));
    /// assert_eq!(year.value(), 170_000_000);
    /// ```
    ///
    /// This function panics if the values provided would overflow
    /// when computed. *If the value of a scientific year overflows an i64, it's frankly too big.
    /// The universe is only 13.77 billion years old, but i64 represents ±9e18, which is 10^8
    /// universe-ages in each direction. Nevertheless, it is easy to accidentally write an
    /// overflowing year in exponential notation.*
    ///
    /// ```should_panic
    /// use edtf::level_2::ScientificYear;
    /// let _this_will_panic = ScientificYear::new(1999, 0, 0);
    /// ```
    pub fn new(mantissa: i64, exponent: u16, sig_digits: u16) -> Self {
        Self::new_or_cal(mantissa, exponent, sig_digits)
            .map_err(|e| {
                e.unwrap_or_else(|| {
                    panic!(
                        "ScientificYear::new values overflowed: {} * 10^{}",
                        mantissa, exponent
                    )
                })
            })
            .expect("cannot be represented as ScientificYear")
    }

    /// Creates a new ScientificYear, and does not panic.
    ///
    /// sig_digits is set to the minimum of the passed argument and the number of
    /// digits in the value.
    ///
    /// If 0 is passed as exponent and/or sig_digits, that is considered to be "no exponent" and/or
    /// "no significant digits" because neither E nor S in Y-years can be followed by a zero.
    ///
    /// If you pass 0 for both, and then the mantissa is in the range `-9999..9999`, then you get
    /// `Err(Some(Edtf::Date(...)))` returned. This is because EDTF does not allow a `Y`-date with
    /// four or fewer digits, and you cannot tack on E0 or S0 to make that valid. So the
    /// appropriate representation is an `Edtf::Date`, aka a Calendar date.
    ///
    /// ```
    /// use edtf::level_2::{ScientificYear, Edtf, Date};
    /// let ex1 = ScientificYear::new_or_cal(12345, 5, 0).map(|x| x.to_string());
    /// assert_eq!(ex1, Ok("Y12345E5".into()));
    /// let ex1 = ScientificYear::new_or_cal(12, 5, 0).map(|x| x.to_string());
    /// assert_eq!(ex1, Ok("Y12E5".into()));
    /// let ex2 = ScientificYear::new_or_cal(123, 1, 0).map(|x| x.to_string());
    /// assert_eq!(ex2, Ok("Y123E1".into()));
    /// let ex3 = ScientificYear::new_or_cal(1234, 0, 0).map(|x| x.to_string());
    /// assert_eq!(ex3, Err(Some(Edtf::Date(Date::Placeholder))));
    /// ```
    ///
    /// If the value overflows, you get `Err(None)`. This is not super hard to do with an exponent.
    ///
    /// ```
    /// use edtf::level_2::ScientificYear;
    /// let overflow = ScientificYear::new_or_cal(10, 20, 0); // oops!
    /// assert_eq!(overflow, Err(None));
    /// ```
    ///
    #[cfg_attr(all(test, not(debug_assertions)), no_panic::no_panic)]
    pub fn new_or_cal(mantissa: i64, exponent: u16, sig_digits: u16) -> Result<Self, Option<Edtf>> {
        ScientificYear {
            mantissa,
            exponent: some_if_nonzero(exponent),
            sig_digits: None,
        }
        .validate()
        .map(|x| x.and_sd(sig_digits))
        .map_err(|_x| None)
        .and_then(|x| {
            let v = x.value_opt();
            if x.sig_digits.is_none() && x.exponent() == 0 && v.map_or(false, inside_9999) {
                return Err(Some(Edtf::Date(Date::Placeholder)));
            }
            Ok(x)
        })
    }

    /// Sets a nice exponent for a particular year. Currently this means using an exponents to
    /// eliminate all the trailing zeroes in the value, except in some corner cases and when exp
    /// would be <= 2. Panics if unrepresentable as a ScientificYear.
    ///
    /// ```
    /// use edtf::level_2::ScientificYear;
    ///
    /// let auto = ScientificYear::auto(1_700_000);
    /// let manual = ScientificYear::new(17, 5, 0);
    /// assert_eq!(auto, manual);
    /// ```
    pub fn auto(n: i64) -> Self {
        Self::auto_opt(n)
            .expect("invalid year for ScientificYear, must be OUTSIDE range -9999..=9999 or have trailing zeroes to place in an exponent")
    }

    /// [ScientificYear::auto] that doesn't panic.
    ///
    /// ```
    /// use edtf::level_2::ScientificYear;
    ///
    /// let fine = ScientificYear::auto_opt(1_700_000);
    /// assert_eq!(fine, Some(ScientificYear::new(17, 5, 0)));
    /// let neg = ScientificYear::auto_opt(-1_700_000);
    /// assert_eq!(neg, Some(ScientificYear::new(-17, 5, 0)));
    /// let valid_4digit = ScientificYear::auto_opt(1750);
    /// assert_eq!(valid_4digit, Some(ScientificYear::new(175, 1, 0)));
    /// let invalid_4digit = ScientificYear::auto_opt(1751);
    /// assert_eq!(invalid_4digit, None);
    /// ```
    #[cfg_attr(all(test, not(debug_assertions)), no_panic::no_panic)]
    pub fn auto_opt(mut m: i64) -> Option<Self> {
        let orig = m;
        let mut e = 0;
        while m % 10 == 0 {
            m /= 10;
            e += 1;
        }
        let s = Self {
            mantissa: m,
            exponent: some_if_nonzero(e),
            sig_digits: None,
        };
        let val = s.value_opt();
        match val {
            Some(x) if inside_9999(x) && e == 0 => return None,
            // overflow is actually impossible, we were given the value m to begin with
            None => return None,
            _ => {}
        }
        // cannot be represented as an exponential year as no trailing zeroes
        // and <= 4 digits. So Y1234 is not valid syntax.
        if inside_9999(m) && e == 0 {
            return None;
        }
        // E2 doesn't really help readability
        if outside_9999(m) && e <= 2 {
            return Some(Self::raw(orig, None, None));
        }
        Some(s)
    }

    /// Runs self through [ScientificYear::auto]. Any overflows/errors result in a new `ScientificYear`
    /// with no changes applied.
    #[cfg_attr(all(test, not(debug_assertions)), no_panic::no_panic)]
    pub fn normalise(&self) -> Self {
        self.value_opt()
            .and_then(|v| Self::auto_opt(v))
            .map(|s| s.and_sd(self.sig_digits.unwrap_or(0)))
            .unwrap_or(*self)
    }

    /// Sets SD to the min of the provided value and the number of digits in the value.
    ///
    /// ```
    /// use edtf::level_2::ScientificYear;
    ///
    /// let auto = ScientificYear::auto(1_700_000).and_sd(3);
    /// let manual = ScientificYear::new(17, 5, 3);
    /// assert_eq!(auto, manual);
    /// ```
    ///
    /// `sig_digits` will be clamped to a maximum of the number of decimal digits in the value.
    /// Setting `sig_digits` to zero removes the significant digits entry, only if the value is
    /// outside the range `-9999..=9999`.
    #[cfg_attr(all(test, not(debug_assertions)), no_panic::no_panic)]
    pub fn and_sd(&self, sig_digits: u16) -> Self {
        let val = match self.value_opt() {
            Some(x) => x,
            None => return *self,
        };
        let dig = n_base10_digits(val);
        let sd = core::cmp::min(dig, sig_digits);
        if sd == 0 {
            if outside_9999(val) && sd == 0 {
                Self {
                    sig_digits: None,
                    ..*self
                }
            } else {
                *self
            }
        } else {
            Self {
                sig_digits: Some(sd),
                ..*self
            }
        }
    }
    // | <4 digits | >4 digits | has exponent | has sig_digits | ScientificYear | Calendar Date |
    // | :-------: | :-------: | :----------: | :------------: | :------------- | :------------ |
    // | - | ☑️ | ☑️ | ☑️ | ✅ | ❌ |
    // | - | ☑️ | - | ☑️ | ✅ | ❌ |
    // | - | ☑️ | ☑️ | - | ✅ | ❌ |
    // | - | ☑️ | - | - | ✅ | ❌ |
    // | - | - | ☑️ | ☑️ | ✅ | ❌ |
    // | - | - | - | ☑️ | ✅ | ❌ |
    // | - | ☑️ | ☑️ | - | ✅ | ❌ |
    // | - | ☑️ | - | - | ✅ | ❌ |
    // | - | - | - | - | ❌ | ✅ |

    /// Gets the value of a scientific year, returning `None` instead of panicking on overflow.
    #[cfg_attr(all(test, not(debug_assertions)), no_panic::no_panic)]
    pub(crate) fn value_opt(&self) -> Option<i64> {
        let m = self.mantissa;
        let exp = self.exponent();
        let tens: i64 = 10i64.checked_pow(exp as u32)?;
        m.checked_mul(tens)
    }

    /// Gets the mantissa
    pub fn mantissa(&self) -> i64 {
        self.mantissa
    }
    /// Gets the exponent. 0 means absent, but if you're using it as an exponent, obviously it also
    /// serves as a no-op because `10^0 == 1`.
    pub fn exponent(&self) -> u16 {
        self.exponent.unwrap_or(0)
    }

    /// Gets the number of significant digits. 0 means absent.
    pub fn sig_digits(&self) -> u16 {
        self.sig_digits.unwrap_or(0)
    }

    /// Private -- create ScientificYear directly
    pub(crate) fn raw(m: i64, e: Option<u16>, s: Option<u16>) -> Self {
        Self {
            mantissa: m,
            exponent: e,
            sig_digits: s,
        }
    }
}

use core::fmt;
use std::ops::RangeInclusive;

impl fmt::Display for ScientificYear {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Y{}", self.mantissa)?;
        if let Some(e) = self.exponent {
            write!(f, "E{}", e)?;
        }
        if let Some(sd) = self.sig_digits {
            write!(f, "S{}", sd)?;
        }
        Ok(())
    }
}

#[test]
fn scientific_value() {
    let val = ScientificYear::new(17, 7, 3);
    assert_eq!(val.value(), 17 * 10i64.pow(7));
}

// XXX: unsure about these. It would be better to have a way to create an SY
// that falls back to a Edtf::Date calendar date.
#[test]
fn scientific_no_lt_5_digits() {
    let val = ScientificYear::new_or_cal(17, 0, 0);
    assert_eq!(val, Err(Some(Edtf::Date(Date::Placeholder))));
    let val = ScientificYear::auto_opt(17)
        .as_ref()
        .map(ToString::to_string);
    assert_eq!(val, None);
}

#[test]
fn truncate_sd() {
    // passing an SD that's too big is okay, it will get truncated
    assert_eq!(
        ScientificYear::new(53, 3, 8),
        ScientificYear::raw(53, Some(3), Some(5)),
    );
    // works with negative values?
    assert_eq!(
        ScientificYear::new(-53, 3, 8),
        ScientificYear::raw(-53, Some(3), Some(5)),
    );
}

#[test]
fn constructors() {
    assert_eq!(
        ScientificYear::new(53, 0, 2),
        ScientificYear::raw(53, None, Some(2)),
    );
    assert_eq!(
        ScientificYear::new_or_cal(53, 0, 0),
        Err(Some(Edtf::Date(Date::Placeholder)))
    );
    assert_eq!(
        ScientificYear::new(53, 0, 8),
        ScientificYear::raw(53, None, Some(2)),
    );
    assert_eq!(
        ScientificYear::new(53, 2, 0),
        ScientificYear::raw(53, Some(2), None),
    );
}

#[test]
fn auto() {
    assert_eq!(
        ScientificYear::auto(530),
        ScientificYear::raw(53, Some(1), None),
    );
    assert_eq!(ScientificYear::auto_opt(53), None,);
}
