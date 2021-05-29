use crate::ParseError;

/// A year equal to `sign(mantissa) * abs(mantissa) * 10^exponent`, to a precision of
/// `significant_digits`.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct ScientificYear {
    /// Includes the sign bit for convenience of representation.
    pub(crate) mantissa: i64,
    pub(crate) exponent: u16,
    pub(crate) significant_digits: u16,
}

// placeholder for the doctests below
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Edtf {
    Scientific(ScientificYear),
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
    pub(crate) fn validate(self) -> Result<Self, ParseError> {
        let Self {
            significant_digits: sd,
            ..
        } = self;
        // if the value overflows an i64, it's frankly too big. The universe is only 13.77 billion
        // years old.
        let v = self.value_opt().ok_or(ParseError::Invalid)?;
        // Now deal with e.g. 15000S44 -- this is nonsensical. We don't allow 'decimal points' of
        // precision on years.
        let num_digits = n_base10_digits(v);
        if sd > num_digits {
            return Err(ParseError::Invalid);
        }
        Ok(self)
    }
}

fn n_base10_digits(n: i64) -> u16 {
    (n.abs() as f64).log10().ceil() as u16
}

impl ScientificYear {
    /// ```
    /// use edtf::level_2::ScientificYear;
    ///
    /// let auto = ScientificYear::auto(1_700_000);
    /// let manual = ScientificYear::new(17, 5, 0);
    /// assert_eq!(auto, manual);
    /// ```
    pub fn auto(n: i64) -> Self {
        let mut s = Self::new(n, 0, 0);
        while s.mantissa % 10 == 0 {
            s.mantissa /= 10;
            s.exponent += 1;
        }
        // E2 doesn't really help readability
        if s.exponent <= 2 {
            return Self::new(n, 0, 0);
        }
        s
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
    pub fn and_sd(&self, significant_digits: u16) -> Self {
        let val = self.value();
        let dig = n_base10_digits(val);
        let sd = core::cmp::min(dig, significant_digits);
        Self {
            significant_digits: sd,
            ..*self
        }
    }

    /// Creates a new ScientificYear with the given mantissa, exponent, and significant digits.
    ///
    /// The mantissa contains the sign. This function panics if the values provided would overflow
    /// when computed.
    ///
    /// ```
    /// use edtf::level_2::{Edtf, ScientificYear};
    /// let edtf = Edtf::parse("Y17E7S3").unwrap();
    /// let year = Edtf::Scientific(ScientificYear::new(17, 7, 3));
    /// assert_eq!(edtf, year);
    /// ```
    pub fn new(mantissa: i64, exponent: u16, significant_digits: u16) -> Self {
        Self::new_opt(mantissa, exponent, significant_digits)
            .expect("ScientificYear::new values would overflow i64")
    }

    /// Creates a new ScientificYear, but returns `None` if the values provided would overflow when
    /// computed.
    ///
    /// Significant_digits is set to the minimum of the passed argument and the number of
    /// digits in the value.
    pub fn new_opt(mantissa: i64, exponent: u16, significant_digits: u16) -> Option<Self> {
        ScientificYear {
            mantissa,
            exponent,
            significant_digits: 0,
        }
        .validate()
        .map(|x| x.and_sd(significant_digits))
        .ok()
    }

    /// Gets the value of a scientific year, by the following formula:
    ///
    /// ```ignore
    /// sign(mantissa) * abs(mantissa) * 10 ^ exponent
    /// ```
    ///
    /// If the value of a scientific year overflows an i64, it's frankly too big. The universe is
    /// only 13.77 billion years old, but i64 represents ±9e18, which is 10^8 universe-ages in each
    /// direction. Nevertheless, it is easy to accidentally write an overflowing year in
    /// exponential notation. This function panics on overflow as is usual for Rust numerics.
    ///
    /// Note that this library validates that the value does not overflow after parsing, so if you
    /// are reading a parsed value, this function will never panic.
    pub fn value(&self) -> i64 {
        self.value_opt()
            .expect("ScientificYear::value() overflowed i64")
    }

    /// Gets the value of a scientific year, returning `None` instead of panicking on overflow.
    pub fn value_opt(&self) -> Option<i64> {
        let Self {
            mantissa: m,
            exponent: exp,
            ..
        } = *self;
        let tens: i64 = 10i64.checked_pow(exp as u32)?;
        let sign = m.signum();
        let abs = m.abs();
        let unsigned = abs.checked_mul(tens)?;
        unsigned.checked_mul(sign)
    }
}

use core::fmt;

impl fmt::Display for ScientificYear {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Y{}", self.mantissa)?;
        if self.exponent > 0 {
            write!(f, "E{}", self.exponent)?;
        }
        if self.significant_digits > 0 {
            write!(f, "S{}", self.significant_digits)?;
        }
        Ok(())
    }
}

impl fmt::Debug for ScientificYear {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

#[test]
fn scientific_value() {
    let val = ScientificYear::new(17, 7, 3);
    assert_eq!(val.value(), 17 * 10i64.pow(7));
}

#[test]
fn truncate_sd() {
    // passing an SD that's too big is okay, it will get truncated
    assert_eq!(
        ScientificYear::new(53, 3, 8),
        ScientificYear {
            mantissa: 53,
            exponent: 3,
            significant_digits: 5
        }
    );
    // works with negative values?
    assert_eq!(
        ScientificYear::new(-53, 3, 8),
        ScientificYear {
            mantissa: -53,
            exponent: 3,
            significant_digits: 5
        }
    );
}
