pub mod api;
mod parser;

use crate::ParseError;
use api::Edtf;
use parser::ParsedEdtf;

impl ParsedEdtf {
    fn validate(self) -> Result<Edtf, ParseError> {
        Ok(match self {
            // Self::Date(d) => Edtf::Date(d.validate()?),
            Self::Scientific(scientific) => Edtf::Scientific(scientific.validate()?),
            // Self::Range(d, d2) => Edtf::Range(d.validate()?, d2.validate()?),
            // Self::DateTime(d, t) => Edtf::DateTime(DateTime::validate(d, t)?),
            // Self::RangeOpenStart(start) => Edtf::RangeOpenStart(start.validate()?),
            // Self::RangeOpenEnd(end) => Edtf::RangeOpenEnd(end.validate()?),
            // Self::RangeUnknownStart(start) => Edtf::RangeOpenStart(start.validate()?),
            // Self::RangeUnknownEnd(end) => Edtf::RangeOpenEnd(end.validate()?),
        })
    }
}

// #[cfg(all(test, feature = "FALSE"))]
#[cfg(test)]
mod test {
    use super::api::ScientificYear;
    use super::*;

    #[test]
    fn scientific_l2() {
        // yes - 1+ digits E
        assert_eq!(
            Edtf::parse("Y17E7"),
            Ok(Edtf::Scientific(ScientificYear::new(17, 7, 0)))
        );
        assert_eq!(
            Edtf::parse("Y17E7S3"),
            Ok(Edtf::Scientific(ScientificYear::new(17, 7, 3)))
        );
        // yes - 1+ digits E, negative
        assert_eq!(
            Edtf::parse("Y-17E7"),
            Ok(Edtf::Scientific(ScientificYear::new(-17, 7, 0)))
        );
        assert_eq!(
            Edtf::parse("Y-17E7S3"),
            Ok(Edtf::Scientific(ScientificYear::new(-17, 7, 3)))
        );
        // yes - <5 digits with E and S
        assert_eq!(
            Edtf::parse("Y1745E1S3"),
            Ok(Edtf::Scientific(ScientificYear::new(1745, 1, 3)))
        );
        assert_eq!(
            Edtf::parse("Y1745E0S3"),
            Ok(Edtf::Scientific(ScientificYear::new(1745, 0, 3)))
        );
        assert_eq!(
            Edtf::parse("Y157900S3"),
            Ok(Edtf::Scientific(ScientificYear::new(157900, 0, 3)))
        );
        // yes - 5+ digits negative
        assert_eq!(
            Edtf::parse("Y-157900"),
            Ok(Edtf::Scientific(ScientificYear::new(-157900, 0, 0)))
        );
        assert_eq!(
            Edtf::parse("Y-157900S3"),
            Ok(Edtf::Scientific(ScientificYear::new(-157900, 0, 3)))
        );
        // yes - 5+ digits E
        assert_eq!(
            Edtf::parse("Y157900E3"),
            Ok(Edtf::Scientific(ScientificYear::new(157900, 3, 0)))
        );
        assert_eq!(
            Edtf::parse("Y157900E3S3"),
            Ok(Edtf::Scientific(ScientificYear::new(157900, 3, 3)))
        );
        // yes - 5+ digits E negative
        assert_eq!(
            Edtf::parse("Y-157900E3"),
            Ok(Edtf::Scientific(ScientificYear::new(-157900, 3, 0)))
        );
        assert_eq!(
            Edtf::parse("Y-157900E3S3"),
            Ok(Edtf::Scientific(ScientificYear::new(-157900, 3, 3)))
        );

        // no - fewer than 5 digits
        assert_eq!(Edtf::parse("Y1745"), Err(ParseError::Invalid));
        assert_eq!(Edtf::parse("Y1745S3"), Err(ParseError::Invalid));
        // no - overflow
        assert_eq!(Edtf::parse("Y17E200"), Err(ParseError::Invalid));
        // no - too many significant digits
        assert_eq!(Edtf::parse("Y12345S7"), Err(ParseError::Invalid));

        // yes - scientific four digit year
        assert_eq!(
            Edtf::parse("1234S2"),
            Ok(Edtf::Scientific(ScientificYear::new(1234, 0, 2)))
        );
        // yes - scientific four digit year, negative
        assert_eq!(
            Edtf::parse("-1234S2"),
            Ok(Edtf::Scientific(ScientificYear::new(-1234, 0, 2)))
        );
    }
}
