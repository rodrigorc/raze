use std::mem::swap;
use std::io::{self, Read, Write};

mod r16;

use self::r16::R16;

pub trait Bus {
    fn peek(&mut self, addr: impl Into<u16>) -> u8;
    fn poke(&mut self, addr: impl Into<u16>, value: u8);
    fn do_in(&mut self, port: impl Into<u16>) -> u8;
    fn do_out(&mut self, port: impl Into<u16>, value: u8);

    fn peek_u16(&mut self, addr: impl Into<u16>) -> u16 {
        let addr = addr.into();
        let lo = self.peek(addr) as u16;
        let addr = addr.wrapping_add(1);
        let hi = self.peek(addr) as u16;
        (hi << 8) | lo
    }
    fn poke_u16(&mut self, addr: impl Into<u16>, data: u16) {
        let addr = addr.into();
        self.poke(addr, data as u8);
        let addr = addr.wrapping_add(1);
        self.poke(addr, (data >> 8) as u8);
    }
}

const FLAG_S  : u8 = 0b1000_0000;
const FLAG_Z  : u8 = 0b0100_0000;
const FLAG_Y : u8 = 0b0010_0000;
const FLAG_H  : u8 = 0b0001_0000;
const FLAG_X : u8 = 0b0000_1000;
const FLAG_PV : u8 = 0b0000_0100;
const FLAG_N  : u8 = 0b0000_0010;
const FLAG_C  : u8 = 0b0000_0001;

#[inline]
fn flag8(f: u8, bit: u8) -> bool {
    (f & bit) != 0
}
#[inline]
fn flag16(f: u16, bit: u16) -> bool {
    (f & bit) != 0
}
#[inline]
fn set_flag8(f: u8, bit: u8, set: bool) -> u8 {
    if set {
        f | bit
    } else {
        f & !bit
    }

}
#[inline]
fn parity(b: u8) -> bool {
    (b.count_ones()) % 2 == 0
}

#[inline]
fn carry8(a: u8, b: u8, c: u8) -> bool {
    let ma = flag8(a, 0x80);
    let mb = flag8(b, 0x80);
    let mc = flag8(c, 0x80);
    (mc && ma && mb) || (!mc && (ma || mb))
}
#[inline]
fn carry16(a: u16, b: u16, c: u16) -> bool {
    let ma = flag16(a, 0x8000);
    let mb = flag16(b, 0x8000);
    let mc = flag16(c, 0x8000);
    (mc && ma && mb) || (!mc && (ma || mb))
}
#[inline]
fn half_carry8(a: u8, b: u8, c: u8) -> bool {
    let ma = flag8(a, 0x08);
    let mb = flag8(b, 0x08);
    let mc = flag8(c, 0x08);
    (mc && ma && mb) || (!mc && (ma || mb))
}
#[inline]
fn half_carry16(a: u16, b: u16, c: u16) -> bool {
    let ma = flag16(a, 0x0800);
    let mb = flag16(b, 0x0800);
    let mc = flag16(c, 0x0800);
    (mc && ma && mb) || (!mc && (ma || mb))
}
#[inline]
fn overflow_add8(a: u8, b: u8, c: u8) -> bool {
    flag8(a, 0x80) == flag8(b, 0x80) && flag8(a, 0x80) != flag8(c, 0x80)
}
#[inline]
fn overflow_add16(a: u16, b: u16, c: u16) -> bool {
    flag16(a, 0x8000) == flag16(b, 0x8000) && flag16(a, 0x8000) != flag16(c, 0x8000)
}
#[inline]
fn overflow_sub8(a: u8, b: u8, c: u8) -> bool {
    flag8(a, 0x80) != flag8(b, 0x80) && flag8(a, 0x80) != flag8(c, 0x80)
}
#[inline]
fn overflow_sub16(a: u16, b: u16, c: u16) -> bool {
    flag16(a, 0x8000) != flag16(b, 0x8000) && flag16(a, 0x8000) != flag16(c, 0x8000)
}

#[inline]
fn set_flag_sz(f: u8, r: u8) -> u8 {
    let f = set_flag8(f, FLAG_Z, r == 0);
    const CF: u8 = FLAG_S | FLAG_X | FLAG_Y;
    (f & !CF) | (r & CF)
}

#[inline]
fn set_flag_szp(f: u8, r: u8) -> u8 {
    let f = set_flag8(f, FLAG_PV, parity(r));
    let f = set_flag_sz(f, r);
    f
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
enum InterruptMode {
    IM0, IM1, IM2,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
enum XYPrefix {
    None, IX, IY,
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
    af: R16, af_: R16,
    bc: R16, bc_: R16,
    de: R16, de_: R16,
    hl: R16, hl_: R16,
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
enum Direction { Inc, Dec }


#[derive(Debug)]
pub enum Z80FileVersion {
    V1, V2, V3(bool)
}

impl Z80 {
    pub fn new() -> Z80 {
        Z80 {
            pc: R16::default(),
            sp: R16::default(),
            af: R16::default(), af_: R16::default(),
            bc: R16::default(), bc_: R16::default(),
            de: R16::default(), de_: R16::default(),
            hl: R16::default(), hl_: R16::default(),
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
    #[allow(unused)]
    pub fn dump_regs(&self) {
        log!("PC {:04x}; AF {:04x}; BC {:04x}; DE {:04x}; HL {:04x}; IR {:02x}{:02x}",
                 self.pc.as_u16(),
                 self.af.as_u16() & 0xffd7,
                 self.bc.as_u16(), self.de.as_u16(), self.hl.as_u16(),
                 self.i, self.r());
    }
    pub fn save(&self, mut w: impl Write) -> io::Result<()> {
        let (next_op, iff1) = {
            match (self.next_op, self.iff1) {
                (NextOp::Interrupt, _) => (NextOp::Fetch, true),
                x => x,
            }
        };
        let data = [
            self.pc.lo(), self.pc.hi(),
            self.sp.lo(), self.sp.hi(),
            self.f(), self.a(),
            self.af_.lo(), self.af_.hi(),
            self.bc.lo(), self.bc.hi(),
            self.bc_.lo(), self.bc_.hi(),
            self.de.lo(), self.de.hi(),
            self.de_.lo(), self.de_.hi(),
            self.hl.lo(), self.hl.hi(),
            self.hl_.lo(), self.hl_.hi(),
            self.ix.lo(), self.ix.hi(),
            self.iy.lo(), self.iy.hi(),
            self.r(), self.i,
            iff1 as u8,
            self.im as u8,
            next_op as u8,
        ];
        w.write_all(&data)?;
        Ok(())
    }
    #[allow(unused)]
    pub fn load(&mut self, mut r: impl Read) -> io::Result<()> {
        let mut data = [0; 2 * 13 + 3];
        r.read_exact(&mut data)?;
        self.pc.set_lo(data[0]); self.pc.set_hi(data[1]);
        self.sp.set_lo(data[2]); self.sp.set_hi(data[3]);
        self.set_f(data[4]); self.set_a(data[5]);
        self.af_.set_lo(data[6]); self.af_.set_hi(data[7]);
        self.bc.set_lo(data[8]); self.bc.set_hi(data[9]);
        self.bc_.set_lo(data[10]); self.bc_.set_hi(data[11]);
        self.de.set_lo(data[12]); self.de.set_hi(data[13]);
        self.de_.set_lo(data[14]); self.de_.set_hi(data[15]);
        self.hl.set_lo(data[16]); self.hl.set_hi(data[17]);
        self.hl_.set_lo(data[18]); self.hl_.set_hi(data[19]);
        self.ix.set_lo(data[20]); self.ix.set_hi(data[21]);
        self.iy.set_lo(data[22]); self.iy.set_hi(data[23]);
        self.set_r(data[24]);
        self.i = data[25];
        self.iff1 = data[26] != 0;
        self.im = match data[27] { 0 => InterruptMode::IM0, 1 => InterruptMode::IM1, 2 => InterruptMode::IM2, _ => panic!("invalid IM") };
        self.next_op = match data[28] { 0=> NextOp::Fetch, 1 => NextOp::Fetch, 2 => NextOp::Halt, _ => panic!("invalid NextOp") };
        Ok(())
    }
    pub fn load_format_z80(&mut self, data: &[u8]) -> Z80FileVersion {
        self.af.set_hi(data[0]);
        self.af.set_lo(data[1]);
        self.bc.set_lo(data[2]);
        self.bc.set_hi(data[3]);
        self.hl.set_lo(data[4]);
        self.hl.set_hi(data[5]);
        self.pc.set_lo(data[6]);
        self.pc.set_hi(data[7]);
        self.sp.set_lo(data[8]);
        self.sp.set_hi(data[9]);
        self.i = data[10];
        self.r_ = data[11] & 0x7f;
        self.r7 = (data[12] & 1) != 0;
        self.de.set_lo(data[13]);
        self.de.set_hi(data[14]);
        self.bc_.set_lo(data[15]);
        self.bc_.set_hi(data[16]);
        self.de_.set_lo(data[17]);
        self.de_.set_hi(data[18]);
        self.hl_.set_lo(data[19]);
        self.hl_.set_hi(data[20]);
        self.af_.set_hi(data[21]);
        self.af_.set_lo(data[22]);
        self.iy.set_lo(data[23]);
        self.iy.set_hi(data[24]);
        self.ix.set_lo(data[25]);
        self.ix.set_hi(data[26]);
        self.iff1 = data[27] != 0;
        //self.iff2 = data[28];
        self.im = match data[29] & 0x03 {
            1 => InterruptMode::IM1,
            2 => InterruptMode::IM2,
            _ => InterruptMode::IM0,
        };
        self.next_op = NextOp::Fetch;

        let res = if self.pc.as_u16() == 0 { //v. 2 or 3
            let extra = (data[30] as u16) | ((data[31] as u16) << 8);
            self.pc.set_lo(data[32]);
            self.pc.set_hi(data[33]);
            match extra {
                23 => Z80FileVersion::V2,
                54 => Z80FileVersion::V3(false),
                55 => Z80FileVersion::V3(true),
                _ => panic!("Unknown Z80 file format"),
            }
        } else {
            Z80FileVersion::V1
        };
        res
    }
    pub fn interrupt(&mut self) {
        if !self.iff1 {
            return;
        }
        self.next_op = NextOp::Interrupt;
        self.iff1 = false;
    }
    #[inline]
    fn r(&self) -> u8 {
        (self.r_ & 0x7f) | if self.r7 { 0x80 } else { 0x00 }
    }
    fn set_r(&mut self, r: u8) {
        self.r_ = r;
        self.r7 = flag8(r, 0x80);
    }
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
    fn inc_r(&mut self) {
        self.r_ = self.r_.wrapping_add(1);
    }
    fn fetch(&mut self, bus: &mut impl Bus) -> u8 {
        let c = bus.peek(self.pc);
        self.pc += 1;
        c
    }
    fn fetch_u16(&mut self, bus: &mut impl Bus) -> u16 {
        let l = bus.peek(self.pc) as u16;
        self.pc += 1;
        let h = bus.peek(self.pc) as u16;
        self.pc += 1;
        h << 8 | l
    }
    fn push(&mut self, bus: &mut impl Bus, x: impl Into<u16>) {
        let x = x.into();
        self.sp -= 1;
        bus.poke(self.sp, (x >> 8) as u8);
        self.sp -= 1;
        bus.poke(self.sp, x as u8);
    }
    fn pop(&mut self, bus: &mut impl Bus) -> u16 {
        let x = bus.peek_u16(self.sp);
        self.sp += 2;
        x
    }
    fn reg_by_num_addr(&mut self, prefix: XYPrefix, r: u8, bus: &mut impl Bus, addr: u16) -> u8 {
        match r {
            0 => self.bc.hi(),
            1 => self.bc.lo(),
            2 => self.de.hi(),
            3 => self.de.lo(),
            4 => self.hlx(prefix).hi(),
            5 => self.hlx(prefix).lo(),
            6 => bus.peek(addr),
            7 => self.a(),
            _ => unreachable!("unknown reg_by_num {}", r),
        }
    }
    fn set_reg_by_num_addr(&mut self, prefix: XYPrefix, r: u8, bus: &mut impl Bus, b: u8, addr: u16) {
        match r {
            0 => self.bc.set_hi(b),
            1 => self.bc.set_lo(b),
            2 => self.de.set_hi(b),
            3 => self.de.set_lo(b),
            4 => self.hlx(prefix).set_hi(b),
            5 => self.hlx(prefix).set_lo(b),
            6 => bus.poke(addr, b),
            7 => self.set_a(b),
            _ => unreachable!("unknown reg_by_num {}", r),
        }
    }
    fn reg_by_num(&mut self, prefix: XYPrefix, r: u8, bus: &mut impl Bus) -> (u8, u32) {
        let (addr, t) = if r == 6 { self.hlx_addr(prefix, bus) } else { (0,0) };
        (self.reg_by_num_addr(prefix, r, bus, addr), t)
    }
    fn set_reg_by_num(&mut self, prefix: XYPrefix, r: u8, bus: &mut impl Bus, b: u8) -> u32 {
        let (addr, t) = if r == 6 { self.hlx_addr(prefix, bus) } else { (0,0) };
        self.set_reg_by_num_addr(prefix, r, bus, b, addr);
        t
    }
    fn reg_by_num_no_pre(&mut self, r: u8) -> u8 {
        match r {
            0 => self.bc.hi(),
            1 => self.bc.lo(),
            2 => self.de.hi(),
            3 => self.de.lo(),
            4 => self.hl.hi(),
            5 => self.hl.lo(),
            //6 is impossible
            7 => self.a(),
            _ => unreachable!("unknown reg_by_num {}", r),
        }
    }
    fn set_reg_by_num_no_pre(&mut self, r: u8, b: u8) {
        match r {
            0 => self.bc.set_hi(b),
            1 => self.bc.set_lo(b),
            2 => self.de.set_hi(b),
            3 => self.de.set_lo(b),
            4 => self.hl.set_hi(b),
            5 => self.hl.set_lo(b),
            //6 is impossible
            7 => self.set_a(b),
            _ => unreachable!("unknown reg_by_num {}", r),
        }
    }
    fn hlx(&mut self, prefix: XYPrefix) -> &mut R16 {
        match prefix {
            XYPrefix::None => &mut self.hl,
            XYPrefix::IX => &mut self.ix,
            XYPrefix::IY => &mut self.iy,
        }
    }
    //Returns the (address, extra_T_states)
    fn hlx_addr(&mut self, prefix: XYPrefix, bus: &mut impl Bus) -> (u16, u32) {
        match prefix {
            XYPrefix::None => (self.hl.as_u16(), 0),
            XYPrefix::IX => {
                let d = self.fetch(bus);
                (self.ix.as_u16().wrapping_add(d as i8 as i16 as u16), 8)
            }
            XYPrefix::IY => {
                let d = self.fetch(bus);
                (self.iy.as_u16().wrapping_add(d as i8 as i16 as u16), 8)
            }
        }
    }
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
    fn sbc16_flags(&mut self, a: u16, mut b: u16) -> u16 {
        let mut f = self.f();
        if flag8(f, FLAG_C) {
            b = b.wrapping_add(1);
        }
        let r = a.wrapping_sub(b);
        f = set_flag8(f, FLAG_N, true);
        f = set_flag8(f, FLAG_C, carry16(r, b, a));
        f = set_flag8(f, FLAG_PV, overflow_sub16(a, b, r));
        f = set_flag8(f, FLAG_Z, r == 0);
        f = set_flag8(f, FLAG_S, flag16(r, 0x8000));
        f = set_flag8(f, FLAG_H, half_carry16(r, b, a));
        self.set_f(f);
        r
    }
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
    fn adc16_flags(&mut self, a: u16, mut b: u16) -> u16 {
        let mut f = self.f();
        if flag8(f, FLAG_C) {
            b = b.wrapping_add(1);
        }
        let r = a.wrapping_add(b);
        f = set_flag8(f, FLAG_N, false);
        f = set_flag8(f, FLAG_C, carry16(a, b, r));
        f = set_flag8(f, FLAG_PV, overflow_add16(a, b, r));
        f = set_flag8(f, FLAG_Z, r == 0);
        f = set_flag8(f, FLAG_S, flag16(r, 0x8000));
        f = set_flag8(f, FLAG_H, half_carry16(a, b, r));
        self.set_f(f);
        r
    }
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
    fn ini_ind(&mut self, dir: Direction, bus: &mut impl Bus) -> u8 {
        let x = bus.do_in(self.bc.as_u16());
        bus.poke(self.hl, x);
        match dir {
            Direction::Inc => self.hl += 1,
            Direction::Dec => self.hl -= 1,
        };
        let b = self.bc.hi().wrapping_sub(1);
        self.bc.set_hi(b);
        let mut f = self.f();
        f = set_flag8(f, FLAG_N | FLAG_S, flag8(b, 0x80));
        f = set_flag8(f, FLAG_Z, b == 0);
        self.set_f(f);
        b
    }
    fn outi_outd(&mut self, dir: Direction, bus: &mut impl Bus) -> u8 {
        let x = bus.peek(self.hl);
        bus.do_out(self.bc.as_u16(), x);
        match dir {
            Direction::Inc => self.hl += 1,
            Direction::Dec => self.hl -= 1,
        };
        let b = self.bc.hi().wrapping_sub(1);
        self.bc.set_hi(b);
        let mut f = self.f();
        f = set_flag8(f, FLAG_N | FLAG_S, flag8(b, 0x80));
        f = set_flag8(f, FLAG_Z, b == 0);
        self.set_f(f);
        b
    }
    fn daa(&mut self) {
        let a = self.a();
        let mut f = self.f();
        const O : u8 = 0;
        const N : u8 = FLAG_N;
        const C : u8 = FLAG_C;
        const H : u8 = FLAG_H;
        const CH : u8 = FLAG_C | FLAG_H;
        const NH : u8 = FLAG_N | FLAG_H;
        const NC : u8 = FLAG_N | FLAG_C;
        const NCH : u8 = FLAG_N | FLAG_C | FLAG_H;
        let (plus_a, new_c, new_h) =
            match (f & (FLAG_N | FLAG_C | FLAG_H), a >> 4, a & 0x0f) {
                (O, 0x0..=0x9, 0x0..=0x9) => (0x00, false, false),
                (O, 0x0..=0x8, 0xa..=0xf) => (0x06, false, true),
                (O, _, 0x0..=0x9) => (0x60, true, false),
                (O, _, 0xa..=0xf) => (0x66, true, true),

                (H,  0x0..=0x9, 0x0..=0x9) => (0x06, false, false),
                (H,  0x0..=0x8, 0xa..=0xf) => (0x06, false, true),
                (H,  _, 0x0..=0x9) => (0x66, true, false),
                (H,  _, 0xa..=0xf) => (0x66, true, true),

                (C, _, 0x0..=0x9) => (0x60, true, false),
                (C, _, 0xa..=0xf) => (0x66, true, true),

                (CH,  _, 0x0..=0x9) => (0x66, true, false),
                (CH,  _, 0xa..=0xf) => (0x66, true, true),

                (N, 0x0..=0x9, 0x0..=0x9) => (0x00, false, false),
                (N, 0x0..=0x8, 0xa..=0xf) => (0xfa, false, false),
                (N, _, 0x0..=0x9) => (0xa0, true, false),
                (N, _, 0xa..=0xf) => (0x9a, true, false),

                (NH,  0x0..=0x9, 0x0..=0x5) => (0xfa, false, true),
                (NH,  _, 0x0..=0x5) => (0x9a, true, true),
                (NH,  0x0..=0x8, 0x6..=0xf) => (0xfa, false, false),
                (NH,  0xa..=0xf, 0x6..=0xf) => (0x9a, true, false),
                (NH,  0x9..=0x9, 0x6..=0x9) => (0xfa, false, false),
                (NH,  0x9..=0x9, 0xa..=0xf) => (0x9a, true, false),

                (NC, _, 0x0..=0x9) => (0xa0, true, false),
                (NC, _, 0xa..=0xf) => (0x9a, true, false),

                (NCH,  _, 0x0..=0x5) => (0x9a, true, true),
                (NCH,  _, 0x6..=0xf) => (0x9a, true, false),

                _ => unreachable!()
            };
        let a = a.wrapping_add(plus_a);
        f = set_flag8(f, FLAG_C, new_c);
        f = set_flag8(f, FLAG_H, new_h);
        f = set_flag_szp(f, a);
        self.set_a(a);
        self.set_f(f);
    }
    pub fn exec(&mut self, bus: &mut impl Bus) -> u32 {
        let mut c = match self.next_op {
            NextOp::Fetch => {
                self.inc_r();
                self.fetch(bus)
            }
            NextOp::Halt => {
                self.inc_r();
                0x00 //NOP
            }
            NextOp::Interrupt => {
                self.inc_r();
                self.next_op = NextOp::Fetch;
                match self.im {
                    InterruptMode::IM0 => {
                        log!("IM0 interrupt!");
                        0x00 //NOP
                    }
                    InterruptMode::IM1 => {
                        self.iff1 = false;
                        0xff //RST 38
                    }
                    InterruptMode::IM2 => {
                        //assume 0xff in the data bus
                        let v = ((self.i as u16) << 8) | 0xff;
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
        let c = loop {
            c = match c {
                0xdd => {
                    prefix = XYPrefix::IX;
                    t += 4;
                    self.inc_r();
                    self.fetch(bus)
                }
                0xfd => {
                    prefix = XYPrefix::IY;
                    t += 4;
                    self.inc_r();
                    self.fetch(bus)
                }
                _ => break c
            };
        };
        t + match c {
            0xcb => { self.exec_cb(prefix, bus) }
            0xed => { self.exec_ed(prefix, bus) }

            0x00 => { //NOP
                4
            }
            0x01 => { //LD BC,nn
                let d = self.fetch_u16(bus);
                self.bc.set(d);
                10
            }
            0x02 => { //LD (BC),A
                let a = self.a();
                bus.poke(self.bc, a);
                7
            }
            0x03 => { //INC BC
                self.bc += 1;
                6
            }
            0x04 => { //INC B
                let mut r = self.bc.hi();
                r = self.inc_flags(r);
                self.bc.set_hi(r);
                4
            }
            0x05 => { //DEC B
                let mut r = self.bc.hi();
                r = self.dec_flags(r);
                self.bc.set_hi(r);
                4
            }
            0x06 => { //LD B,n
                let n = self.fetch(bus);
                self.bc.set_hi(n);
                7
            }
            0x07 => { //RLCA
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
            0x08 => { //EX AF,AF
                swap(&mut self.af, &mut self.af_);
                4
            }
            0x09 => { //ADD HL,BC
                let mut hl = self.hlx(prefix).as_u16();
                let bc = self.bc.as_u16();
                hl = self.add16_flags(hl, bc);
                self.hlx(prefix).set(hl);
                11
            }
            0x0a => { //LD A,(BC)
                let a = bus.peek(self.bc);
                self.set_a(a);
                7
            }
            0x0b => { //DEC BC
                self.bc -= 1;
                6
            }
            0x0c => { //INC C
                let mut r = self.bc.lo();
                r = self.inc_flags(r);
                self.bc.set_lo(r);
                4
            }
            0x0d => { //DEC C
                let mut r = self.bc.lo();
                r = self.dec_flags(r);
                self.bc.set_lo(r);
                4
            }
            0x0e => { //LD C,n
                let n = self.fetch(bus);
                self.bc.set_lo(n);
                7
            }
            0x0f => { //RRCA
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
            0x10 => { //DJNZ d
                let d = self.fetch(bus);
                let mut b = self.bc.hi();
                b = b.wrapping_sub(1);
                self.bc.set_hi(b);
                if b != 0 {
                    self.pc += d as i8 as i16 as u16;
                    13
                } else {
                    8
                }
            }
            0x11 => { //LD DE,nn
                let nn = self.fetch_u16(bus);
                self.de.set(nn);
                10
            }
            0x12 => { //LD (DE),A
                let a = self.a();
                bus.poke(self.de, a);
                7
            }
            0x13 => { //INC DE
                self.de += 1;
                6
            }
            0x14 => { //INC D
                let mut r = self.de.hi();
                r = self.inc_flags(r);
                self.de.set_hi(r);
                4
            }
            0x15 => { //DEC D
                let mut r = self.de.hi();
                r = self.dec_flags(r);
                self.de.set_hi(r);
                4
            }
            0x16 => { //LD D,n
                let n = self.fetch(bus);
                self.de.set_hi(n);
                7
            }
            0x17 => { //RLA
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
            0x18 => { //JR d
                let d = self.fetch(bus);
                self.pc += d as i8 as i16 as u16;
                12
            }
            0x19 => { //ADD HL,DE
                let mut hl = self.hlx(prefix).as_u16();
                let de = self.de.as_u16();
                hl = self.add16_flags(hl, de);
                self.hlx(prefix).set(hl);
                11
            }
            0x1a => { //LD A,(DE)
                let a = bus.peek(self.de);
                self.set_a(a);
                7
            }
            0x1b => { //DEC DE
                self.de -= 1;
                6
            }
            0x1c => { //INC E
                let mut r = self.de.lo();
                r = self.inc_flags(r);
                self.de.set_lo(r);
                4
            }
            0x1d => { //DEC E
                let mut r = self.de.lo();
                r = self.dec_flags(r);
                self.de.set_lo(r);
                4
            }
            0x1e => { //LD E,n
                let n = self.fetch(bus);
                self.de.set_lo(n);
                7
            }
            0x1f => { //RRA
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
            0x20 => { //JR NZ,d
                let d = self.fetch(bus);
                if !flag8(self.f(), FLAG_Z) {
                    self.pc += d as i8 as i16 as u16;
                }
                12
             }
            0x21 => { //LD HL,nn
                let d = self.fetch_u16(bus);
                self.hlx(prefix).set(d);
                10
            }
            0x22 => { //LD (nn),HL
                let addr = self.fetch_u16(bus);
                bus.poke_u16(addr, self.hlx(prefix).as_u16());
                16
            }
            0x23 => { //INC HL
                *self.hlx(prefix) += 1;
                6
            }
            0x24 => { //INC H
                let mut r = self.hlx(prefix).hi();
                r = self.inc_flags(r);
                self.hlx(prefix).set_hi(r);
                4
            }
            0x25 => { //DEC H
                let mut r = self.hlx(prefix).hi();
                r = self.dec_flags(r);
                self.hlx(prefix).set_hi(r);
                4
            }
            0x26 => { //LD H,n
                let n = self.fetch(bus);
                self.hlx(prefix).set_hi(n);
                7
            }
            0x27 => { //DAA
                self.daa();
                4
            }
            0x28 => { //JR Z,d
                let d = self.fetch(bus);
                if flag8(self.f(), FLAG_Z) {
                    self.pc += d as i8 as i16 as u16;
                }
                12
            }
            0x29 => { //ADD HL,HL
                let mut hl = self.hlx(prefix).as_u16();
                hl = self.add16_flags(hl, hl);
                self.hlx(prefix).set(hl);
                11
            }
            0x2a => { //LD HL,(nn)
                let addr = self.fetch_u16(bus);
                let d = bus.peek_u16(addr);
                self.hlx(prefix).set(d);
                16
            }
            0x2b => { //DEC HL
                *self.hlx(prefix) -= 1;
                6
            }
            0x2c => { //INC L
                let mut r = self.hlx(prefix).lo();
                r = self.inc_flags(r);
                self.hlx(prefix).set_lo(r);
                4
            }
            0x2d => { //DEC L
                let mut r = self.hlx(prefix).lo();
                r = self.dec_flags(r);
                self.hlx(prefix).set_lo(r);
                4
            }
            0x2e => { //LD L,n
                let n = self.fetch(bus);
                self.hlx(prefix).set_lo(n);
                7
            }
            0x2f => { //CPL
                let mut a = self.a();
                let mut f = self.f();
                a ^= 0xff;
                f = set_flag8(f, FLAG_H, true);
                f = set_flag8(f, FLAG_N, true);
                self.set_a(a);
                self.set_f(f);
                4
            }
            0x30 => { //JR NC,d
                let d = self.fetch(bus);
                if !flag8(self.f(), FLAG_C) {
                    self.pc += d as i8 as i16 as u16;
                }
                12
            }
            0x31 => { //LD SP,nn
                let nn = self.fetch_u16(bus);
                self.sp.set(nn);
                10
            }
            0x32 => { //LD (nn),A
                let addr = self.fetch_u16(bus);
                bus.poke(addr, self.a());
                13
            }
            0x33 => { //INC SP
                self.sp += 1;
                6
            }
            0x34 => { //INC (HL)
                let (addr, t) = self.hlx_addr(prefix, bus);
                let mut b = bus.peek(addr);
                b = self.inc_flags(b);
                bus.poke(addr, b);
                11 + t
            }
            0x35 => { //DEC (HL)
                let (addr, t) = self.hlx_addr(prefix, bus);
                let mut b = bus.peek(addr);
                b = self.dec_flags(b);
                bus.poke(addr, b);
                11 + t
            }
            0x36 => { //LD (HL),n
                let (addr, t) = self.hlx_addr(prefix, bus);
                let n = self.fetch(bus);
                bus.poke(addr, n);
                10 + t
            }
            0x37 => { //SCF
                let mut f = self.f();
                f = set_flag8(f, FLAG_N, false);
                f = set_flag8(f, FLAG_H, false);
                f = set_flag8(f, FLAG_C, true);
                self.set_f(f);
                4
            }
            0x38 => { //JR C,d
                let d = self.fetch(bus);
                if flag8(self.f(), FLAG_C) {
                    self.pc += d as i8 as i16 as u16;
                }
                12
            }
            0x39 => { //ADD HL,SP
                let mut hl = self.hlx(prefix).as_u16();
                let sp = self.sp.as_u16();
                hl = self.add16_flags(hl, sp);
                self.hlx(prefix).set(hl);
                11
            }
            0x3a => { //LD A,(nn)
                let addr = self.fetch_u16(bus);
                let x = bus.peek(addr);
                self.set_a(x);
                13
            }
            0x3b => { //DEC SP
                self.sp -= 1;
                6
            }
            0x3c => { //INC A
                let mut r = self.a();
                r = self.inc_flags(r);
                self.set_a(r);
                4
            }
            0x3d => { //DEC A
                let mut r = self.a();
                r = self.dec_flags(r);
                self.set_a(r);
                4
            }
            0x3e => { //LD A,n
                let n = self.fetch(bus);
                self.set_a(n);
                7
            }
            0x3f => { //CCF
                let mut f = self.f();
                let c = flag8(f, FLAG_C);
                f = set_flag8(f, FLAG_N, false);
                f = set_flag8(f, FLAG_H, c);
                f = set_flag8(f, FLAG_C, !c);
                self.set_f(f);
                4
            }
            0x76 => { //HALT
                if !self.iff1 {
                    log!("DI/HALT deadlock!");
                }
                self.next_op = NextOp::Halt;
                4
            }
            0xc0 => { //RET NZ
                if !flag8(self.f(), FLAG_Z) {
                    let pc = self.pop(bus);
                    self.pc.set(pc);
                    11
                } else {
                    5
                }
            }
            0xc1 => { //POP BC
                let bc = self.pop(bus);
                self.bc.set(bc);
                10
            }
            0xc2 => { //JP NZ,nn
                let addr = self.fetch_u16(bus);
                if !flag8(self.f(), FLAG_Z) {
                    self.pc.set(addr);
                }
                10
            }
            0xc3 => { //JP nn
                let pc = self.fetch_u16(bus);
                self.pc.set(pc);
                10
            }
            0xc4 => { //CALL NZ,nn
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
            0xc5 => { //PUSH BC
                let bc = self.bc;
                self.push(bus, bc);
                11
            }
            0xc6 => { //ADD n
                let n = self.fetch(bus);
                let a = self.a();
                let a = self.add_flags(a, n, false);
                self.set_a(a);
                7
            }
            0xc7 => { //RST 00
                let pc = self.pc;
                self.push(bus, pc);
                self.pc.set(0x00);
                11
            }
            0xc8 => { //RET Z
                if flag8(self.f(), FLAG_Z) {
                    let pc = self.pop(bus);
                    self.pc.set(pc);
                    11
                } else {
                    5
                }
            }
            0xc9 => { //RET
                let pc = self.pop(bus);
                self.pc.set(pc);
                10
            }
            0xca => { //JP Z,nn
                let addr = self.fetch_u16(bus);
                if flag8(self.f(), FLAG_Z) {
                    self.pc.set(addr);
                }
                10
            }
            0xcc => { //CALL Z,nn
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
            0xcd => { //CALL nn
                let addr = self.fetch_u16(bus);
                let pc = self.pc;
                self.push(bus, pc);
                self.pc.set(addr);
                17
            }
            0xce => { //ADC n
                let n = self.fetch(bus);
                let a = self.a();
                let a = self.add_flags(a, n, true);
                self.set_a(a);
                7
            }
            0xcf => { //RST 08
                let pc = self.pc;
                self.push(bus, pc);
                self.pc.set(0x08);
                11
            }
            0xd0 => { //RET NC
                if !flag8(self.f(), FLAG_C) {
                    let pc = self.pop(bus);
                    self.pc.set(pc);
                    11
                } else {
                    5
                }
            }
            0xd1 => { //POP DE
                let de = self.pop(bus);
                self.de.set(de);
                10
            }
            0xd2 => { //JP NC,nn
                let addr = self.fetch_u16(bus);
                if !flag8(self.f(), FLAG_C) {
                    self.pc.set(addr);
                }
                10
            }
            0xd3 => { //OUT (n),A
                let n = self.fetch(bus);
                let a = self.a();
                let n = ((a as u16) << 8) | n as u16;
                bus.do_out(n, a);
                11
            }
            0xd4 => { //CALL NC,nn
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
            0xd5 => { //PUSH DE
                let de = self.de;
                self.push(bus, de);
                11
            }
            0xd6 => { //SUB n
                let n = self.fetch(bus);
                let a = self.a();
                let a = self.sub_flags(a, n, false);
                self.set_a(a);
                7
            }
            0xd7 => { //RST 10
                let pc = self.pc;
                self.push(bus, pc);
                self.pc.set(0x10);
                11
            }
            0xd8 => { //RET C
                if flag8(self.f(), FLAG_C) {
                    let pc = self.pop(bus);
                    self.pc.set(pc);
                    11
                } else {
                    5
                }
            }
            0xd9 => { //EXX
                swap(&mut self.bc, &mut self.bc_);
                swap(&mut self.de, &mut self.de_);
                swap(&mut self.hl, &mut self.hl_);
                4
            }
            0xda => { //JP C,nn
                let addr = self.fetch_u16(bus);
                if flag8(self.f(), FLAG_C) {
                    self.pc.set(addr);
                }
                10
            }
            0xdb => { //IN A,(n)
                let n = self.fetch(bus);
                let a = self.a();
                let port = ((a as u16) << 8) | (n as u16);
                let a = bus.do_in(port);
                self.set_a(a);
                11
            }
            0xdc => { //CALL C,nn
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
            0xde => { //SBC n
                let mut n = self.fetch(bus);
                let a = self.a();
                let a = self.sub_flags(a, n, true);
                self.set_a(a);
                7
            }
            0xdf => { //RST 18
                let pc = self.pc;
                self.push(bus, pc);
                self.pc.set(0x18);
                11
            }
            0xe0 => { //RET PO
                if !flag8(self.f(), FLAG_PV) {
                    let pc = self.pop(bus);
                    self.pc.set(pc);
                    11
                } else {
                    5
                }
            }
            0xe1 => { //POP HL
                let hl = self.pop(bus);
                self.hlx(prefix).set(hl);
                10
            }
            0xe2 => { //JP PO,nn
                let addr = self.fetch_u16(bus);
                if !flag8(self.f(), FLAG_PV) {
                    self.pc.set(addr);
                }
                10
            }
            0xe3 => { //EX (SP),HL
                let x = bus.peek_u16(self.sp);
                bus.poke_u16(self.sp, self.hlx(prefix).as_u16());
                self.hlx(prefix).set(x);
                19
            }
            0xe4 => { //CALL PO,nn
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
            0xe5 => { //PUSH HL
                let hl = *self.hlx(prefix);
                self.push(bus, hl);
                11
            }
            0xe6 => { //AND n
                let n = self.fetch(bus);
                let mut a = self.a();
                a = self.and_flags(a, n);
                self.set_a(a);
                7
            }
            0xe7 => { //RST 20
                let pc = self.pc;
                self.push(bus, pc);
                self.pc.set(0x20);
                11
            }
            0xe8 => { //RET PE
                if flag8(self.f(), FLAG_PV) {
                    let pc = self.pop(bus);
                    self.pc.set(pc);
                    11
                } else {
                    5
                }
            }
            0xe9 => { //JP (HL)
                self.pc = *self.hlx(prefix);
                4
            }
            0xea => { //JP PE,nn
                let addr = self.fetch_u16(bus);
                if flag8(self.f(), FLAG_PV) {
                    self.pc.set(addr);
                }
                10
            }
            0xeb => { //EX DE,HL
                swap(&mut self.de, &mut self.hl);
                4
            }
            0xec => { //CALL PE,nn
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
            0xee => { //XOR n
                let n = self.fetch(bus);
                let a = self.a();
                let a = self.xor_flags(a, n);
                self.set_a(a);
                7
            }
            0xef => { //RST 28
                let pc = self.pc;
                self.push(bus, pc);
                self.pc.set(0x28);
                11
            }
            0xf0 => { //RET P
                if !flag8(self.f(), FLAG_S) {
                    let pc = self.pop(bus);
                    self.pc.set(pc);
                    11
                } else {
                    5
                }
            }
            0xf1 => { //POP AF
                let af = self.pop(bus);
                self.af.set(af);
                10
            }
            0xf2 => { //JP P,nn
                let addr = self.fetch_u16(bus);
                if !flag8(self.f(), FLAG_S) {
                    self.pc.set(addr);
                }
                10
            }
            0xf3 => { //DI
                self.iff1 = false;
                4
            }
            0xf4 => { //CALL P,nn
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
            0xf5 => { //PUSH AF
                let af = self.af;
                self.push(bus, af);
                11
            }
            0xf6 => { //OR n
                let n = self.fetch(bus);
                let mut a = self.a();
                a = self.or_flags(a, n);
                self.set_a(a);
                7
            }
            0xf7 => { //RST 30
                let pc = self.pc;
                self.push(bus, pc);
                self.pc.set(0x30);
                11
            }
            0xf8 => { //RET M
                if flag8(self.f(), FLAG_S) {
                    let pc = self.pop(bus);
                    self.pc.set(pc);
                    11
                } else {
                    5
                }
            }
            0xf9 => { //LD SP,HL
                self.sp = *self.hlx(prefix);
                6
            }
            0xfa => { //JP M,nn
                let addr = self.fetch_u16(bus);
                if flag8(self.f(), FLAG_S) {
                    self.pc.set(addr);
                }
                10
            }
            0xfb => { //EI
                self.iff1 = true;
                4
            }
            0xfc => { //CALL M,nn
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
            0xfe => { //CP n
                let n = self.fetch(bus);
                let a = self.a();
                self.sub_flags(a, n, false);
                7
            }
            0xff => { //RST 38
                let pc = self.pc;
                self.push(bus, pc);
                self.pc.set(0x38);
                11
            }
            _ => {
                let rs = c & 0x07;
                let rd = (c >> 3) & 0x07;
                match c & 0b1100_0000 {
                    0x40 => { //LD r,r
                        if rs == 6 {
                            let (r, t) = self.reg_by_num(prefix, rs, bus);
                            self.set_reg_by_num_no_pre(rd, r);
                            7 + t
                        } else if rd == 6 {
                            let r = self.reg_by_num_no_pre(rs);
                            let t = self.set_reg_by_num(prefix, rd, bus, r);
                            7 + t
                        } else {
                            let r = self.reg_by_num_addr(prefix, rs, bus, 0);
                            self.set_reg_by_num_addr(prefix, rd, bus, r, 0);
                            4
                        }
                    }
                    _ => {
                        match c & 0b1111_1000 {
                            0x80 => { //ADD r
                                let a = self.a();
                                let (r, t) = self.reg_by_num(prefix, rs, bus);
                                let a = self.add_flags(a, r, false);
                                self.set_a(a);
                                4 + t
                            }
                            0x88 => { //ADC r
                                let a = self.a();
                                let (r, t) = self.reg_by_num(prefix, rs, bus);
                                let a = self.add_flags(a, r, true);
                                self.set_a(a);
                                4 + t
                            }
                            0x90 => { //SUB r
                                let a = self.a();
                                let (r, t) = self.reg_by_num(prefix, rs, bus);
                                let a = self.sub_flags(a, r, false);
                                self.set_a(a);
                                4 + t
                            }
                            0x98 => { //SBC r
                                let a = self.a();
                                let (r, t) = self.reg_by_num(prefix, rs, bus);
                                let a = self.sub_flags(a, r, true);
                                self.set_a(a);
                                4 + t
                            }
                            0xa0 => { //AND r
                                let a = self.a();
                                let (r, t) = self.reg_by_num(prefix, rs, bus);
                                let a = self.and_flags(a, r);
                                self.set_a(a);
                                4 + t
                            }
                            0xa8 => { //XOR r
                                let a = self.a();
                                let (r, t) = self.reg_by_num(prefix, rs, bus);
                                let a = self.xor_flags(a, r);
                                self.set_a(a);
                                4 + t
                            }
                            0xb0 => { //OR r
                                let a = self.a();
                                let (r, t) = self.reg_by_num(prefix, rs, bus);
                                let a = self.or_flags(a, r);
                                self.set_a(a);
                                4 + t
                            }
                            0xb8 => { //CP r
                                let a = self.a();
                                let (r, t) = self.reg_by_num(prefix, rs, bus);
                                self.sub_flags(a, r, false);
                                4 + t
                            }
                            _ => {
                                log!("unimplemented opcode {:02x} pc={:04x}", c, self.pc.as_u16());
                                0
                            }
                        }
                    }
                }
            }
        }
    }
    fn exec_cb(&mut self, prefix: XYPrefix, bus: &mut impl Bus) -> u32 {
        let (addr, t) = self.hlx_addr(prefix, bus);
        let c = self.fetch(bus);
        if prefix == XYPrefix::None {
            self.inc_r();
        }
        let r = c & 0x07;
        let n = (c >> 3) & 0x07;
        let write = match c & 0b1100_0000 {
            0x40 => { //BIT n,r
                let b = self.reg_by_num_addr(prefix, r, bus, addr);
                let r = b & (1 << n);
                let mut f = self.f();
                f = set_flag8(f, FLAG_H, true);
                f = set_flag8(f, FLAG_N, false);
                f = set_flag_szp(f, r);
                self.set_f(f);
                false
            }
            0x80 => { //RES n,r
                let mut b = self.reg_by_num_addr(prefix, r, bus, addr);
                b = set_flag8(b, 1 << n, false);
                self.set_reg_by_num_addr(prefix, r, bus, b, addr);
                true
            }
            0xc0 => { //SET n,r
                let mut b = self.reg_by_num_addr(prefix, r, bus, addr);
                b = set_flag8(b, 1 << n, true);
                self.set_reg_by_num_addr(prefix, r, bus, b, addr);
                true
            }
            _ => match c & 0b1111_1000 {
                0x00 => { //RLC r
                    let mut b = self.reg_by_num_addr(prefix, r, bus, addr);
                    let mut f = self.f();
                    let b7 = flag8(b, 0x80);
                    b = b.rotate_left(1);
                    f = set_flag8(f, FLAG_C, b7);
                    f = set_flag8(f, FLAG_N, false);
                    f = set_flag8(f, FLAG_H, false);
                    f = set_flag_szp(f, b);
                    self.set_reg_by_num_addr(prefix, r, bus, b, addr);
                    self.set_f(f);
                    true
                }
                0x08 => { //RRC r
                    let mut b = self.reg_by_num_addr(prefix, r, bus, addr);
                    let mut f = self.f();
                    let b0 = flag8(b, 0x01);
                    b = b.rotate_right(1);
                    f = set_flag8(f, FLAG_C, b0);
                    f = set_flag8(f, FLAG_N, false);
                    f = set_flag8(f, FLAG_H, false);
                    f = set_flag_szp(f, b);
                    self.set_reg_by_num_addr(prefix, r, bus, b, addr);
                    self.set_f(f);
                    true
                }
                0x10 => { //RL r
                    let mut b = self.reg_by_num_addr(prefix, r, bus, addr);
                    let mut f = self.f();
                    let b7 = flag8(b, 0x80);
                    let c = flag8(f, FLAG_C);
                    b <<= 1;
                    b = set_flag8(b, 1, c);
                    f = set_flag8(f, FLAG_C, b7);
                    f = set_flag8(f, FLAG_N, false);
                    f = set_flag8(f, FLAG_H, false);
                    f = set_flag_szp(f, b);
                    self.set_reg_by_num_addr(prefix, r, bus, b, addr);
                    self.set_f(f);
                    true
                }
                0x18 => { //RR r
                    let mut b = self.reg_by_num_addr(prefix, r, bus, addr);
                    let mut f = self.f();
                    let b0 = flag8(b, 0x01);
                    let c = flag8(f, FLAG_C);
                    b >>= 1;
                    b = set_flag8(b, 0x80, c);
                    f = set_flag8(f, FLAG_C, b0);
                    f = set_flag8(f, FLAG_N, false);
                    f = set_flag8(f, FLAG_H, false);
                    f = set_flag_szp(f, b);
                    self.set_reg_by_num_addr(prefix, r, bus, b, addr);
                    self.set_f(f);
                    true
                }
                0x20 => { //SLA r
                    let mut b = self.reg_by_num_addr(prefix, r, bus, addr);
                    let mut f = self.f();
                    let b7 = flag8(b, 0x80);
                    b <<= 1;
                    f = set_flag8(f, FLAG_C, b7);
                    f = set_flag8(f, FLAG_N, false);
                    f = set_flag8(f, FLAG_H, false);
                    f = set_flag_szp(f, b);
                    self.set_reg_by_num_addr(prefix, r, bus, b, addr);
                    self.set_f(f);
                    true
                }
                0x28 => { //SRA r
                    let mut b = self.reg_by_num_addr(prefix, r, bus, addr);
                    let mut f = self.f();
                    let b0 = flag8(b, 0x01);
                    b = ((b as i8) >> 1) as u8;
                    f = set_flag8(f, FLAG_C, b0);
                    f = set_flag8(f, FLAG_N, false);
                    f = set_flag8(f, FLAG_H, false);
                    f = set_flag_szp(f, b);
                    self.set_reg_by_num_addr(prefix, r, bus, b, addr);
                    self.set_f(f);
                    true
                }
                0x30 => { //SL1 r (undoc)
                    let mut b = self.reg_by_num_addr(prefix, r, bus, addr);
                    let mut f = self.f();
                    let b7 = flag8(b, 0x80);
                    b = (b << 1) | 1;
                    f = set_flag8(f, FLAG_C, b7);
                    f = set_flag8(f, FLAG_N, false);
                    f = set_flag8(f, FLAG_H, false);
                    f = set_flag_szp(f, b);
                    self.set_reg_by_num_addr(prefix, r, bus, b, addr);
                    self.set_f(f);
                    true
                }
                0x38 => { //SRL r
                    let mut b = self.reg_by_num_addr(prefix, r, bus, addr);
                    let mut f = self.f();
                    let b0 = flag8(b, 0x01);
                    b >>= 1;
                    f = set_flag8(f, FLAG_C, b0);
                    f = set_flag8(f, FLAG_N, false);
                    f = set_flag8(f, FLAG_H, false);
                    f = set_flag_szp(f, b);
                    self.set_reg_by_num_addr(prefix, r, bus, b, addr);
                    self.set_f(f);
                    true
                }
                _ => {
                    log!("unimplemented opcode CB {:02x} pc={:04x}", c, self.pc.as_u16());
                    false
                }
            }
        };
        t + if r == 6 { 15 + if write { 3 } else { 0 } } else  { 8 }
    }
    fn exec_ed(&mut self, prefix: XYPrefix, bus: &mut impl Bus) -> u32 {
        let c = self.fetch(bus);
        if prefix == XYPrefix::None {
            self.inc_r();
        }
        match c {
            0x40 => { //IN B,(C)
                let bc = self.bc.as_u16();
                let mut f = self.f();
                let b = bus.do_in(bc);
                f = set_flag8(f, FLAG_N, false);
                f = set_flag8(f, FLAG_H, false);
                f = set_flag_szp(f, b);
                self.bc.set_hi(b);
                self.set_f(f);
                12
            }
            0x41 => { //OUT (C),B
                let bc = self.bc.as_u16();
                bus.do_out(bc, self.bc.hi());
                12
            }
            0x42 => { //SBC HL,BC
                let mut hl = self.hl.as_u16();
                let bc = self.bc.as_u16();
                hl = self.sbc16_flags(hl, bc);
                self.hl.set(hl);
                15
            }
            0x43 => { //LD (nn),BC
                let addr = self.fetch_u16(bus);
                bus.poke_u16(addr, self.bc.as_u16());
                20
            }
            0x44 => { //NEG
                let a = self.a();
                let a = self.sub_flags(0, a, false);
                self.set_a(a);
                8
            }
            0x45 => { //RETN
                let pc = self.pop(bus);
                self.pc.set(pc);
                14
            }
            0x46 => { // /IM 0
                self.im = InterruptMode::IM0;
                8
            }
            0x47 => { //LD I,A
                self.i = self.a();
                9
            }
            0x48 => { //IN C,(C)
                let bc = self.bc.as_u16();
                let mut f = self.f();
                let b = bus.do_in(bc);
                f = set_flag8(f, FLAG_N, false);
                f = set_flag8(f, FLAG_H, false);
                f = set_flag_szp(f, b);
                self.bc.set_lo(b);
                self.set_f(f);
                12
            }
            0x49 => { //OUT (C),C
                let bc = self.bc.as_u16();
                bus.do_out(bc, self.bc.lo());
                12
            }
            0x4a => { //ADC HL,BC
                let mut hl = self.hl.as_u16();
                let bc = self.bc.as_u16();
                hl = self.adc16_flags(hl, bc);
                self.hl.set(hl);
                15
            }
            0x4b => { //LD BC,(nn)
                let addr = self.fetch_u16(bus);
                self.bc.set(bus.peek_u16(addr));
                20
            }
            0x4d => { //RETI
                let pc = self.pop(bus);
                self.pc.set(pc);
                14
            }
            0x4f => { //LD R,A
                let a = self.a();
                self.set_r(a);
                9
            }
            0x50 => { //IN D,(C)
                let bc = self.bc.as_u16();
                let mut f = self.f();
                let b = bus.do_in(bc);
                f = set_flag8(f, FLAG_N, false);
                f = set_flag8(f, FLAG_H, false);
                f = set_flag_szp(f, b);
                self.de.set_hi(b);
                self.set_f(f);
                12
            }
            0x51 => { //OUT (C),D
                let bc = self.bc.as_u16();
                bus.do_out(bc, self.de.hi());
                12
            }
            0x52 => { //SBC HL,DE
                let mut hl = self.hl.as_u16();
                let de = self.de.as_u16();
                hl = self.sbc16_flags(hl, de);
                self.hl.set(hl);
                15
            }
            0x53 => { //LD (nn),DE
                let addr = self.fetch_u16(bus);
                bus.poke_u16(addr, self.de.as_u16());
                20
            }
            0x56 => { //IM 1
                self.im = InterruptMode::IM1;
                8
            }
            0x57 => { //LD A,I
                let i = self.i;
                let mut f = self.f();
                f = set_flag8(f, FLAG_H, false);
                f = set_flag8(f, FLAG_N, false);
                f = set_flag8(f, FLAG_PV, self.iff1);
                f = set_flag_sz(f, i);
                self.set_a(i);
                self.set_f(f);
                9
            }
            0x58 => { //IN E,(C)
                let bc = self.bc.as_u16();
                let mut f = self.f();
                let b = bus.do_in(bc);
                f = set_flag8(f, FLAG_N, false);
                f = set_flag8(f, FLAG_H, false);
                f = set_flag_szp(f, b);
                self.de.set_lo(b);
                self.set_f(f);
                12
            }
            0x59 => { //OUT (C),E
                let bc = self.bc.as_u16();
                bus.do_out(bc, self.de.lo());
                12
            }
            0x5a => { //ADC HL,DE
                let mut hl = self.hl.as_u16();
                let de = self.de.as_u16();
                hl = self.adc16_flags(hl, de);
                self.hl.set(hl);
                15
            }
            0x5b => { //LD DE,(nn)
                let addr = self.fetch_u16(bus);
                self.de.set(bus.peek_u16(addr));
                20
            }
            0x5e => { //IM 2
                self.im = InterruptMode::IM2;
                8
            }
            0x5f => { //LD A,R
                let r = self.r();
                let mut f = self.f();
                f = set_flag8(f, FLAG_H, false);
                f = set_flag8(f, FLAG_N, false);
                f = set_flag8(f, FLAG_PV, self.iff1);
                f = set_flag_sz(f, r);
                self.set_a(r);
                self.set_f(f);
                9
            }
            0x60 => { //IN H,(C)
                let bc = self.bc.as_u16();
                let mut f = self.f();
                let b = bus.do_in(bc);
                f = set_flag8(f, FLAG_N, false);
                f = set_flag8(f, FLAG_H, false);
                f = set_flag_szp(f, b);
                self.hl.set_hi(b);
                self.set_f(f);
                12
            }
            0x61 => { //OUT (C),H
                let bc = self.bc.as_u16();
                bus.do_out(bc, self.hl.hi());
                12
            }
            0x62 => { //SBC HL,HL
                let mut hl = self.hl.as_u16();
                hl = self.sbc16_flags(hl, hl);
                self.hl.set(hl);
                15
            }
            0x63 => { //LD (nn),HL
                let addr = self.fetch_u16(bus);
                bus.poke_u16(addr, self.hl.as_u16());
                20
            }
            0x67 => { //RRD
                let x = bus.peek(self.hl);
                let a = self.a();
                let mut f = self.f();
                let new_a = (a & 0xf0) | (x & 0x0f);
                let new_x = ((a & 0x0f) << 4) | ((x & 0xf0) >> 4);
                f = set_flag8(f, FLAG_H, false);
                f = set_flag8(f, FLAG_N, false);
                f = set_flag_szp(f, new_a);
                self.set_a(new_a);
                self.set_f(f);
                bus.poke(self.hl, new_x);
                18
            }
            0x68 => { //IN L,(C)
                let bc = self.bc.as_u16();
                let mut f = self.f();
                let b = bus.do_in(bc);
                f = set_flag8(f, FLAG_N, false);
                f = set_flag8(f, FLAG_H, false);
                f = set_flag_szp(f, b);
                self.hl.set_lo(b);
                self.set_f(f);
                12
            }
            0x69 => { //OUT (C),L
                let bc = self.bc.as_u16();
                bus.do_out(bc, self.hl.lo());
                12
            }
            0x6a => { //ADC HL,HL
                let mut hl = self.hl.as_u16();
                hl = self.adc16_flags(hl, hl);
                self.hl.set(hl);
                15
            }
            0x6b => { //LD HL,(nn)
                let addr = self.fetch_u16(bus);
                self.hl.set(bus.peek_u16(addr));
                20
            }
            0x6f => { //RLD
                let x = bus.peek(self.hl);
                let a = self.a();
                let mut f = self.f();
                let new_a = (a & 0xf0) | ((x & 0xf0) >> 4);
                let new_x = (a & 0x0f) | ((x & 0x0f) << 4);
                f = set_flag8(f, FLAG_H, false);
                f = set_flag8(f, FLAG_N, false);
                f = set_flag_szp(f, new_a);
                self.set_a(new_a);
                self.set_f(f);
                bus.poke(self.hl, new_x);
                18
            }
            0x70 => { //IN F,(C)
                let bc = self.bc.as_u16();
                let b = bus.do_in(bc);
                self.set_f(b);
                12
            }
            0x71 => { //OUT (C),F
                let bc = self.bc.as_u16();
                bus.do_out(bc, self.f());
                12
            }
            0x72 => { //SBC HL,SP
                let mut hl = self.hl.as_u16();
                let mut sp = self.sp.as_u16();
                hl = self.sbc16_flags(hl, sp);
                self.hl.set(hl);
                15
            }
            0x73 => { //LD (nn),SP
                let addr = self.fetch_u16(bus);
                bus.poke_u16(addr, self.sp.as_u16());
                20
            }
            0x78 => { //IN A,(C)
                let bc = self.bc.as_u16();
                let mut f = self.f();
                let b = bus.do_in(bc);
                f = set_flag8(f, FLAG_N, false);
                f = set_flag8(f, FLAG_H, false);
                f = set_flag_szp(f, b);
                self.set_a(b);
                self.set_f(f);
                12
            }
            0x79 => { //OUT (C),A
                let bc = self.bc.as_u16();
                bus.do_out(bc, self.a());
                12
            }
            0x7a => { //ADC HL,SP
                let mut hl = self.hl.as_u16();
                let sp = self.sp.as_u16();
                hl = self.adc16_flags(hl, sp);
                self.hl.set(hl);
                15
            }
            0x7b => { //LD SP,(nn)
                let addr = self.fetch_u16(bus);
                self.sp.set(bus.peek_u16(addr));
                20
            }
            0xa0 => { //LDI
                self.ldi_ldd(Direction::Inc, bus);
                16
            }
            0xa1 => { //CPI
                self.cpi_cpd(Direction::Inc, bus);
                16
            }
            0xa2 => { //INI
                self.ini_ind(Direction::Inc, bus);
                16
            }
            0xa3 => { //OUTI
                self.outi_outd(Direction::Inc, bus);
                16
            }
            0xa8 => { //LDD
                self.ldi_ldd(Direction::Dec, bus);
                16
            }
            0xa9 => { //CPD
                self.cpi_cpd(Direction::Dec, bus);
                16
            }
            0xaa => { //IND
                self.ini_ind(Direction::Dec, bus);
                16
            }
            0xab => { //OUTD
                self.outi_outd(Direction::Dec, bus);
                16
            }
            0xb0 => { //LDIR
                self.ldi_ldd(Direction::Inc, bus);
                if self.bc.as_u16() != 0 {
                    self.pc -= 2;
                    21
                } else {
                    16
                }
            }
            0xb1 => { //CPIR
                let r = self.cpi_cpd(Direction::Inc, bus);
                if self.bc.as_u16() != 0 && r != 0 {
                    self.pc -= 2;
                    21
                } else {
                    16
                }
            }
            0xb2 => { //INIR
                let b = self.ini_ind(Direction::Inc, bus);
                if b != 0 {
                    self.pc -= 2;
                    21
                } else {
                    16
                }
            }
            0xb3 => { //OTIR
                let b = self.outi_outd(Direction::Inc, bus);
                if b != 0 {
                    self.pc -= 2;
                    21
                } else {
                    16
                }
            }
            0xb8 => { //LDDR
                self.ldi_ldd(Direction::Dec, bus);
                if self.bc.as_u16() != 0 {
                    self.pc -= 2;
                    21
                } else {
                    16
                }
            }
            0xb9 => { //CPDR
                let r = self.cpi_cpd(Direction::Dec, bus);
                if self.bc.as_u16() != 0 && r != 0 {
                    self.pc -= 2;
                    21
                } else {
                    16
                }
            }
            0xba => { //INDR
                let b = self.ini_ind(Direction::Dec, bus);
                if b != 0 {
                    self.pc -= 2;
                    21
                } else {
                    16
                }
            }
            0xbb => { //OTDR
                let b = self.outi_outd(Direction::Dec, bus);
                if b != 0 {
                    self.pc -= 2;
                    21
                } else {
                    16
                }
            }
            _ => {
                log!("unimplemented opcode ED {:02x} pc={:04x}", c, self.pc.as_u16());
                0
            },
        }
    }
}

#[cfg(feature="dump_ops")]
impl Z80 {
    pub fn dump_add(&mut self) {
        for a in 0..=0xff {
            for r in 0..=0xff {
                self.set_f(0);
                let a2 = self.add_flags(a, r, false);
                println!("{:02x} {:02x} {:02x} {:02x}", a, r, a2, self.f() & 0xd7);
            }
        }
    }
    pub fn dump_adc(&mut self) {
        for a in 0..=0xff {
            for r in 0..=0xff {
                for f in 0..=0xff {
                    self.set_f(f);
                    let a2 = self.add_flags(a, r, true);
                    println!("{:02x} {:02x} {:02x} {:02x} {:02x}", a, f, r, a2, self.f() & 0xd7);
                }
            }
        }
    }
    pub fn dump_sub(&mut self) {
        for a in 0..=0xff {
            for r in 0..=0xff {
                self.set_f(0);
                let a2 = self.sub_flags(a, r, false);
                println!("{:02x} {:02x} {:02x} {:02x}", a, r, a2, self.f() & 0xd7);
            }
        }
    }
    pub fn dump_sbc(&mut self) {
        for a in 0..=0xff {
            for r in 0..=0xff {
                for f in 0..=0xff {
                    self.set_f(f);
                    let a2 = self.sub_flags(a, r, true);
                    println!("{:02x} {:02x} {:02x} {:02x} {:02x}", a, f, r, a2, self.f() & 0xd7);
                }
            }
        }
    }
    pub fn dump_daa(&mut self) {
        for f in 0..=0xff {
            for a in 0..=0xff {
                self.set_a(a);
                self.set_f(f);
                self.daa();
                println!("{:02x} {:02x} {:02x} {:02x}", a, f, self.a(), self.f() & 0xd7);
            }
        }
    }
}

