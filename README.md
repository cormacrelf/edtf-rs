# edtf

![docs.rs](https://docs.rs/edtf/badge.svg)

This crate implements the [Extended Date/Time
Format](https://www.loc.gov/standards/datetime/) as of the 2019-02
specification. It contains separate implementations for each level 0, 1 (and 2,
but not yet). Notes on the choices made in each level are found in in the
module level documentation.

### Installation

```toml
[dependencies]
edtf = "0.1.0"
```

### Features:

- Lossless. Each parsed Edtf can be formatted again to output exactly the same string.
- Strict. Rejects everything the specification rejects as a parse error. All
  the types make it impossible to construct an invalid `Edtf` object, down to the
  leap year.
- Integration with the widely used [chrono](https://lib.rs/chrono) crate via
  the optional `features = ["chrono"]`.
- Three implementations, not one, so you can pick your compatibility level.

### Notes on EDTF and the ISO 8601 calendar system

It is probably not stated often enough that EDTF and ISO 8601 support only one
calendar system, a modified [proleptic Gregorian
calendar](https://en.wikipedia.org/wiki/Proleptic_Gregorian_calendar) with a
year zero. **⚠️ This means your old historical records may have to be converted
from a different calendar system to be used with EDTF accurately.**

Here's a summary for EDTF use:

- Years written with `BCE` are off by one. **1BCE is year 0000 in EDTF. 100BCE
  is -0099 in EDTF.**
- Positive years (aka CE) are correct already.
- Be careful transcribing complete dates written down before the local adoption
  of the Gregorian calendar. **You may be unwittingly transcribing a Julian
  date**, or some other calendar, which will be reinterpreted by EDTF as a
  proleptic Gregorian date and behave incorrectly.
- Check the [local adoption][local-adoption] of the Gregorian calendar where
  the historical record was created. Many western jurisdictions adopted it in
  1582, but e.g. the UK didn't switch over until 1752-09.
- Note that, especially if you're attempting to use EDTF naïvely as a dumb storage area
  for a year/month/day combination, then you may unwittingly write an invalid
  ISO date due to the removal of every 25th or so leap year before its
  adoption. **This library validates that parsed EDTF dates actually exist in
  the ISO calendar.**


Here is an online [Julian \<-\> Gregorian
converter][julian-converter], made by Stephen P. Morse, the man who designed
the Intel 8086 chip. More conversion tools and detailed information about a
selection of calendar systems are available on his awesome
[One Step Webpages][stephen-p-morse] site.

> One useful piece of info on there is that the Julian calendar *year* does not
generally increment on the January 1. In the UK it increments on 25 March,
which used to be the end of the fiscal year there, until they switched to
Gregorian and dropped 11 days in the process. Since then, tax day is April 5!
Julian dates between 1 January and 25 March are often 'dual dated', i.e.:
>
> > ... two years separated by a slash. The first year was the year in the
> > Julian calendar in use in that locality, and the second was the year that
> > it would have been if they had changed the year number on January 1.
>
> E.g. *19 February 1683/1684*. Careful, EDTF hadn't been invented yet, so that's
not EDTF!


[local-adoption]: https://en.wikipedia.org/wiki/List_of_adoption_dates_of_the_Gregorian_calendar_per_country
[julian-converter]: https://stevemorse.org/jcal/julian.html
[stephen-p-morse]: https://stevemorse.org/#calendar

### Example usage of the `edtf` crate

#### Level 0

```rust
use edtf::level_0::Edtf;
let edtf = Edtf::parse("2019-01-07/2020-01").unwrap();
match edtf {
    Edtf::Date(d) => println!("date: {}", d),
    Edtf::Interval(from, to) => {
        println!("interval: {} to {}", from, to);
        println!("year parts: {} to {}", from.year(), to.year());
    }
    Edtf::DateTime(dt) => println!("datetime: {}", dt),
}
// prints:
// interval: 2019-01-07 to 2020-01
// year parts: 2019 to 2020
```

#### Level 1

```rust
```

License: MPL-2.0
