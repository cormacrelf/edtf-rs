mod zero {
    use edtf::level_0::*;
    use edtf::ParseError;
    #[test]
    fn interval() {
        assert_eq!(
            Edtf::parse("2019-01-07/2020-01"),
            Ok(Edtf::Interval(
                Date::from_ymd(2019, 1, 7),
                Date::from_ymd(2020, 1, 0)
            )),
        );
    }

    #[test]
    fn interval_must_not_end_before_it_starts() {
        assert_eq!(Edtf::parse("1918/1914"), Err(ParseError::OutOfRange));
        assert_eq!(Edtf::parse("2004-06/2004-05"), Err(ParseError::OutOfRange));
        assert_eq!(
            Edtf::parse("2004-06-11/2004-06-10"),
            Err(ParseError::OutOfRange)
        );
        assert!(Edtf::parse("1914/1918").is_ok());
        assert!(Edtf::parse("1914/1914").is_ok());
        // Different precisions that overlap are fine.
        assert!(Edtf::parse("2004-06/2004").is_ok());
        assert!(Edtf::parse("2004-06-30/2004-06").is_ok());
    }
}

mod one {
    use edtf::level_1::*;
    use edtf::ParseError;

    #[test]
    fn test_access() {
        let _x = YYear::new_or_cal(5).map(Edtf::YYear);
        let _y = Edtf::Interval(Date::from_year(2019), Date::from_year(2020));
    }

    #[test]
    fn interval() {
        assert_eq!(
            Edtf::parse("2019-01-07~/2020-01?"),
            Ok(Edtf::Interval(
                Date::from_ymd(2019, 1, 7).and_certainty(Certainty::Approximate),
                Date::from_ymd(2020, 1, 0).and_certainty(Certainty::Uncertain),
            )),
        );
        assert_eq!(
            Edtf::parse("2019-01-XX~/2020-01?"),
            Ok(Edtf::Interval(
                Date::from_precision(Precision::DayOfMonth(2019, 1))
                    .and_certainty(Certainty::Approximate),
                Date::from_ymd(2020, 1, 0).and_certainty(Certainty::Uncertain),
            )),
        );
    }

    #[test]
    fn interval_must_not_end_before_it_starts() {
        assert_eq!(Edtf::parse("1918/1914"), Err(ParseError::OutOfRange));
        assert_eq!(
            Edtf::parse("2004-06-11~/2004-06-10"),
            Err(ParseError::OutOfRange)
        );
        assert_eq!(Edtf::parse("195X/1949"), Err(ParseError::OutOfRange));
        assert_eq!(Edtf::parse("20XX/1999"), Err(ParseError::OutOfRange));
        assert_eq!(Edtf::parse("-0400/-0500"), Err(ParseError::OutOfRange));
        assert_eq!(Edtf::parse("2001-21/2000"), Err(ParseError::OutOfRange));
        assert_eq!(Edtf::parse("-19XX/-2000"), Err(ParseError::OutOfRange));
        assert_eq!(Edtf::parse("-1850/-19XX"), Err(ParseError::OutOfRange));
        assert!(Edtf::parse("1914/1918").is_ok());
        assert!(Edtf::parse("-0500/-0400").is_ok());
        assert!(Edtf::parse("1914/1914").is_ok());
        // Ends are compared on everything they can mean, so overlapping
        // precisions are fine.
        assert!(Edtf::parse("1950/19XX").is_ok());
        assert!(Edtf::parse("19XX/1950").is_ok());
        assert!(Edtf::parse("2004-06-30/2004-06").is_ok());
        assert!(Edtf::parse("2004-06-XX/2004-06-01").is_ok());
        assert!(Edtf::parse("2004-XX-XX/2004-01").is_ok());
        assert!(Edtf::parse("2004-23/2004-08").is_ok());
        // Winter may straddle the turn of the year.
        assert!(Edtf::parse("2005-01/2004-24").is_ok());
        assert!(Edtf::parse("2004-24/2004-02").is_ok());
        assert!(Edtf::parse("-19XX/-1950").is_ok());
        assert!(Edtf::parse("-1950/-19XX").is_ok());
    }
}
