use edtf::level_1::*;

#[test]
fn test_access() {
    let _x = Edtf::Scientific(5);
    let _y = Edtf::Range(Date::from_year(2019), Date::from_year(2020));
}
