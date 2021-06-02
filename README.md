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
- Strict. Rejects everything the specification rejects as a parse error. All the types make it
  impossible to construct an invalid Edtf object.
- Works with [`chrono`](https://lib.rs/chrono) via the optional `features = ["chrono"]`.

### General notes on EDTF

It is probably not stated often enough that EDTF and ISO 8601 support only one calendar, the
[proleptic Gregorian calendar](https://en.wikipedia.org/wiki/Proleptic_Gregorian_calendar).

> *The proleptic Gregorian calendar is produced by extending the Gregorian calendar backward to
the dates preceding its official introduction in 1582.*

Care should be taken whenever writing down dates before its **adoption** by the country in
which a historical record was created. **That means 1800s for many countries!** You may be
unwittingly transcribing a Julian date, or some other calendar, which will be reinterpreted by
EDTF as proleptic Gregorian and be a few days-to-weeks off. See the [Wikipedia list of adoption
dates](https://en.wikipedia.org/wiki/List_of_adoption_dates_of_the_Gregorian_calendar_per_country).
*[this section needs a link to a good Julian converter]*.


### Level 0 example

```rust
use edtf::level_0::Edtf;
let edtf = Edtf::parse("2019-01-07/2020-01").unwrap();
match edtf {
    Edtf::Date(d) => println!("date, {}", d),
    Edtf::Interval(from, to) => {
        println!("interval, {} to {}", from, to);
        println!("(year only: {} to {})", from.year(), to.year());
    }
    Edtf::DateTime(dt) => println!("datetime, {}", dt),
}
// prints:
// interval, 2019-01-07 to 2020-01
// (year only: 2019 to 2020)
```

License: MPL-2.0
