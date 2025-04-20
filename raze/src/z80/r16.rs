use std::ops::{AddAssign, SubAssign};

#[derive(Default, Copy, Clone)]
pub struct R16 {
    w: u16,
}

impl R16 {
    #[inline]
    pub fn from_bytes(lo: u8, hi: u8) -> R16 {
        let w = u16::from_le_bytes([lo, hi]);
        R16 { w }
    }
    #[inline]
    pub fn as_u16(self) -> u16 {
        self.w
    }
    #[inline]
    pub fn set(&mut self, w: u16) {
        self.w = w;
    }
    #[inline]
    pub fn lo(self) -> u8 {
        self.w.to_le_bytes()[0]
    }
    #[inline]
    pub fn hi(self) -> u8 {
        self.w.to_le_bytes()[1]
    }
    #[inline]
    pub fn set_lo(&mut self, b: u8) {
        let mut bs = self.w.to_le_bytes();
        bs[0] = b;
        self.w = u16::from_le_bytes(bs);
    }
    #[inline]
    pub fn set_hi(&mut self, b: u8) {
        let mut bs = self.w.to_le_bytes();
        bs[1] = b;
        self.w = u16::from_le_bytes(bs);
    }
}

impl From<R16> for u16 {
    #[inline]
    fn from(r: R16) -> Self {
        r.as_u16()
    }
}

impl From<u16> for R16 {
    #[inline]
    fn from(w: u16) -> Self {
        R16 { w }
    }
}

impl AddAssign<u16> for R16 {
    #[inline]
    fn add_assign(&mut self, r: u16) {
        let w = self.w.wrapping_add(r);
        self.w = w;
    }
}

impl SubAssign<u16> for R16 {
    #[inline]
    fn sub_assign(&mut self, r: u16) {
        let w = self.w.wrapping_sub(r);
        self.w = w;
    }
}
