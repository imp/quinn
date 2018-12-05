use std::ops;

use bytes::{Buf, BufMut};

use byteorder::{BigEndian, ByteOrder};

use crate::coding::{Codec, UnexpectedEnd};

//  +------+--------+-------------+-----------------------+
//  | 2Bit | Length | Usable Bits | Range                 |
//  +------+--------+-------------+-----------------------+
//  | 00   | 1      | 6           | 0-63                  |
//  |      |        |             |                       |
//  | 01   | 2      | 14          | 0-16383               |
//  |      |        |             |                       |
//  | 10   | 4      | 30          | 0-1073741823          |
//  |      |        |             |                       |
//  | 11   | 8      | 62          | 0-4611686018427387903 |
//  +------+--------+-------------+-----------------------+

const ONE_OCTET_MAX: u64 = 63;
const TWO_OCTETS_MIN: u64 = ONE_OCTET_MAX + 1;
const TWO_OCTETS_MAX: u64 = 16383;
const FOUR_OCTETS_MIN: u64 = TWO_OCTETS_MAX + 1;
const FOUR_OCTETS_MAX: u64 = 1_073_741_823;
const EIGHT_OCTETS_MIN: u64 = FOUR_OCTETS_MAX + 1;
const EIGHT_OCTETS_MAX: u64 = 4_611_686_018_427_387_903;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct VarInt(u64);

impl VarInt {
    pub fn size(&self) -> usize {
        match self.0 {
            0...ONE_OCTET_MAX => 1,
            TWO_OCTETS_MIN...TWO_OCTETS_MAX => 2,
            FOUR_OCTETS_MIN...FOUR_OCTETS_MAX => 4,
            EIGHT_OCTETS_MIN...EIGHT_OCTETS_MAX => 8,
            _ => unreachable!(),
        }
    }
}

impl From<VarInt> for u64 {
    fn from(varint: VarInt) -> Self {
        varint.0
    }
}

impl From<VarInt> for usize {
    fn from(varint: VarInt) -> Self {
        varint.0 as usize
    }
}

impl From<u8> for VarInt {
    fn from(int: u8) -> Self {
        VarInt(u64::from(int))
    }
}

impl From<u16> for VarInt {
    fn from(int: u16) -> Self {
        VarInt(u64::from(int))
    }
}

impl From<u32> for VarInt {
    fn from(int: u32) -> Self {
        VarInt(u64::from(int))
    }
}

impl From<u64> for VarInt {
    fn from(int: u64) -> Self {
        debug_assert!(int <= EIGHT_OCTETS_MAX);
        VarInt(int)
    }
}
impl From<usize> for VarInt {
    fn from(int: usize) -> Self {
        Self::from(int as u64)
    }
}

impl ops::Add for VarInt {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let sum = self.0 + rhs.0;
        VarInt::from(sum)
    }
}

impl ops::Add<usize> for VarInt {
    type Output = usize;

    fn add(self, rhs: usize) -> Self::Output {
        usize::from(self) + rhs
    }
}

impl ops::Add<VarInt> for usize {
    type Output = Self;

    fn add(self, rhs: VarInt) -> Self::Output {
        self + Self::from(rhs)
    }
}

impl Codec for VarInt {
    fn decode<B: Buf>(buf: &mut B) -> Result<Self, UnexpectedEnd> {
        unimplemented!()
    }

    fn encode<B: BufMut>(&self, buf: &mut B) {
        unimplemented!()
    }
}
pub fn size(x: u64) -> Option<usize> {
    if x < 2u64.pow(6) {
        Some(1)
    } else if x < 2u64.pow(14) {
        Some(2)
    } else if x < 2u64.pow(30) {
        Some(4)
    } else if x < 2u64.pow(62) {
        Some(8)
    } else {
        None
    }
}

pub fn read<R: Buf>(r: &mut R) -> Option<u64> {
    if !r.has_remaining() {
        return None;
    }
    let mut buf = [0; 8];
    buf[0] = r.get_u8();
    let tag = buf[0] >> 6;
    buf[0] &= 0b0011_1111;
    Some(match tag {
        0b00 => buf[0] as u64,
        0b01 => {
            if r.remaining() < 1 {
                return None;
            }
            r.copy_to_slice(&mut buf[1..2]);
            BigEndian::read_u16(&buf) as u64
        }
        0b10 => {
            if r.remaining() < 3 {
                return None;
            }
            r.copy_to_slice(&mut buf[1..4]);
            BigEndian::read_u32(&buf) as u64
        }
        0b11 => {
            if r.remaining() < 7 {
                return None;
            }
            r.copy_to_slice(&mut buf[1..8]);
            BigEndian::read_u64(&buf) as u64
        }
        _ => unreachable!(),
    })
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Fail)]
pub enum WriteError {
    #[fail(display = "insufficient space to encode value")]
    InsufficientSpace,
    #[fail(display = "value too large for varint encoding")]
    OversizedValue,
}

pub fn write<W: BufMut>(x: u64, w: &mut W) -> Result<(), WriteError> {
    if x < 2u64.pow(6) {
        if w.remaining_mut() < 1 {
            return Err(WriteError::InsufficientSpace);
        }
        w.put_u8(x as u8);
    } else if x < 2u64.pow(14) {
        if w.remaining_mut() < 2 {
            return Err(WriteError::InsufficientSpace);
        }
        w.put_u16_be(0b01 << 14 | x as u16);
    } else if x < 2u64.pow(30) {
        if w.remaining_mut() < 4 {
            return Err(WriteError::InsufficientSpace);
        }
        w.put_u32_be(0b10 << 30 | x as u32);
    } else if x < 2u64.pow(62) {
        if w.remaining_mut() < 8 {
            return Err(WriteError::InsufficientSpace);
        }
        w.put_u64_be(0b11 << 62 | x);
    } else {
        return Err(WriteError::OversizedValue);
    }
    Ok(())
}
