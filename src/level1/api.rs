use super::packed::PackedInt;
use super::packed::DMEnum::*;
use crate::{ParseError, level1::packed::YearFlags};

pub use super::Date;
pub use crate::level1::packed::YearMask;
pub use crate::level1::packed::Certainty;

// TODO: Hash everywhere
// TODO: wrap Certainty with one that doesn't expose the implementation detail

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatePart {
    Masked,
    Normal(u8, Certainty),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Season {
    Spring = 21,
    Summer = 22,
    Autumn = 23,
    Winter = 24,
}

impl Season {
    fn from_u32(value: u32) -> Self {
        match value {
            21 => Self::Spring,
            22 => Self::Summer,
            23 => Self::Autumn,
            24 => Self::Winter,
            _ => panic!("invalid season number {}", value),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatePrecision {
    Year(i32, Certainty, YearMask),
    Month(i32, Certainty, DatePart),
    Day(i32, Certainty, DatePart, DatePart),
    Season(i32, Certainty, Season, Certainty),
}

impl Date {
    pub fn parse(input: &str) -> Result<Self, ParseError> {
        Self::parse_inner(input).and_then(Self::validate)
    }
    pub fn as_precision(&self) -> DatePrecision {
        let (y, YearFlags { certainty: yc, mask: ym }) = self.year.unpack();
        match (self.month, self.day) {
            (Some(month), None) => match month {
                Unmasked(m, c) => {
                    let m = m.get();
                    if m >= 21 && m <= 24 {
                        DatePrecision::Season(y, yc, Season::from_u32(m as u32), c)
                    } else if m >= 1 && m <= 12 {
                        DatePrecision::Month(y, yc, DatePart::Normal(m, c))
                    } else {
                        unreachable!("month was out of range")
                    }
                }
                Masked => DatePrecision::Month(y, yc, DatePart::Masked),
            },
            (Some(month), Some(day)) => match (month, day) {
                (Masked, Unmasked(..)) => unreachable!("date should never hold a masked month with unmasked day"),
                (Masked, Masked) => DatePrecision::Day(y, yc, DatePart::Masked, DatePart::Masked),
                (Unmasked(m, c), Masked) => DatePrecision::Day(y, yc, DatePart::Normal(m.get(), c), DatePart::Masked),
                (Unmasked(m, mc), Unmasked(d, dc)) => DatePrecision::Day(y, yc, DatePart::Normal(m.get(), mc), DatePart::Normal(d.get(), dc)),
            }
            (None, None) => DatePrecision::Year(y, yc, ym),
            (None, Some(_)) => unreachable!("date should never hold a day but not a month"),
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;
    #[test]
    fn match_precision() {
        let date = Date::parse("2019-09?").unwrap();
        assert_eq!(date.as_precision(), DatePrecision::Month(2019, Certainty::Certain, DatePart::Normal(9, Certainty::Uncertain)));
    }
}
