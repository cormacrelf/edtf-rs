use core::fmt;
use std::marker::PhantomData;
use std::str::FromStr;

use crate::level_0 as l0;
use crate::level_1 as l1;
// use crate::level_2 as l2;
use crate::ParseError;

use serde::de::{self, Deserialize};
use serde::ser::{self, Serialize};

struct Helper<S>(PhantomData<S>);

impl<'de, S> de::Visitor<'de> for Helper<S>
where
    S: core::fmt::Display,
    S: FromStr<Err = ParseError>,
{
    type Value = S;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "an EDTF string")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        value.parse::<Self::Value>().map_err(de::Error::custom)
    }
}

macro_rules! impl_serde {
    ($t:ty) => {
        impl<'de> Deserialize<'de> for $t {
            fn deserialize<D>(deserializer: D) -> Result<Self, <D as de::Deserializer<'de>>::Error>
            where
                D: de::Deserializer<'de>,
            {
                deserializer.deserialize_str(Helper(PhantomData))
            }
        }

        impl Serialize for $t {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: ser::Serializer,
            {
                serializer.collect_str(self)
            }
        }
    };
}

impl_serde!(l0::Edtf);
impl_serde!(l1::Edtf);
// impl_serde!(l2::Edtf);

#[test]
fn test_serde() {
    use serde_test::{assert_tokens, Token};
    let edtf0: l0::Edtf = l0::Edtf::Date(l0::Date::from_ymd(2021, 04, 00));
    assert_tokens(&edtf0, &[Token::String("2021-04")]);
    let edtf1: l1::Edtf = l1::Edtf::Date(l1::Date::from_ymd(2021, 04, 00));
    assert_tokens(&edtf1, &[Token::String("2021-04")]);
}
