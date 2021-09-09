use super::Certainty::*;
use super::*;
use crate::ParseError::{self, *};

#[test]
fn uncertain_dates_packed() {
    use super::packed::PackedInt;
    let d = Date::parse("2019-07-05%").unwrap();
    println!("{:?}", d.year.unpack());
    println!("{:?}", d.month);
    println!("{:?}", d.day);
    println!("{:?}", std::mem::size_of_val(&d));
    println!("{:?}", std::mem::size_of::<UnvalidatedDate>());
    assert!(std::mem::size_of_val(&d) <= 8);
}

#[test]
fn xx_rightmost_only() {
    // yes
    assert_eq!(
        Date::parse("201X").as_ref().map(Date::precision_certainty),
        Ok((Precision::Decade(2010), Certain))
    );
    assert_eq!(
        Date::parse("20XX"),
        Ok(Date::from_precision(Precision::Century(2000)))
    );
    // same, because we round it
    assert_eq!(
        Date::parse("20XX"),
        Ok(Date::from_precision(Precision::Century(2019)))
    );

    assert_eq!(
        Date::parse("2019-XX"),
        Ok(Date::from_precision(Precision::MonthOfYear(2019)))
    );
    assert_eq!(
        Date::parse("2019-XX-XX"),
        Ok(Date::from_precision(Precision::DayOfYear(2019)))
    );
    assert_eq!(
        Date::parse("2019-07-XX"),
        Ok(Date::from_precision(Precision::DayOfMonth(2019, 7)))
    );
    // no
    assert_eq!(Date::parse("2019-XX-09"), Err(Invalid));
    assert_eq!(Date::parse("201X-XX"), Err(Invalid));
    assert_eq!(Date::parse("20XX-XX"), Err(Invalid));
    assert_eq!(Date::parse("20XX-07"), Err(Invalid));
    assert_eq!(Date::parse("201X-XX-09"), Err(Invalid));
    assert_eq!(Date::parse("201X-07-09"), Err(Invalid));
    assert_eq!(Date::parse("20XX-07-09"), Err(Invalid));
    assert_eq!(Date::parse("20XX-07-XX"), Err(Invalid));
    assert_eq!(Date::parse("20XX-07-0X"), Err(Invalid));
    // Don't think you can reasonably rely on this being Invalid or OutOfRange, it's both
    assert!(Date::parse("2019-XX-00").is_err());
    assert_eq!(Date::parse("2019-0X-00"), Err(Invalid));
    assert_eq!(Date::parse("2019-0X-XX"), Err(Invalid));
}

#[test]
fn no_uncertain_mid_date() {
    // yes
    assert_eq!(
        Date::parse("2019-08-08?"),
        Ok(Date::from_ymd(2019, 8, 8).and_certainty(Uncertain))
    );
    // no
    assert_eq!(Date::parse("2019?-08-08"), Err(ParseError::Invalid));
    assert_eq!(Date::parse("2019-08%-08"), Err(ParseError::Invalid));
    assert_eq!(Date::parse("2019-08?-08%"), Err(ParseError::Invalid));
    assert_eq!(Date::parse("2019?-08-08%"), Err(ParseError::Invalid));
    assert_eq!(Date::parse("2019~-08-08?"), Err(ParseError::Invalid));
    assert_eq!(Date::parse("2019~-08?-08~"), Err(ParseError::Invalid));
    assert_eq!(Date::parse("2019~-08~-08~"), Err(ParseError::Invalid));
}

#[test]
fn xx_with_uncertainty() {
    // yes
    assert!(Date::parse("201X?").is_ok());
    assert!(Date::parse("20XX~").is_ok());
    assert!(Date::parse("20XX%").is_ok());
    assert!(Date::parse("2019-XX?").is_ok());
    assert!(Date::parse("2019-XX~").is_ok());
    assert!(Date::parse("2019-XX%").is_ok());
    assert!(Date::parse("2019-XX-XX?").is_ok());
    assert!(Date::parse("2019-XX-XX~").is_ok());
    assert!(Date::parse("2019-XX-XX%").is_ok());
    assert!(Date::parse("2019-07-XX?").is_ok());
    assert!(Date::parse("2019-07-XX~").is_ok());
    assert!(Date::parse("2019-07-XX%").is_ok());
}

#[test]
fn invalid_calendar_dates() {
    // bad values
    assert_eq!(Date::parse("2019-13"), Err(ParseError::OutOfRange));
    assert_eq!(Date::parse("2019-99"), Err(ParseError::OutOfRange));
    assert_eq!(Date::parse("2019-04-40"), Err(ParseError::OutOfRange));
    assert_eq!(Date::parse("2019-99-99"), Err(ParseError::OutOfRange));
    // bad values inside range of PackedU8
    assert_eq!(Date::parse("2019-00"), Err(ParseError::OutOfRange));
    assert_eq!(Date::parse("2019-30-00"), Err(ParseError::OutOfRange));
    // more zeroes
    assert_eq!(Date::parse("2019-04-00"), Err(ParseError::OutOfRange));
    assert_eq!(Date::parse("2019-00-00"), Err(ParseError::OutOfRange));
    assert_eq!(Date::parse("2019-00-01"), Err(ParseError::OutOfRange));
    // well, year 0 is fine. It's just 1BCE.
    assert_eq!(Date::parse("0000-00-00"), Err(ParseError::OutOfRange));
    assert_eq!(Date::parse("0000-10-00"), Err(ParseError::OutOfRange));
}

#[test]
fn seasons() {
    // yes
    assert!(Date::parse("2019-21").is_ok());
    assert!(Date::parse("2019-22").is_ok());
    assert!(Date::parse("2019-23").is_ok());
    assert!(Date::parse("2019-24").is_ok());
    // no
    assert_eq!(Date::parse("2019-13"), Err(ParseError::OutOfRange));
    assert_eq!(Date::parse("2019-20"), Err(ParseError::OutOfRange));
    assert_eq!(Date::parse("2019-25"), Err(ParseError::OutOfRange));
}

#[test]
fn seasons_day_invalid() {
    assert_eq!(Date::parse("2019-21-05"), Err(ParseError::Invalid));
}

fn scientific_l1() {
    // yes - 5+ digits
    assert_eq!(Edtf::parse("Y157900"), Ok(Edtf::YYear(YYear::raw(157900))),);
    assert_eq!(
        Edtf::parse("Y1234567890"),
        Ok(Edtf::YYear(YYear::raw(1234567890))),
    );
    assert_eq!(
        Edtf::parse("Y-1234567890"),
        Ok(Edtf::YYear(YYear::raw(-1234567890))),
    );
    // no -- <= 4 digits
    assert_eq!(Edtf::parse("Y1745"), Err(ParseError::Invalid));
}

#[test]
fn negative_calendar_dates() {
    // yes
    assert_eq!(
        Edtf::parse("-1900-07-05"),
        Ok(Edtf::Date(Date::from_ymd(-1900, 7, 5)))
    );
    assert_eq!(
        Edtf::parse("-9999-07-05"),
        Ok(Edtf::Date(Date::from_ymd(-9999, 7, 5)))
    );
    assert_eq!(
        Edtf::parse("-0043-07-05"),
        Ok(Edtf::Date(Date::from_ymd(-43, 7, 5)))
    );
    // no - fewer than four digits
    assert_eq!(Edtf::parse("-999-07-05"), Err(ParseError::Invalid));
    // no - negative zero not allowed
    assert_eq!(Edtf::parse("-0000-07-05"), Err(ParseError::Invalid));
}

#[test]
#[ignore = "not sure if we want to support all these yet"]
fn y_year_features() {
    // rejects 4-digit
    assert_eq!(Edtf::parse("Y1234"), Err(Invalid));
    // 5-digit base
    assert_eq!(
        Edtf::parse("Y17000"),
        Ok(Edtf::YYear(YYear::new_opt(17000).unwrap()))
    );

    // full date
    assert_eq!(
        Edtf::parse("Y17000-08-16"),
        Ok(Edtf::Date(Date::from_ymd(17000, 08, 16)))
    );
    //
    // TODO: API to create uncertain/unspecified dates
    //
    // X unspecified digits
    assert_eq!(
        Edtf::parse("Y170XX")
            .unwrap()
            .as_date()
            .unwrap()
            .precision_certainty(),
        (Precision::Century(17000), Certain)
    );
    assert_eq!(
        Edtf::parse("Y17000?")
            .unwrap()
            .as_date()
            .map(|d| d.precision_certainty())
            .unwrap(),
        (Precision::Year(17000), Uncertain)
    );
    // ? uncertainty
    // assert_eq!(Edtf::parse("Y17000?"), Ok(Edtf::Date(Date::from_ymd(17000, 08, 16))));
}
