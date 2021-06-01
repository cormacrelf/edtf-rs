use core::num::NonZeroU8;

/// Specifies the number of Xs in `2019`/`201X`/`20XX`.
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

impl Default for YearMask {
    fn default() -> Self {
        Self::None
    }
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

/// Represents whether a date part is uncertain and in what way. In EDTF, this is encoded as the
/// `?`, `~` and `%` modifiers.
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

impl Default for Certainty {
    fn default() -> Self {
        Self::Certain
    }
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
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub(crate) struct YearFlags {
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
#[repr(u8)]
pub enum DMMask {
    /// `2019-05`
    None = 0,
    /// `2019-XX`
    Masked = 1,
}

impl Default for DMMask {
    fn default() -> Self {
        Self::None
    }
}

impl From<u8> for DMMask {
    fn from(bits: u8) -> Self {
        match bits {
            0 => DMMask::None,
            1 => DMMask::Masked,
            _ => panic!("bit pattern {:b} out of range for DayMonthMask", bits),
        }
    }
}

// 3 bits total
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct DMFlags {
    pub(crate) certainty: Certainty,
    pub(crate) mask: DMMask,
}

impl DMFlags {
    pub(crate) fn certainty(&self) -> Certainty {
        self.certainty
    }
    pub(crate) fn is_masked(&self) -> bool {
        self.mask != DMMask::None
    }
    pub(crate) fn new(certainty: Certainty, mask: DMMask) -> Self {
        Self { certainty, mask }
    }
}
impl From<Certainty> for DMFlags {
    fn from(certainty: Certainty) -> Self {
        Self {
            certainty,
            mask: DMMask::None,
        }
    }
}
impl From<DMMask> for DMFlags {
    fn from(mask: DMMask) -> Self {
        Self {
            certainty: Certainty::Certain,
            mask,
        }
    }
}

impl From<DMFlags> for u8 {
    fn from(dmc: DMFlags) -> Self {
        let DMFlags { certainty, mask } = dmc;
        let cert = certainty as u8 & 0b11;
        let mask = (mask as u8 & 0b1) << 2;
        cert | mask
    }
}

impl From<u8> for DMFlags {
    fn from(bits: u8) -> Self {
        let c_bits = bits & 0b11;
        let mask_bits = (bits & 0b100) >> 2;
        Self {
            certainty: Certainty::from(c_bits),
            mask: DMMask::from(mask_bits),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct PackedU8(NonZeroU8);

impl PackedInt for PackedU8 {
    type Inner = u8;
    type Addendum = DMFlags;
    fn check_range_ok(inner: Self::Inner) -> bool {
        const MAX: u8 = u8::MAX >> 3;
        const MIN: u8 = 1;
        inner >= MIN && inner <= MAX
    }
    fn unpack(&self) -> (Self::Inner, Self::Addendum) {
        let inner = self.0.get() >> 3;
        let addendum = DMFlags::from((self.0.get() & 0b111) as u8);
        (inner, addendum)
    }
    fn pack_unchecked(inner: Self::Inner, addendum: Self::Addendum) -> Self {
        let inner = inner << 3;
        let addendum: u8 = addendum.into();
        Self(NonZeroU8::new(inner | addendum).unwrap())
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
fn test_packed_month_day() {
    use Certainty::*;
    use DMMask as Mask;
    fn roundtrip(a: u8, b: DMFlags) {
        let (aa, bb) = PackedU8::pack(a, b).expect("should be in range").unpack();
        assert_eq!((a, b), (aa, bb));
    }
    roundtrip(1, Certain.into());
    roundtrip(12, Mask::Masked.into());
    // we can store a day in a PackedU8, since 31 == u8::MAX >> 3.
    // However, we can't store the Level 2 extended season info in there, as those go up to 41.
    // So we'll just use a U16 for that.
    roundtrip(31, Mask::Masked.into());
}

#[test]
fn test_packed_size() {
    use core::mem::size_of;
    assert_eq!(size_of::<PackedYear>(), 4);
    assert_eq!(size_of::<(PackedYear, PackedU8, PackedU8)>(), 8);
}
