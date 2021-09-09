# EDTF Level 1

## Letter-prefixed calendar year ✅

> 'Y' may be used at the beginning of the date string to signify that the date is a year, when
(and only when) the year exceeds four digits, i.e. for years later than 9999 or earlier than
-9999.

- 'Y170000002' is the year 170000002
- 'Y-170000002' is the year -170000002

See notes on [YYear], including parsing comparison with other implementations.

## Seasons ✅

Using Spring=21, Summer=22, Autumn=23, Winter=24.

## Qualification of a date (complete) ✅

> The characters '?', '~' and '%' are used to mean "uncertain", "approximate", and "uncertain"
as well as "approximate", respectively. These characters may occur only at the end of the date
string and apply to the entire date.

## Unspecified digit(s) from the right ✅

> The character 'X' may be used in place of one or more rightmost digits to indicate that the
value of that digit is unspecified, for the following cases:

- `201X`, `20XX`: Year only, one or two digits: `201X`, `20XX`
- `2004-XX`: Year specified, *month unspecified*, month precision: `2004-XX` (different from `2004`, as
  it has month precision but no actual month, whereas `2004` has year precision only)
- `2004-07-XX`: Year and month specified, *day unspecified* in a year-month-day expression (day precision)
- `2004-XX-XX`: Year specified, *day and month unspecified* in a year-month-day expression  (day precision)

## Extended Interval (L1) ✅

- unknown start or end: `/[date]`, `[date]/`
- open interval, (for example 'until date' or 'from date onwards'): `../[date]`, `[date]/..`

## Negative calendar year ✅

`-1740`

