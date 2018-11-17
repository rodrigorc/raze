use std::ops::{AddAssign, SubAssign};

#[repr(C)]
#[derive(Copy, Clone)]
pub union R16 {
    w: u16,
    b: [u8; 2]
}

#[cfg(target_endian="little")]
const LO_IDX : usize = 0;
#[cfg(target_endian="big")]
const LO_IDX : usize = 1;

const HI_IDX : usize = 1 - LO_IDX;

impl R16 {
    #[inline]
    pub fn from_bytes(lo: u8, hi: u8) -> R16 {
        let mut r = R16::default();
        r.set_lo(lo);
        r.set_hi(hi);
        r
    }
    pub fn as_u16(self) -> u16 {
        unsafe { self.w }
    }
    #[inline]
    pub fn set(&mut self, w: u16) {
        self.w = w;
    }
    #[inline]
    pub fn lo(self) -> u8 {
        unsafe { self.b[LO_IDX] }
    }
    #[inline]
    pub fn hi(self) -> u8 {
        unsafe { self.b[HI_IDX] }
    }
    #[inline]
    pub fn set_lo(&mut self, b: u8) {
        unsafe { self.b[LO_IDX] = b; }
    }
    #[inline]
    pub fn set_hi(&mut self, b: u8) {
        unsafe { self.b[HI_IDX] = b; }
    }
}

impl Default for R16 {
    fn default() -> Self {
        R16{ w: 0 }
    }
}

impl From<R16> for u16 {
    fn from(r: R16) -> Self {
        r.as_u16()
    }
}

impl From<u16> for R16 {
    fn from(w: u16) -> Self {
        R16{ w }
    }
}

impl AddAssign<u16> for R16 {
    fn add_assign(&mut self, r: u16) {
        let w = self.as_u16().wrapping_add(r);
        self.set(w);
    }
}

impl SubAssign<u16> for R16 {
    fn sub_assign(&mut self, r: u16) {
        let w = self.as_u16().wrapping_sub(r);
        self.set(w);
    }
}



