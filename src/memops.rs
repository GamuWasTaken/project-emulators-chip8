use std::ops::{Index, IndexMut, Range};

use super::Chip8;

// TODO maybe make it more compact?
// stitch!([ a, b, c ] as u16)
macro_rules! stitch {
    [$a: expr, $b: expr, $c: expr, $d: expr] => {
        [stitch![$a, $b], stitch![$c, $d]]
    };
    [$a: expr, $b: expr] => {
        (($a << 4) + $b) as u8
    };
}
pub(crate) use stitch;
// self[Memory + (0..program.len())].copy_from_slice(program);
// copy!(program => self, Memory at 0);
macro_rules! copy {
    ($from: expr => $self: ident at $offset: expr) => {
        $self.0[($offset as usize)..($offset as usize + $from.len())].copy_from_slice($from)
    };
    ($from: expr => $self: ident $target: ident at $offset: expr) => {
        $self[$target + ($offset..($offset + $from.len()))].copy_from_slice($from)
    };
    ($from: expr => $self: ident $target: ident) => {
        copy!($from => $self $target at 0)
        // $self[$target + (0..$from.len())].copy_from_slice($from)
    };
}
pub(crate) use copy;

macro_rules! read {
    (u64 from $self: ident $target: ident at $offset: expr) => {
        u64::from_be_bytes([
            $self[$target + $offset as u16 + 0],
            $self[$target + $offset as u16 + 1],
            $self[$target + $offset as u16 + 2],
            $self[$target + $offset as u16 + 3],
            $self[$target + $offset as u16 + 4],
            $self[$target + $offset as u16 + 5],
            $self[$target + $offset as u16 + 6],
            $self[$target + $offset as u16 + 7],
        ])
    };
    (u64 from $self: ident $target: ident) => { read!(u64 from $self $target at 0u16) };
    (u128 from $self: ident $target: ident at $offset: expr) => {
        u128::from_be_bytes([
            $self[$target + $offset as u16 + 0],
            $self[$target + $offset as u16 + 1],
            $self[$target + $offset as u16 + 2],
            $self[$target + $offset as u16 + 3],
            $self[$target + $offset as u16 + 4],
            $self[$target + $offset as u16 + 5],
            $self[$target + $offset as u16 + 6],
            $self[$target + $offset as u16 + 7],
            $self[$target + $offset as u16 + 8],
            $self[$target + $offset as u16 + 9],
            $self[$target + $offset as u16 + 10],
            $self[$target + $offset as u16 + 11],
            $self[$target + $offset as u16 + 12],
            $self[$target + $offset as u16 + 13],
            $self[$target + $offset as u16 + 14],
            $self[$target + $offset as u16 + 15],
        ])
    };
    (u128 from $self: ident $target: ident) => { read!(u128 from $self $target at 0u16) };
    (u16 from $self: ident $target: ident at $offset: expr) => {
        u16::from_be_bytes([$self[$target + $offset + 0], $self[$target + $offset + 1]])
    };
    (u16 from $self: ident $target: ident) => { read!(u16 from $self $target at 0u16) };

    (u8 from $self: ident $target: ident at $offset: expr) => { $self[$target + $offset] as u8 };
    (u8 from $self: ident $target: ident) => { read!(u8 from $self $target at 0u16) };
}

pub(crate) use read;

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
impl From<Offset> for usize {
    fn from(Offset(region, offset): Offset) -> Self {
        region as usize + offset as usize
    }
}

impl std::ops::Add<u8> for Offset {
    type Output = Offset;

    fn add(self, rhs: u8) -> Self::Output {
        Offset::new(self.0, self.1 + rhs as u16)
    }
}
// Index by u16
impl std::ops::Index<u16> for Chip8 {
    type Output = u8;

    fn index(&self, index: u16) -> &Self::Output {
        &self.0[index as usize]
    }
}
// Index by Region + u16
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
impl std::ops::Index<Offset> for Chip8 {
    type Output = u8;

    fn index(&self, offset: Offset) -> &Self::Output {
        &self.0[usize::from(offset)]
    }
}
impl std::ops::IndexMut<Offset> for Chip8 {
    fn index_mut(&mut self, offset: Offset) -> &mut Self::Output {
        &mut self.0[usize::from(offset)]
    }
}

// Index by Region
impl std::ops::Index<Region> for Chip8 {
    type Output = u8;

    fn index(&self, index: Region) -> &Self::Output {
        &self.0[index as usize]
    }
}
impl std::ops::IndexMut<Region> for Chip8 {
    fn index_mut(&mut self, index: Region) -> &mut Self::Output {
        &mut self.0[index as usize]
    }
}

// Index by Region + Range
impl std::ops::Add<Range<usize>> for Region {
    type Output = Range<Offset>;

    fn add(self, rhs: Range<usize>) -> Self::Output {
        (self + rhs.start as u16)..(self + rhs.end as u16)
    }
}
impl Index<Range<Offset>> for Chip8 {
    type Output = [u8];

    fn index(&self, Range { start, end }: Range<Offset>) -> &Self::Output {
        &self.0[usize::from(start)..usize::from(end)]
    }
}
impl IndexMut<Range<Offset>> for Chip8 {
    fn index_mut(&mut self, Range { start, end }: Range<Offset>) -> &mut Self::Output {
        &mut self.0[usize::from(start)..usize::from(end)]
    }
}
