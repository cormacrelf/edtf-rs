use edtf::level_1::*;

#[test]
fn test_access() {
    let _x = YYear::new_or_cal(5).map(Edtf::YYear);
    let _y = Edtf::Interval(Date::from_year(2019), Date::from_year(2020));
}
