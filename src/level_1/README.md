# EDTF Level 1

## Letter-prefixed calendar year ✅

> 'Y' may be used at the beginning of the date string to signify that the date is a year, when
(and only when) the year exceeds four digits, i.e. for years later than 9999 or earlier than
-9999.

- 'Y170000002' is the year 170000002
- 'Y-170000002' is the year -170000002

### Notes:

It is unclear how many other features should be supported in `Y`-years. The spec is pretty
quiet on this. The main reason in favour of adding a bunch of features is that `Y`-years are
called "date", and the "date" concept is reused all over the place. Here's some pro/con
analysis of adding features:

- Can they be followed by a month and day/season?
  - Probably not, because the spec says '*to signify that the date is a year*'. Also who cares
  whether 10,000BC was a Thursday?
- Can they take `X/XX` unspecified digits?
  - In Level 2 there is already the significant digits functionality, which kinda covers this
  via `S1`/`S2`. So probably not.
- Can they have a `?~%` uncertainty attached?
  - If you're talking about 10,000BC, it is rare that you could actually be certain. But that
  only makes it logical that the additional uncertainty flags are not actually necessary.
- Can they be put in ranges?
  - Absolutely no reason why not. In fact this is probably *the* most useful feature for them.
  Plus, years in L2 can have significant digits, which is shorthand for making a special kind
  of range with an estimate. **Leaning yes.**
- or L2 sets?
  - No great reasons for/against. But those sets are designed for enumerating specific
  years/months/days, which is not useful for Y-years because they are typically so inaccurate.


This table lists compatibility with other implementations as of 2021-05-26.


| Implementation                   | Rust    | [validator][v] | [PHP][php] | [Dart][dart] | [edtf.js][js] | [edtf-ruby][rb] | [python-edtf][py] |
| ---                              | --      | --             | --         | --           | --            | --              | --                |
| Last Updated                     |         | 2020-11        | 2021-05    | 2019         | 2020-11       | 2020-11         | 2018-06           |
| Draft version supported          | 2019-02 | 2019-02        | 2019-02    | 2019-02      | 2019-02       | 2012 ⚠️          | 2012 ⚠️            |
| More info                        |         | [info][vh]     |            |              |               |                 |                   |
| Rejects 4-digit `Y1234`          | ✅      | ✅             | ❌         | ❌           | ✅            | ✅              | ✅                |
| `Y17000`, `Y-17000` (base)       | ✅      | ✅             | ✅         | ✅           | ✅            | ✅              | ✅                |
| `Y17000-08-18`                   | ❌      | ❌             | ✅         | ✅           | ❌            | ❌              | ❌                |
| `Y1700X`                         | 🧐      | ❌             | ✅         | ✅           | ❌            | ❌              | ❌                |
| `Y17000?`                        | 🧐      | ❌             | ✅         | ✅           | ❌            | ❌              | ❌                |
| `Y-17000/2003`, `Y17000/..` etc. | 🧐      | ❌             | ✅         | ✅           | ❌            | ❌              | ❌                |
| `[Y17000..]`, etc.               | 🧐      | ❌             | ✅         | ✅           | ❌            | ❌              | ❌                |


[v]: https://digital2.library.unt.edu/edtf/
[vh]: https://library.unt.edu/digital-projects-unit/metadata/fields/date/
[php]: https://github.com/ProfessionalWiki/EDTF
[dart]: https://github.com/maalexy/edtf
[js]: https://npmjs.com/package/edtf/
[rb]: https://rubygems.org/gems/edtf/
[py]: https://pypi.org/project/edtf/

Test suites: [php](https://github.com/ProfessionalWiki/EDTF/blob/c0f54c0c8dff3c00f9b32ea3e773315d6a5f2c9e/tests/Functional/Level1/PrefixedYearTest.php),
[js]()
[rb](https://github.com/inukshuk/edtf-ruby/blob/7ee86d81ddb7d6503d5b282a409eb43e51f27186/spec/edtf/parser_spec.rb#L74-L80),
[py](https://github.com/ixc/python-edtf/blob/3bff48427b9f1452fcc030e1cc30e4e6808febc5/edtf/parser/tests.py#L101-L103) but [considers `y17e7-12-26` to be "not implemented"](https://github.com/ixc/python-edtf/blob/3bff48427b9f1452fcc030e1cc30e4e6808febc5/edtf/parser/tests.py#L195) rather than not part of the spec.

*⚠️: The 2012 draft uses the old `y12345` syntax.*

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

