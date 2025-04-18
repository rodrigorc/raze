use anyhow::anyhow;
use std::mem;

mod r16;

use self::r16::R16;

pub trait Bus {
    fn peek(&mut self, addr: impl Into<u16>) -> u8;
    fn poke(&mut self, addr: impl Into<u16>, value: u8);
    fn do_in(&mut self, port: impl Into<u16>) -> u8;
    fn do_out(&mut self, port: impl Into<u16>, value: u8);

    fn peek_u16(&mut self, addr: impl Into<u16>) -> u16 {
        let addr = addr.into();
        let lo = u16::from(self.peek(addr));
        let addr = addr.wrapping_add(1);
        let hi = u16::from(self.peek(addr));
        (hi << 8) | lo
    }
    fn poke_u16(&mut self, addr: impl Into<u16>, data: u16) {
        let addr = addr.into();
        self.poke(addr, data as u8);
        let addr = addr.wrapping_add(1);
        self.poke(addr, (data >> 8) as u8);
    }
    fn inc_fetch_count(&mut self, _reason: FetchReason) {}
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum FetchReason {
    Fetch,
    Halt,
    Prefix,
    Interrupt,
}

const FLAG_S: u8 = 0b1000_0000;
const FLAG_Z: u8 = 0b0100_0000;
const FLAG_Y: u8 = 0b0010_0000;
const FLAG_H: u8 = 0b0001_0000;
const FLAG_X: u8 = 0b0000_1000;
const FLAG_PV: u8 = 0b0000_0100;
const FLAG_N: u8 = 0b0000_0010;
const FLAG_C: u8 = 0b0000_0001;

#[inline]
#[must_use]
fn flag8(f: u8, bit: u8) -> bool {
    (f & bit) != 0
}
#[inline]
#[must_use]
fn flag16(f: u16, bit: u16) -> bool {
    (f & bit) != 0
}
#[inline]
#[must_use]
fn set_flag8(f: u8, bit: u8, set: bool) -> u8 {
    if set { f | bit } else { f & !bit }
}
#[inline]
#[must_use]
fn parity(b: u8) -> bool {
    (b.count_ones()) % 2 == 0
}

#[inline]
#[must_use]
fn carry8(a: u8, b: u8, c: u8) -> bool {
    let ma = flag8(a, 0x80);
    let mb = flag8(b, 0x80);
    let mc = flag8(c, 0x80);
    (mc && ma && mb) || (!mc && (ma || mb))
}
#[inline]
#[must_use]
fn carry16(a: u16, b: u16, c: u16) -> bool {
    let ma = flag16(a, 0x8000);
    let mb = flag16(b, 0x8000);
    let mc = flag16(c, 0x8000);
    (mc && ma && mb) || (!mc && (ma || mb))
}
#[inline]
#[must_use]
fn half_carry8(a: u8, b: u8, c: u8) -> bool {
    let ma = flag8(a, 0x08);
    let mb = flag8(b, 0x08);
    let mc = flag8(c, 0x08);
    (mc && ma && mb) || (!mc && (ma || mb))
}
#[inline]
#[must_use]
fn half_carry16(a: u16, b: u16, c: u16) -> bool {
    let ma = flag16(a, 0x0800);
    let mb = flag16(b, 0x0800);
    let mc = flag16(c, 0x0800);
    (mc && ma && mb) || (!mc && (ma || mb))
}
#[inline]
#[must_use]
fn overflow_add8(a: u8, b: u8, c: u8) -> bool {
    flag8(a, 0x80) == flag8(b, 0x80) && flag8(a, 0x80) != flag8(c, 0x80)
}
#[inline]
#[must_use]
fn overflow_add16(a: u16, b: u16, c: u16) -> bool {
    flag16(a, 0x8000) == flag16(b, 0x8000) && flag16(a, 0x8000) != flag16(c, 0x8000)
}
#[inline]
#[must_use]
fn overflow_sub8(a: u8, b: u8, c: u8) -> bool {
    flag8(a, 0x80) != flag8(b, 0x80) && flag8(a, 0x80) != flag8(c, 0x80)
}
#[inline]
#[must_use]
fn overflow_sub16(a: u16, b: u16, c: u16) -> bool {
    flag16(a, 0x8000) != flag16(b, 0x8000) && flag16(a, 0x8000) != flag16(c, 0x8000)
}

#[inline]
#[must_use]
fn set_flag_sz(f: u8, r: u8) -> u8 {
    let f = set_flag8(f, FLAG_Z, r == 0);
    const CF: u8 = FLAG_S | FLAG_X | FLAG_Y;
    (f & !CF) | (r & CF)
}

#[inline]
#[must_use]
fn set_flag_szp(f: u8, r: u8) -> u8 {
    let f = set_flag8(f, FLAG_PV, parity(r));
    set_flag_sz(f, r)
}

#[inline]
#[must_use]
fn extend_sign(x: u8) -> u16 {
    i16::from(x as i8) as u16
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
enum InterruptMode {
    IM0,
    IM1,
    IM2,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
enum XYPrefix {
    None,
    IX,
    IY,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
enum NextOp {
    Fetch,
    Interrupt,
    Halt,
}

pub struct Z80 {
    pc: R16,
    sp: R16,
    af: R16,
    af_: R16,
    bc: R16,
    bc_: R16,
    de: R16,
    de_: R16,
    hl: R16,
    hl_: R16,
    ix: R16,
    iy: R16,
    i: u8,
    r_: u8, //bit 7 should not be used, use r7 instead, or better yet, r()
    r7: bool,
    iff1: bool,
    im: InterruptMode,
    next_op: NextOp,
}

#[derive(Clone, Copy)]
enum Direction {
    Inc,
    Dec,
}

// Known Z80 format versions: V1, V2, V3, V3.1
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Z80FileVersion {
    V1,
    V2,
    V3(bool),
}

mod exec_cb;
mod exec_ed;

impl Z80 {
    pub fn new() -> Z80 {
        Z80 {
            pc: R16::default(),
            sp: R16::default(),
            af: R16::default(),
            af_: R16::default(),
            bc: R16::default(),
            bc_: R16::default(),
            de: R16::default(),
            de_: R16::default(),
            hl: R16::default(),
            hl_: R16::default(),
            ix: R16::default(),
            iy: R16::default(),
            i: 0,
            r_: 0,
            r7: false,
            iff1: false,
            im: InterruptMode::IM0,
            next_op: NextOp::Fetch,
        }
    }
    pub fn _dump_regs(&self) {
        log::debug!(
            "PC {:04x}; AF {:04x}; BC {:04x}; DE {:04x}; HL {:04x}; IR {:02x}{:02x}; INT {}-{}",
            self.pc.as_u16(),
            self.af.as_u16() & 0xffd7,
            self.bc.as_u16(),
            self.de.as_u16(),
            self.hl.as_u16(),
            self.i,
            self.r(),
            match self.im {
                InterruptMode::IM0 => 0,
                InterruptMode::IM1 => 1,
                InterruptMode::IM2 => 2,
            },
            if self.iff1 { 1 } else { 0 }
        );
    }
    // 30 bytes are required for the snapshot, here we always store a V1, and
    // if other version is required it will be transformed later, because the V2/V3
    // information is not related to the CPU.
    pub fn snapshot(&self, data: &mut [u8]) {
        data[0] = self.a();
        data[1] = self.f();
        data[2] = self.c();
        data[3] = self.b();
        data[4] = self.l();
        data[5] = self.h();
        data[6] = self.pc.lo();
        data[7] = self.pc.hi();
        data[8] = self.sp.lo();
        data[9] = self.sp.hi();
        data[10] = self.i;
        data[11] = self.r_;
        data[12] = if self.r7 { 1 } else { 0 };
        data[13] = self.e();
        data[14] = self.d();
        data[15] = self.bc_.lo();
        data[16] = self.bc_.hi();
        data[17] = self.de_.lo();
        data[18] = self.de_.hi();
        data[19] = self.hl_.lo();
        data[20] = self.hl_.hi();
        data[21] = self.af_.hi();
        data[22] = self.af_.lo();
        data[23] = self.iy.lo();
        data[24] = self.iy.hi();
        data[25] = self.ix.lo();
        data[26] = self.ix.hi();
        data[27] = if self.iff1 { 1 } else { 0 };
        data[28] = data[27]; //iff2
        data[29] = match self.im {
            InterruptMode::IM0 => 0,
            InterruptMode::IM1 => 1,
            InterruptMode::IM2 => 2,
        } | 0x40; //kempston joystick
    }
    // V2/V3 use 34 bytes, V1 just 30, this can take a longer slice, so passing always the first 34 bytes of the file is safe
    pub fn load_snapshot(data: &[u8]) -> anyhow::Result<(Self, Z80FileVersion)> {
        if data.len() < 30 {
            return Err(anyhow!("Z80 snaphot too short"));
        }
        let af = R16::from_bytes(data[1], data[0]);
        let bc = R16::from_bytes(data[2], data[3]);
        let hl = R16::from_bytes(data[4], data[5]);
        let pc = R16::from_bytes(data[6], data[7]);
        let sp = R16::from_bytes(data[8], data[9]);
        let i = data[10];
        let r_ = data[11] & 0x7f;
        let r7 = (data[12] & 1) != 0;
        let de = R16::from_bytes(data[13], data[14]);
        let bc_ = R16::from_bytes(data[15], data[16]);
        let de_ = R16::from_bytes(data[17], data[18]);
        let hl_ = R16::from_bytes(data[19], data[20]);
        let af_ = R16::from_bytes(data[22], data[21]);
        let iy = R16::from_bytes(data[23], data[24]);
        let ix = R16::from_bytes(data[25], data[26]);
        let iff1 = data[27] != 0;
        //let iff2 = data[28];
        let im = match data[29] & 0x03 {
            1 => InterruptMode::IM1,
            2 => InterruptMode::IM2,
            _ => InterruptMode::IM0,
        };
        let (pc, version) = if pc.as_u16() == 0 {
            //v. 2 or 3
            if data.len() < 34 {
                return Err(anyhow!("Z80 snaphot too short"));
            }
            let extra = u16::from(data[30]) | (u16::from(data[31]) << 8);
            let pc = R16::from_bytes(data[32], data[33]);
            let version = match extra {
                23 => Z80FileVersion::V2,
                54 => Z80FileVersion::V3(false),
                55 => Z80FileVersion::V3(true),
                v => return Err(anyhow!("unknown Z80 snaphot version {}", v)),
            };
            (pc, version)
        } else {
            (pc, Z80FileVersion::V1)
        };
        let z80 = Z80 {
            pc,
            sp,
            af,
            af_,
            bc,
            bc_,
            de,
            de_,
            hl,
            hl_,
            ix,
            iy,
            i,
            r_,
            r7,
            iff1,
            im,
            next_op: NextOp::Fetch,
        };
        Ok((z80, version))
    }
    // Signals the CPU to run an interrupt on next fetch
    pub fn interrupt(&mut self) {
        if !self.iff1 {
            return;
        }
        self.next_op = NextOp::Interrupt;
    }
    // The value of the R register, as a full 8-bit value
    #[inline]
    fn r(&self) -> u8 {
        (self.r_ & 0x7f) | if self.r7 { 0x80 } else { 0x00 }
    }
    // Increments the R register as result of a memory cycle
    #[inline]
    fn inc_r(&mut self, bus: &mut impl Bus, reason: FetchReason) {
        self.r_ = self.r_.wrapping_add(1);
        bus.inc_fetch_count(reason);
    }
    // Forces the value of the R register, as a full 8-bit
    fn set_r(&mut self, r: u8) {
        self.r_ = r;
        self.r7 = flag8(r, 0x80);
    }

    // Easy access to registers by name
    #[inline]
    fn a(&self) -> u8 {
        self.af.hi()
    }
    #[inline]
    fn set_a(&mut self, a: u8) {
        self.af.set_hi(a);
    }
    #[inline]
    fn f(&self) -> u8 {
        self.af.lo()
    }
    #[inline]
    fn set_f(&mut self, f: u8) {
        self.af.set_lo(f);
    }
    #[inline]
    fn b(&self) -> u8 {
        self.bc.hi()
    }
    #[inline]
    fn set_b(&mut self, a: u8) {
        self.bc.set_hi(a);
    }
    #[inline]
    fn c(&self) -> u8 {
        self.bc.lo()
    }
    #[inline]
    fn set_c(&mut self, f: u8) {
        self.bc.set_lo(f);
    }
    #[inline]
    fn d(&self) -> u8 {
        self.de.hi()
    }
    #[inline]
    fn set_d(&mut self, a: u8) {
        self.de.set_hi(a);
    }
    #[inline]
    fn e(&self) -> u8 {
        self.de.lo()
    }
    #[inline]
    fn set_e(&mut self, f: u8) {
        self.de.set_lo(f);
    }
    #[inline]
    fn h(&self) -> u8 {
        self.hl.hi()
    }
    #[inline]
    fn set_h(&mut self, a: u8) {
        self.hl.set_hi(a);
    }
    #[inline]
    fn l(&self) -> u8 {
        self.hl.lo()
    }
    #[inline]
    fn set_l(&mut self, f: u8) {
        self.hl.set_lo(f);
    }
    #[inline]
    fn hx(&self, prefix: XYPrefix) -> u8 {
        self.hlx(prefix).hi()
    }
    #[inline]
    fn set_hx(&mut self, prefix: XYPrefix, a: u8) {
        self.hlx_mut(prefix).set_hi(a);
    }
    #[inline]
    fn lx(&self, prefix: XYPrefix) -> u8 {
        self.hlx(prefix).lo()
    }
    #[inline]
    fn set_lx(&mut self, prefix: XYPrefix, f: u8) {
        self.hlx_mut(prefix).set_lo(f);
    }

    // Fetches an 8-bit value from address PC and increments PC
    fn fetch(&mut self, bus: &mut impl Bus) -> u8 {
        let c = bus.peek(self.pc);
        self.pc += 1;
        c
    }
    // Fetches a 16-bit value from address PC and increments PC
    fn fetch_u16(&mut self, bus: &mut impl Bus) -> u16 {
        let l = u16::from(bus.peek(self.pc));
        self.pc += 1;
        let h = u16::from(bus.peek(self.pc));
        self.pc += 1;
        (h << 8) | l
    }
    // Pushes a 16-bit value into the stack SP
    fn push(&mut self, bus: &mut impl Bus, x: impl Into<u16>) {
        let x = x.into();
        self.sp -= 1;
        bus.poke(self.sp, (x >> 8) as u8);
        self.sp -= 1;
        bus.poke(self.sp, x as u8);
    }
    // Pops a 16-bit value from the stack SP
    fn pop(&mut self, bus: &mut impl Bus) -> u16 {
        let x = bus.peek_u16(self.sp);
        self.sp += 2;
        x
    }
    // Gets either HL, IX or IY depending on the prefix
    fn hlx(&self, prefix: XYPrefix) -> R16 {
        match prefix {
            XYPrefix::None => self.hl,
            XYPrefix::IX => self.ix,
            XYPrefix::IY => self.iy,
        }
    }
    // Same as hlx() but as a mutable reference
    fn hlx_mut(&mut self, prefix: XYPrefix) -> &mut R16 {
        match prefix {
            XYPrefix::None => &mut self.hl,
            XYPrefix::IX => &mut self.ix,
            XYPrefix::IY => &mut self.iy,
        }
    }
    // Gets either HL, IX+n or IY+n. If needed n is fetched from PC.
    // Returns the (address, extra_T_states).
    fn hlx_addr(&mut self, prefix: XYPrefix, bus: &mut impl Bus) -> (u16, u32) {
        match prefix {
            XYPrefix::None => (self.hl.as_u16(), 0),
            XYPrefix::IX => {
                let d = self.fetch(bus);
                (self.ix.as_u16().wrapping_add(extend_sign(d)), 8)
            }
            XYPrefix::IY => {
                let d = self.fetch(bus);
                (self.iy.as_u16().wrapping_add(extend_sign(d)), 8)
            }
        }
    }
    // Substracts two 8-bit values, maybe with carry
    fn sub_flags(&mut self, a: u8, b: u8, with_carry: bool) -> u8 {
        let mut r = a.wrapping_sub(b);
        let mut f = self.f();
        if with_carry && flag8(f, FLAG_C) {
            r = r.wrapping_sub(1);
        }
        f = set_flag8(f, FLAG_N, true);
        f = set_flag8(f, FLAG_C, carry8(r, b, a));
        f = set_flag8(f, FLAG_H, half_carry8(r, b, a));
        f = set_flag8(f, FLAG_PV, overflow_sub8(a, b, r));
        f = set_flag_sz(f, r);
        self.set_f(f);
        r
    }
    // Substracts two 16-bit values with carry (there is no sub16_flags)
    fn sbc16_flags(&mut self, a: u16, b: u16) -> u16 {
        let mut f = self.f();
        let mut r = a.wrapping_sub(b);
        if flag8(f, FLAG_C) {
            r = r.wrapping_sub(1);
        }
        f = set_flag8(f, FLAG_N, true);
        f = set_flag8(f, FLAG_C, carry16(r, b, a));
        f = set_flag8(f, FLAG_PV, overflow_sub16(a, b, r));
        f = set_flag8(f, FLAG_Z, r == 0);
        f = set_flag8(f, FLAG_S, flag16(r, 0x8000));
        f = set_flag8(f, FLAG_H, half_carry16(r, b, a));
        self.set_f(f);
        r
    }
    // Adds thow 8-bit values, maybe with carry
    fn add_flags(&mut self, a: u8, b: u8, with_carry: bool) -> u8 {
        let mut f = self.f();
        let mut r = a.wrapping_add(b);
        if with_carry && flag8(f, FLAG_C) {
            r = r.wrapping_add(1);
        }
        f = set_flag8(f, FLAG_N, false);
        f = set_flag8(f, FLAG_C, carry8(a, b, r));
        f = set_flag8(f, FLAG_H, half_carry8(a, b, r));
        f = set_flag8(f, FLAG_PV, overflow_add8(a, b, r));
        f = set_flag_sz(f, r);
        self.set_f(f);
        r
    }
    // Adds thow 16-bit values with carry
    fn adc16_flags(&mut self, a: u16, b: u16) -> u16 {
        let mut f = self.f();
        let mut r = a.wrapping_add(b);
        if flag8(f, FLAG_C) {
            r = r.wrapping_add(1);
        }
        f = set_flag8(f, FLAG_N, false);
        f = set_flag8(f, FLAG_C, carry16(a, b, r));
        f = set_flag8(f, FLAG_PV, overflow_add16(a, b, r));
        f = set_flag8(f, FLAG_Z, r == 0);
        f = set_flag8(f, FLAG_S, flag16(r, 0x8000));
        f = set_flag8(f, FLAG_H, half_carry16(a, b, r));
        self.set_f(f);
        r
    }
    // Adds thow 16-bit values without carry
    fn add16_flags(&mut self, a: u16, b: u16) -> u16 {
        let r = a.wrapping_add(b);
        let mut f = self.f();
        f = set_flag8(f, FLAG_N, false);
        f = set_flag8(f, FLAG_C, carry16(a, b, r));
        //No PV, Z, S flags!
        f = set_flag8(f, FLAG_H, half_carry16(a, b, r));
        self.set_f(f);
        r
    }
    // Increments an 8-bit value
    fn inc_flags(&mut self, a: u8) -> u8 {
        let r = a.wrapping_add(1);
        let mut f = self.f();
        f = set_flag8(f, FLAG_N, false);
        f = set_flag8(f, FLAG_H, (r & 0x0f) == 0x00);
        f = set_flag8(f, FLAG_PV, r == 0x80);
        f = set_flag_sz(f, r);
        self.set_f(f);
        r
    }
    // Decrements an 8-bit value
    fn dec_flags(&mut self, a: u8) -> u8 {
        let r = a.wrapping_sub(1);
        let mut f = self.f();
        f = set_flag8(f, FLAG_N, true);
        f = set_flag8(f, FLAG_H, (r & 0x0f) == 0x0f);
        f = set_flag8(f, FLAG_PV, r == 0x7f);
        f = set_flag_sz(f, r);
        self.set_f(f);
        r
    }
    // BitAnd between two 8-bit values
    fn and_flags(&mut self, a: u8, b: u8) -> u8 {
        let r = a & b;
        let mut f = self.f();
        f = set_flag8(f, FLAG_C, false);
        f = set_flag8(f, FLAG_N, false);
        f = set_flag8(f, FLAG_H, true);
        f = set_flag_szp(f, r);
        self.set_f(f);
        r
    }
    // BitOr between two 8-bit values
    fn or_flags(&mut self, a: u8, b: u8) -> u8 {
        let r = a | b;
        let mut f = self.f();
        f = set_flag8(f, FLAG_C, false);
        f = set_flag8(f, FLAG_N, false);
        f = set_flag8(f, FLAG_H, false);
        f = set_flag_szp(f, r);
        self.set_f(f);
        r
    }
    // BitXor between two 8-bit values
    fn xor_flags(&mut self, a: u8, b: u8) -> u8 {
        let r = a ^ b;
        let mut f = self.f();
        f = set_flag8(f, FLAG_C, false);
        f = set_flag8(f, FLAG_N, false);
        f = set_flag8(f, FLAG_H, false);
        f = set_flag_szp(f, r);
        self.set_f(f);
        r
    }
    // Load & increment/decrement from (HL) into (DE), then decrements BC
    fn ldi_ldd(&mut self, dir: Direction, bus: &mut impl Bus) {
        let hl = self.hl;
        let de = self.de;
        let x = bus.peek(hl);
        bus.poke(de, x);

        match dir {
            Direction::Inc => {
                self.hl += 1;
                self.de += 1;
            }
            Direction::Dec => {
                self.hl -= 1;
                self.de -= 1;
            }
        };
        self.bc -= 1;

        let mut f = self.f();
        f = set_flag8(f, FLAG_N, false);
        f = set_flag8(f, FLAG_H, false);
        f = set_flag8(f, FLAG_PV, self.bc.as_u16() != 0);
        self.set_f(f);
    }
    // Compare & increment/decrement (HL) and A, then decrement BC
    fn cpi_cpd(&mut self, dir: Direction, bus: &mut impl Bus) -> u8 {
        let hl = self.hl;
        let x = bus.peek(hl);
        let a = self.a();
        let mut f = self.f();

        match dir {
            Direction::Inc => self.hl += 1,
            Direction::Dec => self.hl -= 1,
        };
        self.bc -= 1;

        let r = a.wrapping_sub(x);
        f = set_flag8(f, FLAG_H, half_carry8(r, x, a));
        f = set_flag8(f, FLAG_N, true);
        f = set_flag8(f, FLAG_PV, self.bc.as_u16() != 0);
        f = set_flag_sz(f, r);
        self.set_f(f);
        r
    }
    // Reads 8-bit IO port BC, store it into (HL), increment/decrement HL, then decrements B
    fn ini_ind(&mut self, dir: Direction, bus: &mut impl Bus) -> u8 {
        let x = bus.do_in(self.bc.as_u16());
        bus.poke(self.hl, x);
        match dir {
            Direction::Inc => self.hl += 1,
            Direction::Dec => self.hl -= 1,
        };
        let b = self.b().wrapping_sub(1);
        self.set_b(b);
        let mut f = self.f();
        f = set_flag8(f, FLAG_N | FLAG_S, flag8(b, 0x80));
        f = set_flag8(f, FLAG_Z, b == 0);
        self.set_f(f);
        b
    }
    // Reads 8-bit from (HL), writes it to IO port BC, increment/decrement HL, then decrements B
    fn outi_outd(&mut self, dir: Direction, bus: &mut impl Bus) -> u8 {
        let x = bus.peek(self.hl);
        bus.do_out(self.bc.as_u16(), x);
        match dir {
            Direction::Inc => self.hl += 1,
            Direction::Dec => self.hl -= 1,
        };
        let b = self.b().wrapping_sub(1);
        self.set_b(b);
        let mut f = self.f();
        f = set_flag8(f, FLAG_N | FLAG_S, flag8(b, 0x80));
        f = set_flag8(f, FLAG_Z, b == 0);
        self.set_f(f);
        b
    }
    // Decimal Arithmetic Adjust, the logic is incomprehensible
    fn daa(&mut self) {
        let a = self.a();
        let mut f = self.f();
        const O: u8 = 0;
        const N: u8 = FLAG_N;
        const C: u8 = FLAG_C;
        const H: u8 = FLAG_H;
        const CH: u8 = FLAG_C | FLAG_H;
        const NH: u8 = FLAG_N | FLAG_H;
        const NC: u8 = FLAG_N | FLAG_C;
        const NCH: u8 = FLAG_N | FLAG_C | FLAG_H;
        let (plus_a, new_c, new_h) = match (f & (FLAG_N | FLAG_C | FLAG_H), a >> 4, a & 0x0f) {
            (O, 0x0..=0x9, 0x0..=0x9) => (0x00, false, false),
            (O, 0x0..=0x8, 0xa..=0xf) => (0x06, false, true),
            (O, _, 0x0..=0x9) => (0x60, true, false),
            (O, _, 0xa..=0xf) => (0x66, true, true),

            (H, 0x0..=0x9, 0x0..=0x9) => (0x06, false, false),
            (H, 0x0..=0x8, 0xa..=0xf) => (0x06, false, true),
            (H, _, 0x0..=0x9) => (0x66, true, false),
            (H, _, 0xa..=0xf) => (0x66, true, true),

            (C, _, 0x0..=0x9) => (0x60, true, false),
            (C, _, 0xa..=0xf) => (0x66, true, true),

            (CH, _, 0x0..=0x9) => (0x66, true, false),
            (CH, _, 0xa..=0xf) => (0x66, true, true),

            (N, 0x0..=0x9, 0x0..=0x9) => (0x00, false, false),
            (N, 0x0..=0x8, 0xa..=0xf) => (0xfa, false, false),
            (N, _, 0x0..=0x9) => (0xa0, true, false),
            (N, _, 0xa..=0xf) => (0x9a, true, false),

            (NH, 0x0..=0x9, 0x0..=0x5) => (0xfa, false, true),
            (NH, _, 0x0..=0x5) => (0x9a, true, true),
            (NH, 0x0..=0x8, 0x6..=0xf) => (0xfa, false, false),
            (NH, 0xa..=0xf, 0x6..=0xf) => (0x9a, true, false),
            (NH, 0x9..=0x9, 0x6..=0x9) => (0xfa, false, false),
            (NH, 0x9..=0x9, 0xa..=0xf) => (0x9a, true, false),

            (NC, _, 0x0..=0x9) => (0xa0, true, false),
            (NC, _, 0xa..=0xf) => (0x9a, true, false),

            (NCH, _, 0x0..=0x5) => (0x9a, true, true),
            (NCH, _, 0x6..=0xf) => (0x9a, true, false),

            _ => unreachable!(),
        };
        let a = a.wrapping_add(plus_a);
        f = set_flag8(f, FLAG_C, new_c);
        f = set_flag8(f, FLAG_H, new_h);
        f = set_flag_szp(f, a);
        self.set_a(a);
        self.set_f(f);
    }
    // Exects the next instruction from PC, returns the consumed T-states
    pub fn exec(&mut self, bus: &mut impl Bus) -> u32 {
        let mut opcode = match self.next_op {
            NextOp::Fetch => {
                self.inc_r(bus, FetchReason::Fetch);
                self.fetch(bus)
            }
            NextOp::Halt => {
                self.inc_r(bus, FetchReason::Halt);
                0x00 //NOP
            }
            NextOp::Interrupt => {
                self.inc_r(bus, FetchReason::Interrupt);
                self.next_op = NextOp::Fetch;
                self.iff1 = false;
                match self.im {
                    InterruptMode::IM0 => {
                        log::debug!("IM0 interrupt!");
                        0x00 //NOP
                    }
                    InterruptMode::IM1 => {
                        0xff //RST 38
                    }
                    InterruptMode::IM2 => {
                        //assume 0xff in the data bus
                        let v = (u16::from(self.i) << 8) | 0xff;
                        let v = bus.peek_u16(v);
                        let pc = self.pc;
                        self.push(bus, pc);
                        self.pc.set(v);
                        return 0;
                    }
                }
            }
        };
        let mut t = 0;
        let mut prefix = XYPrefix::None;
        // The IX/IY prefix can be repeated many times to force a delay or avoid an interrupt
        let opcode = loop {
            opcode = match opcode {
                0xdd => {
                    prefix = XYPrefix::IX;
                    t += 4;
                    self.inc_r(bus, FetchReason::Prefix);
                    self.fetch(bus)
                }
                0xfd => {
                    prefix = XYPrefix::IY;
                    t += 4;
                    self.inc_r(bus, FetchReason::Prefix);
                    self.fetch(bus)
                }
                _ => break opcode,
            };
        };
        // Decode the opcode, computing the additional T-states
        let t2 = match opcode {
            0x00 => {
                //NOP
                4
            }
            0x01 => {
                //LD BC,nn
                let d = self.fetch_u16(bus);
                self.bc.set(d);
                10
            }
            0x02 => {
                //LD (BC),A
                let a = self.a();
                bus.poke(self.bc, a);
                7
            }
            0x03 => {
                //INC BC
                self.bc += 1;
                6
            }
            0x04 => {
                //INC B
                let mut r = self.b();
                r = self.inc_flags(r);
                self.set_b(r);
                4
            }
            0x05 => {
                //DEC B
                let mut r = self.b();
                r = self.dec_flags(r);
                self.set_b(r);
                4
            }
            0x06 => {
                //LD B,n
                let n = self.fetch(bus);
                self.set_b(n);
                7
            }
            0x07 => {
                //RLCA
                let mut a = self.a();
                let mut f = self.f();
                let b7 = flag8(a, 0x80);
                a = a.rotate_left(1);
                f = set_flag8(f, FLAG_C, b7);
                f = set_flag8(f, FLAG_N, false);
                f = set_flag8(f, FLAG_H, false);
                self.set_a(a);
                self.set_f(f);
                4
            }
            0x08 => {
                //EX AF,AF
                mem::swap(&mut self.af, &mut self.af_);
                4
            }
            0x09 => {
                //ADD HL,BC
                let mut hl = self.hlx(prefix).as_u16();
                let bc = self.bc.as_u16();
                hl = self.add16_flags(hl, bc);
                self.hlx_mut(prefix).set(hl);
                11
            }
            0x0a => {
                //LD A,(BC)
                let a = bus.peek(self.bc);
                self.set_a(a);
                7
            }
            0x0b => {
                //DEC BC
                self.bc -= 1;
                6
            }
            0x0c => {
                //INC C
                let mut r = self.c();
                r = self.inc_flags(r);
                self.set_c(r);
                4
            }
            0x0d => {
                //DEC C
                let mut r = self.c();
                r = self.dec_flags(r);
                self.set_c(r);
                4
            }
            0x0e => {
                //LD C,n
                let n = self.fetch(bus);
                self.set_c(n);
                7
            }
            0x0f => {
                //RRCA
                let mut a = self.a();
                let mut f = self.f();
                let b0 = flag8(a, 0x01);
                a = a.rotate_right(1);
                f = set_flag8(f, FLAG_C, b0);
                f = set_flag8(f, FLAG_N, false);
                f = set_flag8(f, FLAG_H, false);
                self.set_a(a);
                self.set_f(f);
                4
            }
            0x10 => {
                //DJNZ d
                let d = self.fetch(bus);
                let mut b = self.b();
                b = b.wrapping_sub(1);
                self.set_b(b);
                if b != 0 {
                    self.pc += extend_sign(d);
                    13
                } else {
                    8
                }
            }
            0x11 => {
                //LD DE,nn
                let nn = self.fetch_u16(bus);
                self.de.set(nn);
                10
            }
            0x12 => {
                //LD (DE),A
                let a = self.a();
                bus.poke(self.de, a);
                7
            }
            0x13 => {
                //INC DE
                self.de += 1;
                6
            }
            0x14 => {
                //INC D
                let mut r = self.d();
                r = self.inc_flags(r);
                self.set_d(r);
                4
            }
            0x15 => {
                //DEC D
                let mut r = self.d();
                r = self.dec_flags(r);
                self.set_d(r);
                4
            }
            0x16 => {
                //LD D,n
                let n = self.fetch(bus);
                self.set_d(n);
                7
            }
            0x17 => {
                //RLA
                let mut a = self.a();
                let mut f = self.f();
                let b7 = flag8(a, 0x80);
                let c = flag8(f, FLAG_C);
                a <<= 1;
                a = set_flag8(a, 1, c);
                f = set_flag8(f, FLAG_C, b7);
                f = set_flag8(f, FLAG_N, false);
                f = set_flag8(f, FLAG_H, false);
                self.set_a(a);
                self.set_f(f);
                4
            }
            0x18 => {
                //JR d
                let d = self.fetch(bus);
                self.pc += extend_sign(d);
                12
            }
            0x19 => {
                //ADD HL,DE
                let mut hl = self.hlx(prefix).as_u16();
                let de = self.de.as_u16();
                hl = self.add16_flags(hl, de);
                self.hlx_mut(prefix).set(hl);
                11
            }
            0x1a => {
                //LD A,(DE)
                let a = bus.peek(self.de);
                self.set_a(a);
                7
            }
            0x1b => {
                //DEC DE
                self.de -= 1;
                6
            }
            0x1c => {
                //INC E
                let mut r = self.e();
                r = self.inc_flags(r);
                self.set_e(r);
                4
            }
            0x1d => {
                //DEC E
                let mut r = self.e();
                r = self.dec_flags(r);
                self.set_e(r);
                4
            }
            0x1e => {
                //LD E,n
                let n = self.fetch(bus);
                self.set_e(n);
                7
            }
            0x1f => {
                //RRA
                let mut a = self.a();
                let mut f = self.f();
                let b0 = flag8(a, 0x01);
                let c = flag8(f, FLAG_C);
                a >>= 1;
                a = set_flag8(a, 0x80, c);
                f = set_flag8(f, FLAG_C, b0);
                f = set_flag8(f, FLAG_N, false);
                f = set_flag8(f, FLAG_H, false);
                self.set_a(a);
                self.set_f(f);
                4
            }
            0x20 => {
                //JR NZ,d
                let d = self.fetch(bus);
                if !flag8(self.f(), FLAG_Z) {
                    self.pc += extend_sign(d);
                }
                12
            }
            0x21 => {
                //LD HL,nn
                let d = self.fetch_u16(bus);
                self.hlx_mut(prefix).set(d);
                10
            }
            0x22 => {
                //LD (nn),HL
                let addr = self.fetch_u16(bus);
                bus.poke_u16(addr, self.hlx(prefix).as_u16());
                16
            }
            0x23 => {
                //INC HL
                *self.hlx_mut(prefix) += 1;
                6
            }
            0x24 => {
                //INC H
                let mut r = self.hx(prefix);
                r = self.inc_flags(r);
                self.set_hx(prefix, r);
                4
            }
            0x25 => {
                //DEC H
                let mut r = self.hx(prefix);
                r = self.dec_flags(r);
                self.set_hx(prefix, r);
                4
            }
            0x26 => {
                //LD H,n
                let n = self.fetch(bus);
                self.set_hx(prefix, n);
                7
            }
            0x27 => {
                //DAA
                self.daa();
                4
            }
            0x28 => {
                //JR Z,d
                let d = self.fetch(bus);
                if flag8(self.f(), FLAG_Z) {
                    self.pc += extend_sign(d);
                }
                12
            }
            0x29 => {
                //ADD HL,HL
                let mut hl = self.hlx(prefix).as_u16();
                hl = self.add16_flags(hl, hl);
                self.hlx_mut(prefix).set(hl);
                11
            }
            0x2a => {
                //LD HL,(nn)
                let addr = self.fetch_u16(bus);
                let d = bus.peek_u16(addr);
                self.hlx_mut(prefix).set(d);
                16
            }
            0x2b => {
                //DEC HL
                *self.hlx_mut(prefix) -= 1;
                6
            }
            0x2c => {
                //INC L
                let mut r = self.lx(prefix);
                r = self.inc_flags(r);
                self.set_lx(prefix, r);
                4
            }
            0x2d => {
                //DEC L
                let mut r = self.lx(prefix);
                r = self.dec_flags(r);
                self.set_lx(prefix, r);
                4
            }
            0x2e => {
                //LD L,n
                let n = self.fetch(bus);
                self.set_lx(prefix, n);
                7
            }
            0x2f => {
                //CPL
                let mut a = self.a();
                let mut f = self.f();
                a ^= 0xff;
                f = set_flag8(f, FLAG_H, true);
                f = set_flag8(f, FLAG_N, true);
                self.set_a(a);
                self.set_f(f);
                4
            }
            0x30 => {
                //JR NC,d
                let d = self.fetch(bus);
                if !flag8(self.f(), FLAG_C) {
                    self.pc += extend_sign(d);
                }
                12
            }
            0x31 => {
                //LD SP,nn
                let nn = self.fetch_u16(bus);
                self.sp.set(nn);
                10
            }
            0x32 => {
                //LD (nn),A
                let addr = self.fetch_u16(bus);
                bus.poke(addr, self.a());
                13
            }
            0x33 => {
                //INC SP
                self.sp += 1;
                6
            }
            0x34 => {
                //INC (HL)
                let (addr, t) = self.hlx_addr(prefix, bus);
                let mut b = bus.peek(addr);
                b = self.inc_flags(b);
                bus.poke(addr, b);
                11 + t
            }
            0x35 => {
                //DEC (HL)
                let (addr, t) = self.hlx_addr(prefix, bus);
                let mut b = bus.peek(addr);
                b = self.dec_flags(b);
                bus.poke(addr, b);
                11 + t
            }
            0x36 => {
                //LD (HL),n
                let (addr, t) = self.hlx_addr(prefix, bus);
                let n = self.fetch(bus);
                bus.poke(addr, n);
                10 + t
            }
            0x37 => {
                //SCF
                let mut f = self.f();
                f = set_flag8(f, FLAG_N, false);
                f = set_flag8(f, FLAG_H, false);
                f = set_flag8(f, FLAG_C, true);
                self.set_f(f);
                4
            }
            0x38 => {
                //JR C,d
                let d = self.fetch(bus);
                if flag8(self.f(), FLAG_C) {
                    self.pc += extend_sign(d);
                }
                12
            }
            0x39 => {
                //ADD HL,SP
                let mut hl = self.hlx(prefix).as_u16();
                let sp = self.sp.as_u16();
                hl = self.add16_flags(hl, sp);
                self.hlx_mut(prefix).set(hl);
                11
            }
            0x3a => {
                //LD A,(nn)
                let addr = self.fetch_u16(bus);
                let x = bus.peek(addr);
                self.set_a(x);
                13
            }
            0x3b => {
                //DEC SP
                self.sp -= 1;
                6
            }
            0x3c => {
                //INC A
                let mut r = self.a();
                r = self.inc_flags(r);
                self.set_a(r);
                4
            }
            0x3d => {
                //DEC A
                let mut r = self.a();
                r = self.dec_flags(r);
                self.set_a(r);
                4
            }
            0x3e => {
                //LD A,n
                let n = self.fetch(bus);
                self.set_a(n);
                7
            }
            0x3f => {
                //CCF
                let mut f = self.f();
                let c = flag8(f, FLAG_C);
                f = set_flag8(f, FLAG_N, false);
                f = set_flag8(f, FLAG_H, c);
                f = set_flag8(f, FLAG_C, !c);
                self.set_f(f);
                4
            }
            0x40 => {
                //LD B,B
                4
            }
            0x41 => {
                //LD B,C
                let r = self.c();
                self.set_b(r);
                4
            }
            0x42 => {
                //LD B,D
                let r = self.d();
                self.set_b(r);
                4
            }
            0x43 => {
                //LD B,E
                let r = self.e();
                self.set_b(r);
                4
            }
            0x44 => {
                //LD B,H
                let r = self.hx(prefix);
                self.set_b(r);
                4
            }
            0x45 => {
                //LD B,L
                let r = self.lx(prefix);
                self.set_b(r);
                4
            }
            0x46 => {
                //LD B,(HL)
                let (addr, t) = self.hlx_addr(prefix, bus);
                let r = bus.peek(addr);
                self.set_b(r);
                7 + t
            }
            0x47 => {
                //LD B,A
                let r = self.a();
                self.set_b(r);
                4
            }
            0x48 => {
                //LD C,B
                let r = self.b();
                self.set_c(r);
                4
            }
            0x49 => {
                //LD C,C
                4
            }
            0x4a => {
                //LD C,D
                let r = self.d();
                self.set_c(r);
                4
            }
            0x4b => {
                //LD C,E
                let r = self.e();
                self.set_c(r);
                4
            }
            0x4c => {
                //LD C,H
                let r = self.hx(prefix);
                self.set_c(r);
                4
            }
            0x4d => {
                //LD C,L
                let r = self.lx(prefix);
                self.set_c(r);
                4
            }
            0x4e => {
                //LD C,(HL)
                let (addr, t) = self.hlx_addr(prefix, bus);
                let r = bus.peek(addr);
                self.set_c(r);
                7 + t
            }
            0x4f => {
                //LD C,A
                let r = self.a();
                self.set_c(r);
                4
            }
            0x50 => {
                //LD D,B
                let r = self.b();
                self.set_d(r);
                4
            }
            0x51 => {
                //LD D,C
                let r = self.c();
                self.set_d(r);
                4
            }
            0x52 => {
                //LD D,D
                4
            }
            0x53 => {
                //LD D,E
                let r = self.e();
                self.set_d(r);
                4
            }
            0x54 => {
                //LD D,H
                let r = self.hx(prefix);
                self.set_d(r);
                4
            }
            0x55 => {
                //LD D,L
                let r = self.lx(prefix);
                self.set_d(r);
                4
            }
            0x56 => {
                //LD D,(HL)
                let (addr, t) = self.hlx_addr(prefix, bus);
                let r = bus.peek(addr);
                self.set_d(r);
                7 + t
            }
            0x57 => {
                //LD D,A
                let r = self.a();
                self.set_d(r);
                4
            }
            0x58 => {
                //LD E,B
                let r = self.b();
                self.set_e(r);
                4
            }
            0x59 => {
                //LD E,C
                let r = self.c();
                self.set_e(r);
                4
            }
            0x5a => {
                //LD E,D
                let r = self.d();
                self.set_e(r);
                4
            }
            0x5b => {
                //LD E,E
                4
            }
            0x5c => {
                //LD E,H
                let r = self.hx(prefix);
                self.set_e(r);
                4
            }
            0x5d => {
                //LD E,L
                let r = self.lx(prefix);
                self.set_e(r);
                4
            }
            0x5e => {
                //LD E,(HL)
                let (addr, t) = self.hlx_addr(prefix, bus);
                let r = bus.peek(addr);
                self.set_e(r);
                7 + t
            }
            0x5f => {
                //LD E,A
                let r = self.a();
                self.set_e(r);
                4
            }
            0x60 => {
                //LD H,B
                let r = self.b();
                self.set_hx(prefix, r);
                4
            }
            0x61 => {
                //LD H,C
                let r = self.c();
                self.set_hx(prefix, r);
                4
            }
            0x62 => {
                //LD H,D
                let r = self.d();
                self.set_hx(prefix, r);
                4
            }
            0x63 => {
                //LD H,E
                let r = self.e();
                self.set_hx(prefix, r);
                4
            }
            0x64 => {
                //LD H,H
                let r = self.hx(prefix);
                self.set_h(r);
                4
            }
            0x65 => {
                //LD H,L
                let r = self.lx(prefix);
                self.set_hx(prefix, r);
                4
            }
            0x66 => {
                //LD H,(HL)
                let (addr, t) = self.hlx_addr(prefix, bus);
                let r = bus.peek(addr);
                self.set_h(r);
                7 + t
            }
            0x67 => {
                //LD H,A
                let r = self.a();
                self.set_hx(prefix, r);
                4
            }
            0x68 => {
                //LD L,B
                let r = self.b();
                self.set_lx(prefix, r);
                4
            }
            0x69 => {
                //LD L,C
                let r = self.c();
                self.set_lx(prefix, r);
                4
            }
            0x6a => {
                //LD L,D
                let r = self.d();
                self.set_lx(prefix, r);
                4
            }
            0x6b => {
                //LD L,E
                let r = self.e();
                self.set_lx(prefix, r);
                4
            }
            0x6c => {
                //LD L,H
                let r = self.hx(prefix);
                self.set_lx(prefix, r);
                4
            }
            0x6d => {
                //LD L,L
                4
            }
            0x6e => {
                //LD L,(HL)
                let (addr, t) = self.hlx_addr(prefix, bus);
                let r = bus.peek(addr);
                self.set_l(r);
                7 + t
            }
            0x6f => {
                //LD L,A
                let r = self.a();
                self.set_lx(prefix, r);
                4
            }
            0x70 => {
                //LD (HL),B
                let r = self.b();
                let (addr, t) = self.hlx_addr(prefix, bus);
                bus.poke(addr, r);
                7 + t
            }
            0x71 => {
                //LD (HL),C
                let r = self.c();
                let (addr, t) = self.hlx_addr(prefix, bus);
                bus.poke(addr, r);
                7 + t
            }
            0x72 => {
                //LD (HL),D
                let r = self.d();
                let (addr, t) = self.hlx_addr(prefix, bus);
                bus.poke(addr, r);
                7 + t
            }
            0x73 => {
                //LD (HL),E
                let r = self.e();
                let (addr, t) = self.hlx_addr(prefix, bus);
                bus.poke(addr, r);
                7 + t
            }
            0x74 => {
                //LD (HL),H
                let r = self.h();
                let (addr, t) = self.hlx_addr(prefix, bus);
                bus.poke(addr, r);
                7 + t
            }
            0x75 => {
                //LD (HL),L
                let r = self.l();
                let (addr, t) = self.hlx_addr(prefix, bus);
                bus.poke(addr, r);
                7 + t
            }
            0x76 => {
                //HALT
                if !self.iff1 {
                    log::warn!("DI/HALT deadlock!");
                }
                self.next_op = NextOp::Halt;
                4
            }
            0x77 => {
                //LD (HL),A
                let r = self.a();
                let (addr, t) = self.hlx_addr(prefix, bus);
                bus.poke(addr, r);
                7 + t
            }
            0x78 => {
                //LD A,B
                let r = self.b();
                self.set_a(r);
                4
            }
            0x79 => {
                //LD A,C
                let r = self.c();
                self.set_a(r);
                4
            }
            0x7a => {
                //LD A,D
                let r = self.d();
                self.set_a(r);
                4
            }
            0x7b => {
                //LD A,E
                let r = self.e();
                self.set_a(r);
                4
            }
            0x7c => {
                //LD A,H
                let r = self.hx(prefix);
                self.set_a(r);
                4
            }
            0x7d => {
                //LD A,L
                let r = self.lx(prefix);
                self.set_a(r);
                4
            }
            0x7e => {
                //LD A,(HL)
                let (addr, t) = self.hlx_addr(prefix, bus);
                let r = bus.peek(addr);
                self.set_a(r);
                7 + t
            }
            0x7f => {
                //LD A,A
                4
            }
            0x80 => {
                //ADD B
                let a = self.a();
                let r = self.b();
                let a = self.add_flags(a, r, false);
                self.set_a(a);
                4
            }
            0x81 => {
                //ADD C
                let a = self.a();
                let r = self.c();
                let a = self.add_flags(a, r, false);
                self.set_a(a);
                4
            }
            0x82 => {
                //ADD D
                let a = self.a();
                let r = self.d();
                let a = self.add_flags(a, r, false);
                self.set_a(a);
                4
            }
            0x83 => {
                //ADD E
                let a = self.a();
                let r = self.e();
                let a = self.add_flags(a, r, false);
                self.set_a(a);
                4
            }
            0x84 => {
                //ADD H
                let a = self.a();
                let r = self.hx(prefix);
                let a = self.add_flags(a, r, false);
                self.set_a(a);
                4
            }
            0x85 => {
                //ADD L
                let a = self.a();
                let r = self.lx(prefix);
                let a = self.add_flags(a, r, false);
                self.set_a(a);
                4
            }
            0x86 => {
                //ADD (HL)
                let a = self.a();
                let (addr, t) = self.hlx_addr(prefix, bus);
                let r = bus.peek(addr);
                let a = self.add_flags(a, r, false);
                self.set_a(a);
                4 + t
            }
            0x87 => {
                //ADD A
                let a = self.a();
                let a = self.add_flags(a, a, false);
                self.set_a(a);
                4
            }
            0x88 => {
                //ADC B
                let a = self.a();
                let r = self.b();
                let a = self.add_flags(a, r, true);
                self.set_a(a);
                4
            }
            0x89 => {
                //ADC C
                let a = self.a();
                let r = self.c();
                let a = self.add_flags(a, r, true);
                self.set_a(a);
                4
            }
            0x8a => {
                //ADC D
                let a = self.a();
                let r = self.d();
                let a = self.add_flags(a, r, true);
                self.set_a(a);
                4
            }
            0x8b => {
                //ADC E
                let a = self.a();
                let r = self.e();
                let a = self.add_flags(a, r, true);
                self.set_a(a);
                4
            }
            0x8c => {
                //ADC H
                let a = self.a();
                let r = self.hx(prefix);
                let a = self.add_flags(a, r, true);
                self.set_a(a);
                4
            }
            0x8d => {
                //ADC L
                let a = self.a();
                let r = self.lx(prefix);
                let a = self.add_flags(a, r, true);
                self.set_a(a);
                4
            }
            0x8e => {
                //ADC (HL)
                let a = self.a();
                let (addr, t) = self.hlx_addr(prefix, bus);
                let r = bus.peek(addr);
                let a = self.add_flags(a, r, true);
                self.set_a(a);
                4 + t
            }
            0x8f => {
                //ADC A
                let a = self.a();
                let a = self.add_flags(a, a, true);
                self.set_a(a);
                4
            }
            0x90 => {
                //SUB B
                let a = self.a();
                let r = self.b();
                let a = self.sub_flags(a, r, false);
                self.set_a(a);
                4
            }
            0x91 => {
                //SUB C
                let a = self.a();
                let r = self.c();
                let a = self.sub_flags(a, r, false);
                self.set_a(a);
                4
            }
            0x92 => {
                //SUB D
                let a = self.a();
                let r = self.d();
                let a = self.sub_flags(a, r, false);
                self.set_a(a);
                4
            }
            0x93 => {
                //SUB E
                let a = self.a();
                let r = self.e();
                let a = self.sub_flags(a, r, false);
                self.set_a(a);
                4
            }
            0x94 => {
                //SUB H
                let a = self.a();
                let r = self.hx(prefix);
                let a = self.sub_flags(a, r, false);
                self.set_a(a);
                4
            }
            0x95 => {
                //SUB L
                let a = self.a();
                let r = self.lx(prefix);
                let a = self.sub_flags(a, r, false);
                self.set_a(a);
                4
            }
            0x96 => {
                //SUB (HL)
                let a = self.a();
                let (addr, t) = self.hlx_addr(prefix, bus);
                let r = bus.peek(addr);
                let a = self.sub_flags(a, r, false);
                self.set_a(a);
                4 + t
            }
            0x97 => {
                //SUB A
                let a = self.a();
                let a = self.sub_flags(a, a, false);
                self.set_a(a);
                4
            }
            0x98 => {
                //SBC B
                let a = self.a();
                let r = self.b();
                let a = self.sub_flags(a, r, true);
                self.set_a(a);
                4
            }
            0x99 => {
                //SBC C
                let a = self.a();
                let r = self.c();
                let a = self.sub_flags(a, r, true);
                self.set_a(a);
                4
            }
            0x9a => {
                //SBC D
                let a = self.a();
                let r = self.d();
                let a = self.sub_flags(a, r, true);
                self.set_a(a);
                4
            }
            0x9b => {
                //SBC E
                let a = self.a();
                let r = self.e();
                let a = self.sub_flags(a, r, true);
                self.set_a(a);
                4
            }
            0x9c => {
                //SBC H
                let a = self.a();
                let r = self.hx(prefix);
                let a = self.sub_flags(a, r, true);
                self.set_a(a);
                4
            }
            0x9d => {
                //SBC L
                let a = self.a();
                let r = self.lx(prefix);
                let a = self.sub_flags(a, r, true);
                self.set_a(a);
                4
            }
            0x9e => {
                //SBC (HL)
                let a = self.a();
                let (addr, t) = self.hlx_addr(prefix, bus);
                let r = bus.peek(addr);
                let a = self.sub_flags(a, r, true);
                self.set_a(a);
                4 + t
            }
            0x9f => {
                //SBC A
                let a = self.a();
                let a = self.sub_flags(a, a, true);
                self.set_a(a);
                4
            }
            0xa0 => {
                //AND B
                let a = self.a();
                let r = self.b();
                let a = self.and_flags(a, r);
                self.set_a(a);
                4
            }
            0xa1 => {
                //AND C
                let a = self.a();
                let r = self.c();
                let a = self.and_flags(a, r);
                self.set_a(a);
                4
            }
            0xa2 => {
                //AND D
                let a = self.a();
                let r = self.d();
                let a = self.and_flags(a, r);
                self.set_a(a);
                4
            }
            0xa3 => {
                //AND E
                let a = self.a();
                let r = self.e();
                let a = self.and_flags(a, r);
                self.set_a(a);
                4
            }
            0xa4 => {
                //AND H
                let a = self.a();
                let r = self.hx(prefix);
                let a = self.and_flags(a, r);
                self.set_a(a);
                4
            }
            0xa5 => {
                //AND L
                let a = self.a();
                let r = self.lx(prefix);
                let a = self.and_flags(a, r);
                self.set_a(a);
                4
            }
            0xa6 => {
                //AND (HL)
                let a = self.a();
                let (addr, t) = self.hlx_addr(prefix, bus);
                let r = bus.peek(addr);
                let a = self.and_flags(a, r);
                self.set_a(a);
                4 + t
            }
            0xa7 => {
                //AND A
                let a = self.a();
                let a = self.and_flags(a, a);
                self.set_a(a);
                4
            }
            0xa8 => {
                //XOR B
                let a = self.a();
                let r = self.b();
                let a = self.xor_flags(a, r);
                self.set_a(a);
                4
            }
            0xa9 => {
                //XOR C
                let a = self.a();
                let r = self.c();
                let a = self.xor_flags(a, r);
                self.set_a(a);
                4
            }
            0xaa => {
                //XOR D
                let a = self.a();
                let r = self.d();
                let a = self.xor_flags(a, r);
                self.set_a(a);
                4
            }
            0xab => {
                //XOR E
                let a = self.a();
                let r = self.e();
                let a = self.xor_flags(a, r);
                self.set_a(a);
                4
            }
            0xac => {
                //XOR H
                let a = self.a();
                let r = self.hx(prefix);
                let a = self.xor_flags(a, r);
                self.set_a(a);
                4
            }
            0xad => {
                //XOR L
                let a = self.a();
                let r = self.lx(prefix);
                let a = self.xor_flags(a, r);
                self.set_a(a);
                4
            }
            0xae => {
                //XOR (HL)
                let a = self.a();
                let (addr, t) = self.hlx_addr(prefix, bus);
                let r = bus.peek(addr);
                let a = self.xor_flags(a, r);
                self.set_a(a);
                4 + t
            }
            0xaf => {
                //XOR A
                let a = self.a();
                let a = self.xor_flags(a, a);
                self.set_a(a);
                4
            }
            0xb0 => {
                //OR B
                let a = self.a();
                let r = self.b();
                let a = self.or_flags(a, r);
                self.set_a(a);
                4
            }
            0xb1 => {
                //OR C
                let a = self.a();
                let r = self.c();
                let a = self.or_flags(a, r);
                self.set_a(a);
                4
            }
            0xb2 => {
                //OR D
                let a = self.a();
                let r = self.d();
                let a = self.or_flags(a, r);
                self.set_a(a);
                4
            }
            0xb3 => {
                //OR E
                let a = self.a();
                let r = self.e();
                let a = self.or_flags(a, r);
                self.set_a(a);
                4
            }
            0xb4 => {
                //OR H
                let a = self.a();
                let r = self.hx(prefix);
                let a = self.or_flags(a, r);
                self.set_a(a);
                4
            }
            0xb5 => {
                //OR L
                let a = self.a();
                let r = self.lx(prefix);
                let a = self.or_flags(a, r);
                self.set_a(a);
                4
            }
            0xb6 => {
                //OR (HL)
                let a = self.a();
                let (addr, t) = self.hlx_addr(prefix, bus);
                let r = bus.peek(addr);
                let a = self.or_flags(a, r);
                self.set_a(a);
                4 + t
            }
            0xb7 => {
                //OR A
                let a = self.a();
                let a = self.or_flags(a, a);
                self.set_a(a);
                4
            }
            0xb8 => {
                //CP B
                let a = self.a();
                let r = self.b();
                self.sub_flags(a, r, false);
                4
            }
            0xb9 => {
                //CP C
                let a = self.a();
                let r = self.c();
                self.sub_flags(a, r, false);
                4
            }
            0xba => {
                //CP D
                let a = self.a();
                let r = self.d();
                self.sub_flags(a, r, false);
                4
            }
            0xbb => {
                //CP E
                let a = self.a();
                let r = self.e();
                self.sub_flags(a, r, false);
                4
            }
            0xbc => {
                //CP H
                let a = self.a();
                let r = self.hx(prefix);
                self.sub_flags(a, r, false);
                4
            }
            0xbd => {
                //CP L
                let a = self.a();
                let r = self.lx(prefix);
                self.sub_flags(a, r, false);
                4
            }
            0xbe => {
                //CP (HL)
                let a = self.a();
                let (addr, t) = self.hlx_addr(prefix, bus);
                let r = bus.peek(addr);
                self.sub_flags(a, r, false);
                4 + t
            }
            0xbf => {
                //CP A
                let a = self.a();
                self.sub_flags(a, a, false);
                4
            }
            0xc0 => {
                //RET NZ
                if !flag8(self.f(), FLAG_Z) {
                    let pc = self.pop(bus);
                    self.pc.set(pc);
                    11
                } else {
                    5
                }
            }
            0xc1 => {
                //POP BC
                let bc = self.pop(bus);
                self.bc.set(bc);
                10
            }
            0xc2 => {
                //JP NZ,nn
                let addr = self.fetch_u16(bus);
                if !flag8(self.f(), FLAG_Z) {
                    self.pc.set(addr);
                }
                10
            }
            0xc3 => {
                //JP nn
                let pc = self.fetch_u16(bus);
                self.pc.set(pc);
                10
            }
            0xc4 => {
                //CALL NZ,nn
                let addr = self.fetch_u16(bus);
                if !flag8(self.f(), FLAG_Z) {
                    let pc = self.pc;
                    self.push(bus, pc);
                    self.pc.set(addr);
                    17
                } else {
                    10
                }
            }
            0xc5 => {
                //PUSH BC
                let bc = self.bc;
                self.push(bus, bc);
                11
            }
            0xc6 => {
                //ADD n
                let n = self.fetch(bus);
                let a = self.a();
                let a = self.add_flags(a, n, false);
                self.set_a(a);
                7
            }
            0xc7 => {
                //RST 00
                let pc = self.pc;
                self.push(bus, pc);
                self.pc.set(0x00);
                11
            }
            0xc8 => {
                //RET Z
                if flag8(self.f(), FLAG_Z) {
                    let pc = self.pop(bus);
                    self.pc.set(pc);
                    11
                } else {
                    5
                }
            }
            0xc9 => {
                //RET
                let pc = self.pop(bus);
                self.pc.set(pc);
                10
            }
            0xca => {
                //JP Z,nn
                let addr = self.fetch_u16(bus);
                if flag8(self.f(), FLAG_Z) {
                    self.pc.set(addr);
                }
                10
            }
            0xcb => self.exec_cb(prefix, bus),
            0xcc => {
                //CALL Z,nn
                let addr = self.fetch_u16(bus);
                if flag8(self.f(), FLAG_Z) {
                    let pc = self.pc;
                    self.push(bus, pc);
                    self.pc.set(addr);
                    17
                } else {
                    10
                }
            }
            0xcd => {
                //CALL nn
                let addr = self.fetch_u16(bus);
                let pc = self.pc;
                self.push(bus, pc);
                self.pc.set(addr);
                17
            }
            0xce => {
                //ADC n
                let n = self.fetch(bus);
                let a = self.a();
                let a = self.add_flags(a, n, true);
                self.set_a(a);
                7
            }
            0xcf => {
                //RST 08
                let pc = self.pc;
                self.push(bus, pc);
                self.pc.set(0x08);
                11
            }
            0xd0 => {
                //RET NC
                if !flag8(self.f(), FLAG_C) {
                    let pc = self.pop(bus);
                    self.pc.set(pc);
                    11
                } else {
                    5
                }
            }
            0xd1 => {
                //POP DE
                let de = self.pop(bus);
                self.de.set(de);
                10
            }
            0xd2 => {
                //JP NC,nn
                let addr = self.fetch_u16(bus);
                if !flag8(self.f(), FLAG_C) {
                    self.pc.set(addr);
                }
                10
            }
            0xd3 => {
                //OUT (n),A
                let n = self.fetch(bus);
                let a = self.a();
                let n = (u16::from(a) << 8) | u16::from(n);
                bus.do_out(n, a);
                11
            }
            0xd4 => {
                //CALL NC,nn
                let addr = self.fetch_u16(bus);
                if !flag8(self.f(), FLAG_C) {
                    let pc = self.pc;
                    self.push(bus, pc);
                    self.pc.set(addr);
                    17
                } else {
                    10
                }
            }
            0xd5 => {
                //PUSH DE
                let de = self.de;
                self.push(bus, de);
                11
            }
            0xd6 => {
                //SUB n
                let n = self.fetch(bus);
                let a = self.a();
                let a = self.sub_flags(a, n, false);
                self.set_a(a);
                7
            }
            0xd7 => {
                //RST 10
                let pc = self.pc;
                self.push(bus, pc);
                self.pc.set(0x10);
                11
            }
            0xd8 => {
                //RET C
                if flag8(self.f(), FLAG_C) {
                    let pc = self.pop(bus);
                    self.pc.set(pc);
                    11
                } else {
                    5
                }
            }
            0xd9 => {
                //EXX
                mem::swap(&mut self.bc, &mut self.bc_);
                mem::swap(&mut self.de, &mut self.de_);
                mem::swap(&mut self.hl, &mut self.hl_);
                4
            }
            0xda => {
                //JP C,nn
                let addr = self.fetch_u16(bus);
                if flag8(self.f(), FLAG_C) {
                    self.pc.set(addr);
                }
                10
            }
            0xdb => {
                //IN A,(n)
                let n = self.fetch(bus);
                let a = self.a();
                let port = (u16::from(a) << 8) | u16::from(n);
                let a = bus.do_in(port);
                self.set_a(a);
                11
            }
            0xdc => {
                //CALL C,nn
                let addr = self.fetch_u16(bus);
                if flag8(self.f(), FLAG_C) {
                    let pc = self.pc;
                    self.push(bus, pc);
                    self.pc.set(addr);
                    17
                } else {
                    10
                }
            }
            0xdd => {
                //IX prefix
                unreachable!();
            }
            0xde => {
                //SBC n
                let n = self.fetch(bus);
                let a = self.a();
                let a = self.sub_flags(a, n, true);
                self.set_a(a);
                7
            }
            0xdf => {
                //RST 18
                let pc = self.pc;
                self.push(bus, pc);
                self.pc.set(0x18);
                11
            }
            0xe0 => {
                //RET PO
                if !flag8(self.f(), FLAG_PV) {
                    let pc = self.pop(bus);
                    self.pc.set(pc);
                    11
                } else {
                    5
                }
            }
            0xe1 => {
                //POP HL
                let hl = self.pop(bus);
                self.hlx_mut(prefix).set(hl);
                10
            }
            0xe2 => {
                //JP PO,nn
                let addr = self.fetch_u16(bus);
                if !flag8(self.f(), FLAG_PV) {
                    self.pc.set(addr);
                }
                10
            }
            0xe3 => {
                //EX (SP),HL
                let x = bus.peek_u16(self.sp);
                bus.poke_u16(self.sp, self.hlx(prefix).as_u16());
                self.hlx_mut(prefix).set(x);
                19
            }
            0xe4 => {
                //CALL PO,nn
                let addr = self.fetch_u16(bus);
                if !flag8(self.f(), FLAG_PV) {
                    let pc = self.pc;
                    self.push(bus, pc);
                    self.pc.set(addr);
                    17
                } else {
                    10
                }
            }
            0xe5 => {
                //PUSH HL
                let hl = self.hlx(prefix);
                self.push(bus, hl);
                11
            }
            0xe6 => {
                //AND n
                let n = self.fetch(bus);
                let mut a = self.a();
                a = self.and_flags(a, n);
                self.set_a(a);
                7
            }
            0xe7 => {
                //RST 20
                let pc = self.pc;
                self.push(bus, pc);
                self.pc.set(0x20);
                11
            }
            0xe8 => {
                //RET PE
                if flag8(self.f(), FLAG_PV) {
                    let pc = self.pop(bus);
                    self.pc.set(pc);
                    11
                } else {
                    5
                }
            }
            0xe9 => {
                //JP (HL)
                self.pc = self.hlx(prefix);
                4
            }
            0xea => {
                //JP PE,nn
                let addr = self.fetch_u16(bus);
                if flag8(self.f(), FLAG_PV) {
                    self.pc.set(addr);
                }
                10
            }
            0xeb => {
                //EX DE,HL
                mem::swap(&mut self.de, &mut self.hl);
                4
            }
            0xec => {
                //CALL PE,nn
                let addr = self.fetch_u16(bus);
                if flag8(self.f(), FLAG_PV) {
                    let pc = self.pc;
                    self.push(bus, pc);
                    self.pc.set(addr);
                    17
                } else {
                    10
                }
            }
            0xed => self.exec_ed(prefix, bus),
            0xee => {
                //XOR n
                let n = self.fetch(bus);
                let a = self.a();
                let a = self.xor_flags(a, n);
                self.set_a(a);
                7
            }
            0xef => {
                //RST 28
                let pc = self.pc;
                self.push(bus, pc);
                self.pc.set(0x28);
                11
            }
            0xf0 => {
                //RET P
                if !flag8(self.f(), FLAG_S) {
                    let pc = self.pop(bus);
                    self.pc.set(pc);
                    11
                } else {
                    5
                }
            }
            0xf1 => {
                //POP AF
                let af = self.pop(bus);
                self.af.set(af);
                10
            }
            0xf2 => {
                //JP P,nn
                let addr = self.fetch_u16(bus);
                if !flag8(self.f(), FLAG_S) {
                    self.pc.set(addr);
                }
                10
            }
            0xf3 => {
                //DI
                self.iff1 = false;
                4
            }
            0xf4 => {
                //CALL P,nn
                let addr = self.fetch_u16(bus);
                if !flag8(self.f(), FLAG_S) {
                    let pc = self.pc;
                    self.push(bus, pc);
                    self.pc.set(addr);
                    17
                } else {
                    10
                }
            }
            0xf5 => {
                //PUSH AF
                let af = self.af;
                self.push(bus, af);
                11
            }
            0xf6 => {
                //OR n
                let n = self.fetch(bus);
                let mut a = self.a();
                a = self.or_flags(a, n);
                self.set_a(a);
                7
            }
            0xf7 => {
                //RST 30
                let pc = self.pc;
                self.push(bus, pc);
                self.pc.set(0x30);
                11
            }
            0xf8 => {
                //RET M
                if flag8(self.f(), FLAG_S) {
                    let pc = self.pop(bus);
                    self.pc.set(pc);
                    11
                } else {
                    5
                }
            }
            0xf9 => {
                //LD SP,HL
                self.sp = self.hlx(prefix);
                6
            }
            0xfa => {
                //JP M,nn
                let addr = self.fetch_u16(bus);
                if flag8(self.f(), FLAG_S) {
                    self.pc.set(addr);
                }
                10
            }
            0xfb => {
                //EI
                self.iff1 = true;
                4
            }
            0xfc => {
                //CALL M,nn
                let addr = self.fetch_u16(bus);
                if flag8(self.f(), FLAG_S) {
                    let pc = self.pc;
                    self.push(bus, pc);
                    self.pc.set(addr);
                    17
                } else {
                    10
                }
            }
            0xfd => {
                //IY prefix
                unreachable!();
            }
            0xfe => {
                //CP n
                let n = self.fetch(bus);
                let a = self.a();
                self.sub_flags(a, n, false);
                7
            }
            0xff => {
                //RST 38
                let pc = self.pc;
                self.push(bus, pc);
                self.pc.set(0x38);
                11
            }
        };
        t + t2
    }
}

#[cfg(feature = "dump_ops")]
mod dumps;
