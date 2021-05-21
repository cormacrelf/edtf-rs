use core::marker::PhantomData;
use core::num::NonZeroU8;

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

#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct PackedYear(i32);

impl PackedInt for PackedYear {
    type Inner = i32;
    type Addendum = Certainty;
    fn check_range_ok(inner: Self::Inner) -> bool {
        // 536870911
        const MAX: i32 = i32::MAX >> 2;
        // i32 can obviously be negative.
        // with two's complement, this number is actually -536870912
        const MIN: i32 = i32::MIN >> 2;
        inner >= MIN && inner <= MAX
    }
    fn unpack(&self) -> (Self::Inner, Self::Addendum) {
        let inner = self.0 >> 2;
        let addendum = Certainty::from((self.0 & 0b11) as u8);
        (inner, addendum)
    }
    fn pack_unchecked(inner: Self::Inner, addendum: Self::Addendum) -> Self {
        let inner = inner << 2;
        Self(inner | addendum.as_bits_i32())
    }
}

#[derive(Debug, Copy, Clone)]
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
    type Addendum = Certainty;
    fn check_range_ok(inner: Self::Inner) -> bool {
        // make sure it's nonzero
        inner > 0 && R::includes(inner)
    }
    fn unpack(&self) -> (Self::Inner, Self::Addendum) {
        let inner = self.0.get() >> 2;
        let addendum = Certainty::from((self.0.get() & 0b11) as u8);
        (inner, addendum)
    }
    fn pack_unchecked(inner: Self::Inner, addendum: Self::Addendum) -> Self {
        let inner = inner << 2;
        Self(NonZeroU8::new(inner | addendum.as_bits_u8()).unwrap(), Default::default())
    }
}

#[test]
fn test_packed_year() {
    use Certainty::*;
    fn roundtrip(a: i32, b: Certainty) {
        let (aa, bb) = PackedYear::pack(a, b).expect("should be in range").unpack();
        assert_eq!((a, b), (aa, bb));
    }
    roundtrip(1995, Certain);
    roundtrip(-1000, Uncertain);
    roundtrip(-1000, ApproximateUncertain);
    roundtrip(0, ApproximateUncertain);
    roundtrip(-1, ApproximateUncertain);
}

#[test]
fn test_packed_month() {
    use Certainty::*;
    fn roundtrip(a: u8, b: Certainty) {
        let (aa, bb) = PackedU8::<MonthRange>::pack(a, b).expect("should be in range").unpack();
        assert_eq!((a, b), (aa, bb));
    }
    roundtrip(1, Certain);
    roundtrip(12, Uncertain);
    roundtrip(21, ApproximateUncertain);
}
