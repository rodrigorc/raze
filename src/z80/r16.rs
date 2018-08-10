use std::ops::{AddAssign, SubAssign};

#[cfg(target_endian="little")]
#[derive(Copy, Clone)]
#[repr(C)]
struct B8x2 {
    lo: u8, hi: u8
}

#[cfg(target_endian="big")]
#[derive(Copy, Clone)]
#[repr(C)]
struct B8x2x {
    hi: u8, lo: u8
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union R16 {
    w: u16,
    b: B8x2
}

impl R16 {
    pub fn as_u16(&self) -> u16 {
        unsafe { self.w }
    }
    pub fn set(&mut self, w: u16) {
        self.w = w;
    }
    pub fn lo(&self) -> u8 {
        unsafe { self.b.lo }
    }
    pub fn hi(&self) -> u8 {
        unsafe { self.b.hi }
    }
    pub fn set_lo(&mut self, b: u8) {
        unsafe { self.b.lo = b; }
    }
    pub fn set_hi(&mut self, b: u8) {
        unsafe { self.b.hi = b; }
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



