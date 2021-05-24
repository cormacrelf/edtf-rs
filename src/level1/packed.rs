use core::marker::PhantomData;
use core::num::NonZeroU8;

// turns out don't need quite as much packing on the month and day, because in struct [32, x], x
// can be up to 32 bits before causing the whole struct's size to bump up beyond 64 bits. So each
// of month and day has 16 bits to use, each value fits in 8 bits, and so each one's flags can have
// 8 bits to itself.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum DMEnum {
    Masked,
    Unmasked(NonZeroU8, Certainty),
}

#[test]
fn test_enumday_size() {
    struct InContext(PackedYear, Option<DMEnum>, Option<DMEnum>);
    assert_eq!(std::mem::size_of::<Option<DMEnum>>(), 2);
    assert_eq!(std::mem::size_of::<InContext>(), 8);
}

impl DMEnum {
    pub(crate) fn value(&self) -> Option<u8> {
        match self {
            Self::Masked => None,
            Self::Unmasked(v, _c) => Some(v.get()),
        }
    }
    pub(crate) fn certainty(&self) -> Option<Certainty> {
        match self {
            Self::Masked => None,
            Self::Unmasked(_v, c) => Some(*c),
        }
    }
    pub(crate) fn is_masked(&self) -> bool {
        match self {
            Self::Masked => true,
            _ => false,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum YearMask {
    /// `2019`
    None = 0b00,
    /// `201X`
    OneDigit = 0b01,
    /// `20XX`
    TwoDigits = 0b10,
}

impl From<u8> for YearMask {
    fn from(bits: u8) -> Self {
        match bits {
            0b00 => YearMask::None,
            0b01 => YearMask::OneDigit,
            0b10 => YearMask::TwoDigits,
            _ => panic!("bit pattern {:b} out of range for YearMaskDigits", bits),
        }
    }
}

/// Represents whether a date part is uncertain and in what way.
/// In EDTF, this is encoded as the `?`, `~` and `%` modifiers.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum Certainty {
    /// no modifier
    Certain = 0b00,
    /// `?`
    Uncertain = 0b01,
    /// `~`
    Approximate = 0b10,
    /// `%`
    ApproximateUncertain = 0b11,
}

impl Certainty {
    /// packs self into two bits.
    fn as_bits_u8(&self) -> u8 {
        *self as u8
    }
    /// packs self into two bits.
    fn as_bits_i32(&self) -> i32 {
        *self as u8 as i32
    }
}

impl From<u8> for Certainty {
    fn from(bits: u8) -> Self {
        match bits {
            0b00 => Self::Certain,
            0b01 => Self::Uncertain,
            0b10 => Self::Approximate,
            0b11 => Self::ApproximateUncertain,
            _ => panic!("bit pattern {:b} out of range for Certainty", bits),
        }
    }
}

// 4 bits total
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct YearFlags {
    pub(crate) certainty: Certainty,
    pub(crate) mask: YearMask,
}

impl YearFlags {
    pub(crate) fn new(certainty: Certainty, mask: YearMask) -> Self {
        Self { certainty, mask }
    }
}
impl From<Certainty> for YearFlags {
    fn from(certainty: Certainty) -> Self {
        Self {
            certainty,
            mask: YearMask::None,
        }
    }
}
impl From<YearMask> for YearFlags {
    fn from(mask: YearMask) -> Self {
        Self {
            certainty: Certainty::Certain,
            mask,
        }
    }
}

impl From<YearFlags> for u8 {
    fn from(yc: YearFlags) -> Self {
        let YearFlags { certainty, mask } = yc;
        let cert = certainty as u8 & 0b11;
        let mask = (mask as u8 & 0b11) << 2;
        cert | mask
    }
}

impl From<u8> for YearFlags {
    fn from(bits: u8) -> Self {
        let c_bits = bits & 0b11;
        let mask_bits = (bits & 0b1100) >> 2;
        Self {
            certainty: Certainty::from(c_bits),
            mask: YearMask::from(mask_bits),
        }
    }
}

pub(crate) trait PackedInt {
    type Inner: Copy;
    type Addendum: Copy;
    fn check_range_ok(inner: Self::Inner) -> bool;
    fn pack_unchecked(inner: Self::Inner, addendum: Self::Addendum) -> Self;
    fn unpack(&self) -> (Self::Inner, Self::Addendum);
    fn pack(inner: Self::Inner, addendum: Self::Addendum) -> Option<Self>
    where
        Self: Sized,
    {
        if !Self::check_range_ok(inner) {
            return None;
        }
        Some(Self::pack_unchecked(inner, addendum))
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(transparent)]
pub(crate) struct PackedYear(pub(crate) i32);

impl PackedInt for PackedYear {
    type Inner = i32;
    type Addendum = YearFlags;
    fn check_range_ok(inner: Self::Inner) -> bool {
        const MAX: i32 = i32::MAX >> 4;
        const MIN: i32 = i32::MIN >> 4;
        inner >= MIN && inner <= MAX
    }
    fn unpack(&self) -> (Self::Inner, Self::Addendum) {
        let inner = self.0 >> 4;
        let addendum = YearFlags::from((self.0 & 0b1111) as u8);
        (inner, addendum)
    }
    fn pack_unchecked(inner: Self::Inner, addendum: Self::Addendum) -> Self {
        let inner = inner << 4;
        let addendum: u8 = addendum.into();
        Self(inner | addendum as i32)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct PackedU8<R>(NonZeroU8, PhantomData<R>);

pub trait U8Range {
    fn includes(int: u8) -> bool;
}
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct MonthSeasonRange;
impl U8Range for MonthSeasonRange {
    fn includes(int: u8) -> bool {
        (int >= 1 && int <= 12) || (int >= 21 && int <= 24)
    }
}
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct DayRange;
impl U8Range for DayRange {
    fn includes(int: u8) -> bool {
        int >= 1 && int <= 31
    }
}

#[test]
fn test_packed_year() {
    use Certainty::*;
    fn roundtrip(a: i32, b: YearFlags) {
        let (aa, bb) = PackedYear::pack(a, b).expect("should be in range").unpack();
        assert_eq!((a, b), (aa, bb));
    }
    roundtrip(1995, Certain.into());
    roundtrip(-1000, Uncertain.into());
    roundtrip(-1000, ApproximateUncertain.into());
    roundtrip(0, ApproximateUncertain.into());
    roundtrip(-1, ApproximateUncertain.into());
}
