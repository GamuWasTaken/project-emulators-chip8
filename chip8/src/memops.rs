use super::Chip8;

impl<'a, const K: usize> ByteArray<'a, [u8; K]> for Chip8 {}
impl<'a> ByteArray<'a, u8> for Chip8 {}
impl<'a> ByteArray<'a, u16> for Chip8 {}
impl<'a> ByteArray<'a, u32> for Chip8 {}
impl<'a> ByteArray<'a, u64> for Chip8 {}
impl<'a> ByteArray<'a, u128> for Chip8 {}

impl std::ops::Index<std::ops::Range<usize>> for Chip8 {
    type Output = [u8];

    fn index(&self, index: std::ops::Range<usize>) -> &Self::Output {
        &self.0[index]
    }
}
impl std::ops::IndexMut<std::ops::Range<usize>> for Chip8 {
    fn index_mut(&mut self, index: std::ops::Range<usize>) -> &mut Self::Output {
        &mut self.0[index]
    }
}

pub trait ByteArray<'a, T: ByteList<'a, Output: ToSlice<u8>>>:
    std::ops::IndexMut<std::ops::Range<usize>, Output = [u8]>
{
    #[must_use]
    fn read(&'a self, from: impl Into<u16>) -> Option<T> {
        let from = from.into() as usize;
        let range = from..from.checked_add(size_of::<T>())?;
        T::from(&self[range])
    }
    #[must_use]
    fn write(&mut self, data: T, at: impl Into<u16>) -> Option<()> {
        let at = at.into() as usize;
        let at = at..at.checked_add(size_of::<T>())?;
        self[at].copy_from_slice(data.to_list().to_slice());

        Some(())
    }
}

pub trait ByteList<'a>: Sized {
    type Output;
    fn from(data: &'a [u8]) -> Option<Self>;
    fn to_list(self) -> Self::Output;
}

pub trait ToSlice<T> {
    fn to_slice(&self) -> &[T];
}

macro_rules! impl_FromBytes {
    ($t: ty) => {
        impl<'a> ByteList<'a> for $t {
            type Output = [u8; size_of::<$t>()];
            fn from(data: &'a [u8]) -> Option<Self> {
                Some(<$t>::from_be_bytes(
                    data.get(0..size_of::<Self>())?.try_into().ok()?,
                ))
            }
            fn to_list(self) -> Self::Output {
                <$t>::to_be_bytes(self)
            }
        }
    };
}
impl_FromBytes!(u8);
impl_FromBytes!(u16);
impl_FromBytes!(u32);
impl_FromBytes!(u64);
impl_FromBytes!(u128);

impl<'a, const K: usize> ByteList<'a> for [u8; K] {
    type Output = [u8; K];

    fn from(data: &'a [u8]) -> Option<Self> {
        data.try_into().ok()
    }

    fn to_list(self) -> Self::Output {
        self
    }
}
impl<const K: usize> ToSlice<u8> for [u8; K] {
    fn to_slice(&self) -> &[u8] {
        self.as_slice()
    }
}

// TODO maybe make it more compact?
// stitch!([ a, b, c ] as u16)
macro_rules! stitch {
    [$a: expr, $b: expr, $c: expr, $d: expr] => {
        u16::from_be_bytes([stitch![$a, $b], stitch![$c, $d]])
    };
    [$a: expr, $b: expr] => {
        (($a << 4) + $b) as u8
    };
}
pub(crate) use stitch;

pub use Region::*;

// TODO consider impl Index for Chip8 -> self[Memory + reg] = ...
// Better control over access, a place to add checks
#[derive(Debug, Clone, Copy)]
pub enum Region {
    End = 0x1000,     // 0x1000 | End (0)
    Display = 0x0F00, // 0x0F00 | Display (256)
    Empty = 0x0ED8,   // 0x0ED8 | Empty (41)
    KEY = 0x0ED7,     // 0x0ED7 | Key (1)
    ST = 0x0ED6,      // 0x0ED6 | Sound timer (1)
    DT = 0x0ED5,      // 0x0ED5 | Delay timer (1)
    SP = 0x0ED4,      // 0x0ED4 | Stack pointer (1)
    PC = 0x0ED2,      // 0x0ED2 | Program counter (2)
    I = 0x0ED0,       // 0x0ED0 | Address register (2)
    Vs = 0x0EC0,      // 0x0EC0 | V Registers (16)
    Stack = 0x0EA0,   // 0x0EA0 | Call stack (32)
    Memory = 0x0200,  // 0x0200 | Free (3232)
    Data = 0x0000,    // 0x0000 | Internal Data (200)
}

impl From<Region> for u16 {
    fn from(value: Region) -> Self {
        value as u16
    }
}

impl Region {
    pub const fn size(&self) -> usize {
        match self {
            Region::End => 0,
            Region::Display => End as usize - Display as usize,
            Region::Empty => Display as usize - Empty as usize,
            Region::KEY => Empty as usize - KEY as usize,
            Region::ST => KEY as usize - ST as usize,
            Region::DT => ST as usize - DT as usize,
            Region::SP => DT as usize - SP as usize,
            Region::PC => SP as usize - PC as usize,
            Region::I => PC as usize - I as usize,
            Region::Vs => I as usize - Vs as usize,
            Region::Stack => Vs as usize - Stack as usize,
            Region::Memory => Stack as usize - Memory as usize,
            Region::Data => Memory as usize - Data as usize,
        }
    }
}

#[derive(Debug)]
pub struct Offset(Region, u16);
impl Offset {
    pub fn new(region: Region, offset: u16) -> Self {
        assert!(
            region.size() >= offset as usize,
            "Offset overlow: Region '{region:?}' is {} bytes offset is {offset}",
            region.size(),
        );

        Offset(region, offset)
    }
}
impl From<Offset> for u16 {
    fn from(Offset(region, offset): Offset) -> Self {
        region as u16 + offset
    }
}

impl std::ops::Add<u8> for Offset {
    type Output = Offset;

    fn add(self, rhs: u8) -> Self::Output {
        Offset::new(self.0, self.1 + rhs as u16)
    }
}
impl std::ops::Add<u8> for Region {
    type Output = Offset;

    fn add(self, rhs: u8) -> Self::Output {
        Offset::new(self, rhs as u16)
    }
}
impl std::ops::Add<u16> for Region {
    type Output = Offset;

    fn add(self, rhs: u16) -> Self::Output {
        Offset::new(self, rhs)
    }
}
impl std::ops::Add<i32> for Region {
    type Output = Offset;

    fn add(self, rhs: i32) -> Self::Output {
        self + rhs as u16
    }
}
