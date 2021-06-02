mod zero {
    use edtf::level_0::*;
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
}

mod one {
    use edtf::level_1::*;

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
                Date::from_ym_masked_day(2019, 1).and_certainty(Certainty::Approximate),
                Date::from_ymd(2020, 1, 0).and_certainty(Certainty::Uncertain),
            )),
        );
    }
}

mod version {
    #[test]
    fn test_readme_deps() {
        version_sync::assert_markdown_deps_updated!("README.md");
    }
}
