use super::*;

/// # Parsing and accessing contents
impl Edtf {
    /// Parse a Level 1 EDTF.
    pub fn parse(input: &str) -> Result<Self, ParseError> {
        ParsedEdtf::parse_inner(input).and_then(ParsedEdtf::validate)
    }
    /// If self is an [Edtf::Date], return it
    pub fn as_date(&self) -> Option<Date> {
        match self {
            Self::Date(d) => Some(*d),
            _ => None,
        }
    }

    /// If self is an [Edtf::DateTime], return it
    pub fn as_datetime(&self) -> Option<DateTime> {
        match self {
            Self::DateTime(d) => Some(*d),
            _ => None,
        }
    }

    /// Shorthand for matching [Matcher::Interval]
    pub fn as_interval(&self) -> Option<(MatchTerminal, MatchTerminal)> {
        match self.as_matcher() {
            Matcher::Interval(t1, t2) => Some((t1, t2)),
            _ => None,
        }
    }

    /// Return an enum that's easier to match against for generic intervals. See [Matcher] docs for
    /// details.
    pub fn as_matcher(&self) -> Matcher {
        use self::Matcher::*;
        match self {
            Self::Date(d) => Date(d.precision(), d.certainty()),
            Self::DateTime(d) => DateTime(*d),
            Self::YYear(d) => Scientific(d.value()),
            Self::Interval(d, d2) => Interval(d.as_terminal(), d2.as_terminal()),
            Self::IntervalFrom(d, t) => Interval(d.as_terminal(), t.as_match_terminal()),
            Self::IntervalTo(t, d) => Interval(t.as_match_terminal(), d.as_terminal()),
        }
    }
}

/// # Creating a [Date]
///
/// ```
/// use edtf::level_1::{Date, Certainty};
/// let _ = Date::from_ymd(2019, 07, 09).and_certainty(Certainty::Uncertain);
/// ```
impl Date {
    /// Parses a Date from a string. **Note!** This is not part of the EDTF spec. It is
    /// merely a convenience, helpful for constructing proper [Edtf] values programmatically. It
    /// does not handle any of the parts of EDTF using two dates separated by a slash, or date
    /// times.
    ///
    /// ```
    /// use edtf::level_1::Date;
    /// assert_eq!(Date::parse("2019-07-09"), Ok(Date::from_ymd(2019, 07, 09)));
    /// assert_eq!(Date::parse("2019-07"), Ok(Date::from_ymd(2019, 07, 0)));
    ///
    /// assert!(Date::parse("2019-07/2020").is_err());
    /// assert!(Date::parse("2019-00-01").is_err());
    /// assert!(Date::parse("2019-01-01T00:00:00Z").is_err());
    /// ```
    pub fn parse(input: &str) -> Result<Self, ParseError> {
        Self::parse_inner(input).and_then(UnvalidatedDate::validate)
    }

    /// Construct a date with no month or day components, e.g. `2021`. Panics if out of range.
    pub fn from_year(year: i32) -> Self {
        Self::from_ymd(year, 0, 0)
    }

    /// Construct a date with no day component, e.g. `2021-04`. Panics if out of range.
    pub fn from_ym(year: i32, month: u32) -> Self {
        Self::from_ymd(year, month, 0)
    }

    /// Creates a Date from a year, month and day. If month or day are zero, this is treated as if
    /// they have not been specified at all in an EDTF string. So it is invalid to pass `month = 0`
    /// but `day != 0`. This function **panics** on invalid input, including dates that do not
    /// exist, like non-leap-year February 29.
    ///
    /// ```
    /// use edtf::level_1::Date;
    /// assert_eq!(Date::parse("2019-07-09"), Ok(Date::from_ymd(2019, 07, 09)));
    /// assert_eq!(Date::parse("2019-07"), Ok(Date::from_ymd(2019, 07, 0)));
    /// assert_eq!(Date::parse("2019"), Ok(Date::from_ymd(2019, 0, 0)));
    /// ```
    pub fn from_ymd(year: i32, month: u32, day: u32) -> Self {
        UnvalidatedDate::from_ymd(year, month, day)
            .validate()
            .unwrap_or_else(|_| panic!("date not valid: {:04}-{:02}-{:02}", year, month, day))
    }

    /// Creates a Date from a year, month and day. If month or day are zero, this is treated as if
    /// they have not been specified at all in an EDTF string. So it is invalid to pass `month=0`
    /// but `day!=0`. This function **returns None** on invalid input, including dates that do not
    /// exist, like non-leap-year February 29.
    ///
    /// Month is `1..=12` but can also be a [Season] as an integer in range `21..=24`.
    /// ```
    /// use edtf::level_1::Date;
    /// assert_eq!(Date::parse("2019-07-09"), Ok(Date::from_ymd(2019, 07, 09)));
    /// ```
    pub fn from_ymd_opt(year: i32, month: u32, day: u32) -> Option<Self> {
        UnvalidatedDate::from_ymd(year, month, day).validate().ok()
    }

    /// Checks if a year falls inside the acceptable range of years allowed by this library.
    /// This may be a value other than `i32::MIN..=i32::MAX`. It is currently `i32::MIN >> 4 ..
    /// i32::MAX >> 4` to allow for a packed representation.
    pub fn year_in_range(year: i32) -> bool {
        PackedYear::check_range_ok(year)
    }

    /// Get the year. Dates always have one.
    pub fn year(&self) -> i32 {
        let (y, _yf) = self.year.unpack();
        y
    }

    /// Get the season. Dates don't always have one.
    pub fn season(&self) -> Option<Season> {
        let (m, _mf) = self.month?.unpack();
        Season::from_u32_opt(m.into())
    }

    /// Get the month. Dates don't always have one.
    pub fn month(&self) -> Option<Component> {
        Component::from_packed_filter(self.month?, 1..=12)
    }

    /// Get the day. Dates don't always have one.
    pub fn day(&self) -> Option<Component> {
        Some(Component::from_packed(self.day?))
    }

    /// Constructs a generic date from its complete (yyyy-mm-dd) form
    pub fn from_complete(complete: DateComplete) -> Self {
        Self::from_ymd(complete.year(), complete.month(), complete.day())
    }

    /// ```
    /// use edtf::level_1::{Date, Certainty, Precision};
    /// let date = Date::from_precision(Precision::Century(1900))
    ///     .and_certainty(Certainty::Uncertain).to_string();
    /// assert_eq!(date, "19XX?");
    /// ```
    pub fn from_precision(prec: Precision) -> Self {
        Self::from_precision_opt(prec).expect("values out of range in Date::from_precision")
    }

    /// ```
    /// use edtf::level_1::{Date, Certainty, Precision};
    /// let date = Date::from_precision_opt(Precision::DayOfYear(1908));
    /// assert!(date.is_some());
    /// assert_eq!(date, Date::parse("1908-XX-XX").ok());
    /// ```
    pub fn from_precision_opt(prec: Precision) -> Option<Self> {
        use Precision as DP;
        let (y, ym, m, mf, d, df) = match prec {
            DP::Century(x) => (
                helpers::beginning_of_century(x),
                YearMask::TwoDigits,
                0,
                None,
                0,
                None,
            ),
            DP::Decade(x) => (
                helpers::beginning_of_decade(x),
                YearMask::OneDigit,
                0,
                None,
                0,
                None,
            ),
            DP::Year(x) => (x, YearMask::None, 0, None, 0, None),
            DP::Season(x, s) => (x, YearMask::None, s as u32 as u8, None, 0, None),
            DP::Month(x, m) => (x, YearMask::None, m.try_into().ok()?, None, 0, None),
            DP::Day(x, m, d) => (
                x,
                YearMask::None,
                m.try_into().ok()?,
                None,
                d.try_into().ok()?,
                None,
            ),
            DP::MonthOfYear(x) => (x, YearMask::None, 1, Some(DMMask::Unspecified), 0, None),
            DP::DayOfMonth(x, m) => (
                x,
                YearMask::None,
                m.try_into().ok()?,
                None,
                1,
                Some(DMMask::Unspecified),
            ),
            DP::DayOfYear(x) => (
                x,
                YearMask::None,
                1,
                Some(DMMask::Unspecified),
                1,
                Some(DMMask::Unspecified),
            ),
        };
        let year = PackedYear::pack(y, ym.into())?;
        let month = PackedU8::pack(m, mf.unwrap_or_else(Default::default).into());
        let day = PackedU8::pack(d, df.unwrap_or_else(Default::default).into());
        Some(Date {
            year,
            month,
            day,
            certainty: Certainty::Certain,
        })
    }

    /// Returns a new Date with the specified [Certainty]. This certainty applies to the date as a
    /// whole.
    pub fn and_certainty(&self, certainty: Certainty) -> Self {
        Date {
            year: self.year,
            month: self.month,
            day: self.day,
            certainty,
        }
    }

    /// Private.
    fn as_terminal(&self) -> MatchTerminal {
        MatchTerminal::Fixed(self.precision(), self.certainty())
    }

    /// If the date represents a specific day, this returns a [DateComplete] for it.
    ///
    /// That type has more [chrono] integration.
    pub fn complete(&self) -> Option<DateComplete> {
        let (year, yflags) = self.year.unpack();
        let ymask = yflags.mask;
        let (month, mflags) = self.month?.unpack();
        let (day, dflags) = self.day?.unpack();
        if ymask != YearMask::None
            || mflags.mask != DMMask::None
            || dflags.mask != DMMask::None
            || month > 12
        {
            return None;
        }
        Some(DateComplete {
            year,
            // these never fail
            month: NonZeroU8::new(month)?,
            day: NonZeroU8::new(day)?,
        })
    }

    /// [Date::precision] + [Date::certainty]
    pub fn precision_certainty(&self) -> (Precision, Certainty) {
        (self.precision(), self.certainty())
    }

    /// Returns the [Certainty] of this date.
    pub fn certainty(&self) -> Certainty {
        self.certainty
    }

    /// Returns an structure more suited to use with a `match` expression.
    ///
    /// The internal representation of `Date` is packed to reduce its memory footprint, hence this
    /// API.
    pub fn precision(&self) -> Precision {
        let (y, yflags) = self.year.unpack();
        let ym = yflags.mask;
        let precision = match (self.month, self.day) {
            // Only month provided. Could be a season.
            (Some(month), None) => match month.value_u32() {
                Some(m) => {
                    if (1..=12).contains(&m) {
                        Precision::Month(y, m)
                    } else if (21..=24).contains(&m) {
                        Precision::Season(y, Season::from_u32(m as u32))
                    } else {
                        unreachable!("month was out of range")
                    }
                }
                None => Precision::MonthOfYear(y),
            },
            // Both provided, but one or both might be XX (None in the match below)
            (Some(month), Some(day)) => match (month.value_u32(), day.value_u32()) {
                (None, Some(_)) => {
                    unreachable!("date should never hold a masked month with unmasked day")
                }
                (None, None) => Precision::DayOfYear(y),
                (Some(m), None) => Precision::DayOfMonth(y, m),
                (Some(m), Some(d)) => Precision::Day(y, m, d),
            },
            (None, None) => match ym {
                YearMask::None => Precision::Year(y),
                YearMask::OneDigit => Precision::Decade(y),
                YearMask::TwoDigits => Precision::Century(y),
            },
            (None, Some(_)) => unreachable!("date should never hold a day but not a month"),
        };

        precision
    }
}
