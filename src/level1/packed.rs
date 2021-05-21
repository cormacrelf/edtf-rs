use core::marker::PhantomData;
use core::num::NonZeroU8;

/// /////////////////
/// TODO : use this for days and months instead, because it's still u16 sized and that is
/// acceptable when you're adding two of these to an i32 field. Doesn't need to be packed into 8
/// bits.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DMEnum {
    Masked,
    Unmasked(NonZeroU8, Certainty)
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
    One = 0b01,
    /// `20XX`
    Two = 0b10,
}

impl From<u8> for YearMask {
    fn from(bits: u8) -> Self {
        match bits {
            0b00 => YearMask::None,
            0b01 => YearMask::One,
            0b10 => YearMask::Two,
            _ => panic!("bit pattern {:b} out of range for YearMaskDigits", bits),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum DayMonthMask {
    /// `2019-05`
    None = 0,
    /// `2019-XX`
    Masked = 1,
}

impl From<u8> for DayMonthMask {
    fn from(bits: u8) -> Self {
        match bits {
            0 => DayMonthMask::None,
            1 => DayMonthMask::Masked,
            _ => panic!("bit pattern {:b} out of range for DayMonthMask", bits),
        }
    }
}

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
    pub fn new(certainty: Certainty, mask: YearMask) -> Self {
        Self { certainty, mask }
    }
}
impl From<Certainty> for YearFlags {
    fn from(certainty: Certainty) -> Self {
        Self { certainty, mask: YearMask::None }
    }
}
impl From<YearMask> for YearFlags {
    fn from(mask: YearMask) -> Self {
        Self { certainty: Certainty::Certain, mask }
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

// 3 bits total
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct DayMonthCertainty {
    pub(crate) certainty: Certainty,
    pub(crate) mask: DayMonthMask,
}

impl DayMonthCertainty {
    pub fn new(certainty: Certainty, mask: DayMonthMask) -> Self {
        Self { certainty, mask }
    }
}
impl From<Certainty> for DayMonthCertainty {
    fn from(certainty: Certainty) -> Self {
        Self { certainty, mask: DayMonthMask::None }
    }
}
impl From<DayMonthMask> for DayMonthCertainty {
    fn from(mask: DayMonthMask) -> Self {
        Self { certainty: Certainty::Certain, mask }
    }
}

impl From<DayMonthCertainty> for u8 {
    fn from(dmc: DayMonthCertainty) -> Self {
        let DayMonthCertainty { certainty, mask } = dmc;
        let cert = certainty as u8 & 0b11;
        let mask = (mask as u8 & 0b1) << 2;
        cert | mask
    }
}

impl From<u8> for DayMonthCertainty {
    fn from(bits: u8) -> Self {
        let c_bits = bits & 0b11;
        let mask_bits = (bits & 0b100) >> 2;
        Self {
            certainty: Certainty::from(c_bits),
            mask: DayMonthMask::from(mask_bits),
        }
    }
}

pub trait PackedInt {
    type Inner: Copy;
    type Addendum: Copy;
    fn check_range_ok(inner: Self::Inner) -> bool;
    fn pack_unchecked(inner: Self::Inner, addendum: Self::Addendum) -> Self;
    fn unpack(&self) -> (Self::Inner, Self::Addendum);
    fn pack(inner: Self::Inner, addendum: Self::Addendum) -> Option<Self> where Self: Sized {
        if !Self::check_range_ok(inner) {
            return None;
        }
        Some(Self::pack_unchecked(inner, addendum))
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct PackedYear(i32);

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

pub type PackedDay = PackedU8<DayRange>;
pub type PackedMonth = PackedU8<MonthRange>;


pub trait U8Range {
    fn includes(int: u8) -> bool;
}
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct MonthRange;
impl U8Range for MonthRange {
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

impl<R: U8Range> PackedInt for PackedU8<R> {
    type Inner = u8;
    type Addendum = DayMonthCertainty;
    fn check_range_ok(inner: Self::Inner) -> bool {
        // make sure it's nonzero and not too big for the u8 as a whole
        inner > 0 && inner <= u8::MAX >> 3 && R::includes(inner)
    }
    fn unpack(&self) -> (Self::Inner, Self::Addendum) {
        let inner = self.0.get() >> 3;
        let addendum = DayMonthCertainty::from((self.0.get() & 0b111) as u8);
        (inner, addendum)
    }
    fn pack_unchecked(inner: Self::Inner, addendum: Self::Addendum) -> Self {
        let inner = inner << 3;
        let addendum: u8 = addendum.into();
        Self(NonZeroU8::new(inner | addendum).unwrap(), Default::default())
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

#[test]
fn test_packed_month() {
    use Certainty::*;
    use DayMonthMask as Mask;
    fn roundtrip(a: u8, b: DayMonthCertainty) {
        let (aa, bb) = PackedU8::<MonthRange>::pack(a, b).expect("should be in range").unpack();
        assert_eq!((a, b), (aa, bb));
    }
    roundtrip(1, Certain.into());
    roundtrip(12, Mask::Masked.into());
}
