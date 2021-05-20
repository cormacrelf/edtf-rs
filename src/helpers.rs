/// Proleptic Gregorian leap year function.
/// From RFC3339 Appendix C.
pub(crate) fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

#[test]
fn leap_year() {
    // yes
    assert!(is_leap_year(2004));
    assert!(is_leap_year(-400));
    assert!(is_leap_year(-204));
    assert!(is_leap_year(0));
    // no
    assert!(!is_leap_year(1));
    assert!(!is_leap_year(100));
    assert!(!is_leap_year(1900));
    assert!(!is_leap_year(1901));
    assert!(!is_leap_year(2005));
    assert!(!is_leap_year(-1));
    assert!(!is_leap_year(-100));
    assert!(!is_leap_year(-200));
    assert!(!is_leap_year(-300));
    assert!(!is_leap_year(-309));
}
