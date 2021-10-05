use super::*;
use chrono::NaiveTime;

/// # Sort orders
///
/// Ord is not implemented on Edtf, because Edtf's Eq and Hash implementations usefully
/// differentiate between e.g. `2010-08-12` and `2010-08-12T00:00:00Z`, and sorting Edtfs on
/// by the points in time they represent is incompatible with that. Edtf's Eq/Hash
/// implementations should be thought of as being roughly equivalent to a string comparison on
/// the underlying EDTF format, whereas this function sorts on an actual timeline.
///
/// All the sort orders here convert any datetimes to UTC, including "no datetime" which is just
/// presumed to be UTC.
///
impl Edtf {
    /// A type that is sortable in a reasonable way, first using [Edtf::sort_order_start] and tie-breaking
    /// with [Edtf::sort_order_end]. Use with `Vec::sort_by_key` etc.
    ///
    /// ```
    /// use edtf::level_1::Edtf;
    /// let a = Edtf::parse("2009-08").unwrap();
    /// let b = Edtf::parse("2009/2010").unwrap();
    /// let c = Edtf::parse("2008/2012").unwrap();
    /// let d = Edtf::parse("../2011").unwrap();
    /// let e = Edtf::parse("2008/2011").unwrap();
    /// let mut edtfs = vec![a, b, c, d, e];
    /// // you can't sort this directly
    /// // edtfs.sort();
    /// edtfs.sort_by_key(|a| a.sort_order());
    /// assert_eq!(edtfs, vec![d, e, c, b, a])
    /// ```
    pub fn sort_order(&self) -> SortOrder {
        SortOrder(*self)
    }

    /// A sort order that sorts by the EDTF's start point. Use with `Vec::sort_by_key` etc.
    ///
    /// An open range on the left is considered to be negative infinity.
    ///
    /// ```
    /// use edtf::level_1::Edtf;
    /// let a = Edtf::parse("2009-08").unwrap();
    /// let b = Edtf::parse("2009/2010").unwrap();
    /// let c = Edtf::parse("2008/2012").unwrap();
    /// let d = Edtf::parse("../2011").unwrap();
    /// let e = Edtf::parse("2008/2011").unwrap();
    /// let mut edtfs = vec![a, b, c, d, e];
    /// edtfs.sort_by_key(|a| a.sort_order_start());
    /// assert_eq!(edtfs, vec![d, c, e, b, a])
    /// ```
    pub fn sort_order_start(&self) -> SortOrderStart {
        SortOrderStart(edtf_start_date(self))
    }

    /// A sort order that sorts by the EDTF's end point. Use with `Vec::sort_by_key` etc.
    ///
    /// An open range on the right is considered to be infinity.
    ///
    /// ```
    /// use edtf::level_1::Edtf;
    /// let a = Edtf::parse("2009-08").unwrap();
    /// let b = Edtf::parse("2009/2010").unwrap();
    /// let c = Edtf::parse("2008/2012").unwrap();
    /// let d = Edtf::parse("../2011").unwrap();
    /// let e = Edtf::parse("2008/2011").unwrap();
    /// let mut edtfs = vec![a, b, c, d, e];
    /// edtfs.sort_by_key(|a| a.sort_order_end());
    /// assert_eq!(edtfs, vec![a, b, d, e, c])
    /// ```
    pub fn sort_order_end(&self) -> SortOrderEnd {
        SortOrderEnd(edtf_end_date(self))
    }
}

/// See [Edtf::sort_order_start]
#[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
#[cfg(feature = "chrono")]
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct SortOrderStart(Infinite);

/// See [Edtf::sort_order_end]
#[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
#[cfg(feature = "chrono")]
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct SortOrderEnd(Infinite);

/// See [Edtf::sort_order]
#[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
#[cfg(feature = "chrono")]
pub struct SortOrder(Edtf);

impl Ord for SortOrder {
    fn cmp(&self, other: &Self) -> Ordering {
        sort_order(&self.0, &other.0)
    }
}

impl PartialOrd for SortOrder {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for SortOrder {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl Eq for SortOrder {}

fn sort_order(a: &Edtf, b: &Edtf) -> Ordering {
    edtf_start_date(a)
        .cmp(&edtf_start_date(b))
        .then_with(|| edtf_end_date(a).cmp(&edtf_end_date(b)))
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Infinite {
    NegativeInfinity,
    YYearNegative(i64),
    Value(Date, NaiveTime),
    YYearPositive(i64),
    Infinity,
}

impl From<Date> for Infinite {
    fn from(d: Date) -> Self {
        Self::Value(d, NaiveTime::from_num_seconds_from_midnight(0, 0))
    }
}

fn edtf_start_date(edtf: &Edtf) -> Infinite {
    use self::Infinite::*;
    match *edtf {
        Edtf::Date(d) => d.into(),
        Edtf::Interval(d, _) => d.into(),
        Edtf::IntervalFrom(d, _) => d.into(),
        // this sorts first, which makes sense
        Edtf::IntervalTo(_, _) => NegativeInfinity,
        Edtf::DateTime(d) => {
            let tz = chrono::Utc;
            let dt = d.to_chrono(&tz);
            let date = dt.date().naive_utc();
            let time = dt.time();
            Value(date.into(), time)
        }
        Edtf::YYear(y) => {
            let val = y.value();
            if val < i32::MIN as i64 {
                YYearNegative(val)
            } else if val > i32::MAX as i64 {
                YYearPositive(val)
            } else {
                let v32 = val as i32;
                if !Date::year_in_range(v32) {
                    if v32 < 0 {
                        YYearNegative(val)
                    } else {
                        YYearPositive(val)
                    }
                } else {
                    Date::from_ymd(v32, 0, 0).into()
                }
            }
        }
    }
}

fn edtf_end_date(edtf: &Edtf) -> Infinite {
    use self::Infinite::*;
    match *edtf {
        Edtf::Date(_) | Edtf::DateTime(_) | Edtf::YYear(_) => edtf_start_date(edtf),
        Edtf::Interval(_, d) => d.into(),
        Edtf::IntervalFrom(_, _) => Infinity,
        Edtf::IntervalTo(_, d) => d.into(),
    }
}

#[cfg(test)]
fn cmp(a: &str, b: &str) -> Ordering {
    sort_order(&Edtf::parse(a).unwrap(), &Edtf::parse(b).unwrap())
}

#[test]
fn test_cmp_single() {
    assert_eq!(cmp("2009", "2010"), Ordering::Less);
    assert_eq!(cmp("2011", "2010"), Ordering::Greater);
    assert_eq!(cmp("2010", "2010"), Ordering::Equal);
    assert_eq!(cmp("2010-08", "2010"), Ordering::Greater);
    assert_eq!(cmp("2010-08", "2010-09"), Ordering::Less);
    assert_eq!(cmp("2010-08", "2010-08"), Ordering::Equal);
}

#[test]
fn test_cmp_single_interval() {
    assert_eq!(cmp("2009", "2010/2011"), Ordering::Less);
    assert_eq!(cmp("2011", "2009/2011"), Ordering::Greater);
    assert_eq!(cmp("2010", "2010/2010"), Ordering::Equal);
    assert_eq!(cmp("2010", "2010/2011"), Ordering::Less);
    assert_eq!(cmp("2010-08", "2010/2011"), Ordering::Greater);
}

#[test]
fn test_cmp_double_interval() {
    // we compare first on the LHS terminal, and then tie break with the RHS
    // 2009    2010    2011    2012    2013
    // |---------------|
    //         |-------|
    assert_eq!(cmp("2009/2011", "2010/2011"), Ordering::Less);
    // |---------------|
    //                 |-------|
    assert_eq!(cmp("2009/2011", "2011/2012",), Ordering::Less);
    // |---------------|
    //         |---------------|
    assert_eq!(cmp("2009/2011", "2010/2012"), Ordering::Less);
    // |---------------|
    //                         |-------|
    assert_eq!(cmp("2009/2011", "2012/2013"), Ordering::Less);
    // |---------------|
    //     |-------|
    assert_eq!(cmp("2009/2011", "2009-03/2010-07"), Ordering::Less);
}

#[test]
fn test_cmp_double_interval_open() {
    // the LHS terminal being .. means it starts at the beginning of time itself, beats everything
    // 2009    2010    2011    2012    2013
    // ----------------|
    //         |-------|
    assert_eq!(cmp("../2011", "2010/2011"), Ordering::Less);
    // ----------------|
    // ----------------|
    assert_eq!(cmp("../2011", "../2011"), Ordering::Equal);
    // ----------------|
    //         |-------|
    assert_eq!(cmp("../2011", "2010/2011"), Ordering::Less);
    // and now for the RHS being open
    //
    //         |---------------
    //         |-------|
    assert_eq!(cmp("2010/..", "2010/2011",), Ordering::Greater);
}

#[test]
fn test_cmp_yyear() {
    assert_eq!(cmp("Y-10000", "2010"), Ordering::Less);
    assert_eq!(cmp("Y10000", "2010"), Ordering::Greater);
    assert_eq!(cmp("Y10000", "2010/.."), Ordering::Greater);
    assert_eq!(cmp("Y-10000", "2010/.."), Ordering::Less);
    assert_eq!(cmp("Y-10000", "../2010"), Ordering::Greater);
}

#[test]
fn test_cmp_datetime() {
    assert_eq!(cmp("2010-08-12T00:00:00Z", "2010-08-12"), Ordering::Equal);
    assert_eq!(
        cmp("2010-08-12T00:00:00Z", "2010-08-12T00:00:05Z"),
        Ordering::Less
    );
    assert_eq!(
        cmp("2010-08-12T00:00:00Z", "2010-08-12T00:00:00-01:00"),
        Ordering::Less
    );
    // the first one is on the 12th in the -01:00 timezone, but convert it to UTC and it's 50
    // minutes past midnight on the 13th.
    assert_eq!(
        cmp("2010-08-12T23:50:00-01:00", "2010-08-13"),
        Ordering::Greater
    );
}
