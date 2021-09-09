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
| `Y1700X`                         | ❌      | ❌             | ✅         | ✅           | ❌            | ❌              | ❌                |
| `Y17000?`                        | ❌      | ❌             | ✅         | ✅           | ❌            | ❌              | ❌                |
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

