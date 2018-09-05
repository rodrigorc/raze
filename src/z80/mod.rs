use std::mem::swap;
use std::io::{self, Read, Write};

mod r16;

use memory::Memory;
use self::r16::R16;

pub trait InOut {
    fn do_in(&mut self, port: u16) -> u8;
    fn do_out(&mut self, port: u16, value: u8);
}

const FLAG_S  : u8 = 0b1000_0000;
const FLAG_Z  : u8 = 0b0100_0000;
const FLAG_F5 : u8 = 0b0010_0000;
const FLAG_H  : u8 = 0b0001_0000;
const FLAG_F3 : u8 = 0b0000_1000;
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
fn parity(mut b: u8) -> bool {
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
    prefix: XYPrefix,
    next_op: NextOp,
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
            prefix: XYPrefix::None,
            next_op: NextOp::Fetch,
        }
    }
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
    pub fn interrupt(&mut self, mem: &mut Memory) {
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
    fn fetch(&mut self, mem: &Memory) -> u8 {
        let c = mem.peek(self.pc);
        self.pc += 1;
        c
    }
    fn fetch_u16(&mut self, mem: &mut Memory) -> u16 {
        let l = mem.peek(self.pc) as u16;
        self.pc += 1;
        let h = mem.peek(self.pc) as u16;
        self.pc += 1;
        h << 8 | l
    }
    fn push(&mut self, mem: &mut Memory, x: impl Into<u16>) {
        let x = x.into();
        self.sp -= 1;
        mem.poke(self.sp, (x >> 8) as u8);
        self.sp -= 1;
        mem.poke(self.sp, x as u8);
    }
    fn pop(&mut self, mem: &mut Memory) -> u16 {
        let x = mem.peek_u16(self.sp);
        self.sp += 2;
        x
    }
    fn reg_by_num(&mut self, r: u8, mem: &Memory, addr: u16) -> u8 {
        match r {
            0 => self.bc.hi(),
            1 => self.bc.lo(),
            2 => self.de.hi(),
            3 => self.de.lo(),
            4 => self.hlx().hi(),
            5 => self.hlx().lo(),
            6 => mem.peek(addr),
            7 => self.a(),
            _ => panic!("unknown reg_by_num {}", r),
        }
    }
    fn set_reg_by_num(&mut self, r: u8, mem: &mut Memory, b: u8, addr: u16) {
        match r {
            0 => self.bc.set_hi(b),
            1 => self.bc.set_lo(b),
            2 => self.de.set_hi(b),
            3 => self.de.set_lo(b),
            4 => self.hlx().set_hi(b),
            5 => self.hlx().set_lo(b),
            6 => mem.poke(addr, b),
            7 => self.set_a(b),
            _ => panic!("unknown reg_by_num {}", r),
        }
    }
    fn reg_by_num_no_pre(&mut self, r: u8, mem: &Memory) -> u8 {
        match r {
            0 => self.bc.hi(),
            1 => self.bc.lo(),
            2 => self.de.hi(),
            3 => self.de.lo(),
            4 => self.hl.hi(),
            5 => self.hl.lo(),
            7 => self.a(),
            _ => panic!("unknown reg_by_num {}", r),
        }
    }
    fn set_reg_by_num_no_pre(&mut self, r: u8, mem: &mut Memory, b: u8) {
        match r {
            0 => self.bc.set_hi(b),
            1 => self.bc.set_lo(b),
            2 => self.de.set_hi(b),
            3 => self.de.set_lo(b),
            4 => self.hl.set_hi(b),
            5 => self.hl.set_lo(b),
            7 => self.set_a(b),
            _ => panic!("unknown reg_by_num {}", r),
        }
    }
    fn hlx(&mut self) -> &mut R16 {
        match self.prefix {
            XYPrefix::None => &mut self.hl,
            XYPrefix::IX => &mut self.ix,
            XYPrefix::IY => &mut self.iy,
        }
    }
    fn hlx_addr(&mut self, mem: &Memory) -> u16 {
        match self.prefix {
            XYPrefix::None => self.hl.as_u16(),
            XYPrefix::IX => {
                let d = self.fetch(mem);
                self.ix.as_u16().wrapping_add(d as i8 as i16 as u16)
            }
            XYPrefix::IY => {
                let d = self.fetch(mem);
                self.iy.as_u16().wrapping_add(d as i8 as i16 as u16)
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
        f = set_flag8(f, FLAG_PV,
                 flag8(a, 0x80) != flag8(b, 0x80) && flag8(a, 0x80) != flag8(r, 0x80));
        f = set_flag8(f, FLAG_Z, r == 0);
        f = set_flag8(f, FLAG_S, flag8(r, 0x80));
        f = set_flag8(f, FLAG_H, half_carry8(r, b, a));
        self.set_f(f);
        r
    }
    fn sub16_flags(&mut self, a: u16, b: u16) -> u16 {
        let r = a.wrapping_sub(b);
        let mut f = self.f();
        f = set_flag8(f, FLAG_N, true);
        f = set_flag8(f, FLAG_C, carry16(r, b, a));
        f = set_flag8(f, FLAG_PV,
                 flag16(a, 0x8000) != flag16(b, 0x8000) && flag16(a, 0x8000) != flag16(r, 0x8000));
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
        f = set_flag8(f, FLAG_PV,
                 flag8(a, 0x80) == flag8(b, 0x80) && flag8(a, 0x80) != flag8(r, 0x80));
        f = set_flag8(f, FLAG_Z, r == 0);
        f = set_flag8(f, FLAG_S, flag8(r, 0x80));
        f = set_flag8(f, FLAG_H, half_carry8(a, b, r));
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
        f = set_flag8(f, FLAG_PV, r == 0x80);
        f = set_flag8(f, FLAG_Z,
                 r == 0);
        f = set_flag8(f, FLAG_S, flag8(r, 0x80));
        f = set_flag8(f, FLAG_H, (r & 0x0f) == 0x00);
        self.set_f(f);
        r
    }
    fn dec_flags(&mut self, a: u8) -> u8 {
        let r = a.wrapping_sub(1);
        let mut f = self.f();
        f = set_flag8(f, FLAG_N, true);
        f = set_flag8(f, FLAG_PV, r == 0x7f);
        f = set_flag8(f, FLAG_Z, r == 0);
        f = set_flag8(f, FLAG_S, flag8(r, 0x80));
        f = set_flag8(f, FLAG_H, (r & 0x0f) == 0x0f);
        self.set_f(f);
        r
    }
    fn and_flags(&mut self, a: u8, b: u8) -> u8 {
        let r = a & b;
        let mut f = self.f();
        f = set_flag8(f, FLAG_C, false);
        f = set_flag8(f, FLAG_N, false);
        f = set_flag8(f, FLAG_PV, parity(r));
        f = set_flag8(f, FLAG_Z, r == 0);
        f = set_flag8(f, FLAG_S, flag8(r, 0x80));
        f = set_flag8(f, FLAG_H, true);
        self.set_f(f);
        r
    }
    fn or_flags(&mut self, a: u8, b: u8) -> u8 {
        let r = a | b;
        let mut f = self.f();
        f = set_flag8(f, FLAG_C, false);
        f = set_flag8(f, FLAG_N, false);
        f = set_flag8(f, FLAG_PV, parity(r));
        f = set_flag8(f, FLAG_Z, r == 0);
        f = set_flag8(f, FLAG_S, flag8(r, 0x80));
        f = set_flag8(f, FLAG_H, false);
        self.set_f(f);
        r
    }
    fn xor_flags(&mut self, a: u8, b: u8) -> u8 {
        let r = a ^ b;
        let mut f = self.f();
        f = set_flag8(f, FLAG_C, false);
        f = set_flag8(f, FLAG_N, false);
        f = set_flag8(f, FLAG_PV, parity(r));
        f = set_flag8(f, FLAG_Z, r == 0);
        f = set_flag8(f, FLAG_S, flag8(r, 0x80));
        f = set_flag8(f, FLAG_H, false);
        self.set_f(f);
        r
    }
    fn ldi(&mut self, mem: &mut Memory) {
        let hl = self.hl;
        let de = self.de;
        let x = mem.peek(hl);
        mem.poke(de, x);

        self.hl += 1;
        self.de += 1;
        self.bc -= 1;

        let mut f = self.f();
        f = set_flag8(f, FLAG_N, false);
        f = set_flag8(f, FLAG_H, false);
        f = set_flag8(f, FLAG_PV, self.bc.as_u16() != 0);
        self.set_f(f);
    }
    fn ldd(&mut self, mem: &mut Memory) {
        let hl = self.hl;
        let de = self.de;
        let x = mem.peek(hl);
        mem.poke(de, x);

        self.hl -= 1;
        self.de -= 1;
        self.bc -= 1;

        let mut f = self.f();
        f = set_flag8(f, FLAG_N, false);
        f = set_flag8(f, FLAG_H, false);
        f = set_flag8(f, FLAG_PV, self.bc.as_u16() != 0);
        self.set_f(f);

    }
    fn cpi(&mut self, mem: &mut Memory) -> u8 {
        let hl = self.hl;
        let x = mem.peek(hl);
        let a = self.a();
        let mut f = self.f();

        self.hl += 1;
        self.bc -= 1;

        let r = a.wrapping_sub(x);
        f = set_flag8(f, FLAG_Z, r == 0);
        f = set_flag8(f, FLAG_S, flag8(r, 0x80));
        f = set_flag8(f, FLAG_H, half_carry8(r, x, a));
        f = set_flag8(f, FLAG_N, true);
        f = set_flag8(f, FLAG_PV, self.bc.as_u16() != 0);
        self.set_f(f);
        r
    }
    fn cpd(&mut self, mem: &mut Memory) -> u8{
        let hl = self.hl;
        let x = mem.peek(hl);
        let a = self.a();
        let mut f = self.f();

        self.hl -= 1;
        self.bc -= 1;

        let r = a.wrapping_sub(x);
        f = set_flag8(f, FLAG_Z, r == 0);
        f = set_flag8(f, FLAG_S, flag8(r, 0x80));
        f = set_flag8(f, FLAG_H, half_carry8(r, x, a));
        f = set_flag8(f, FLAG_N, true);
        f = set_flag8(f, FLAG_PV, self.bc.as_u16() != 0);
        self.set_f(f);
        r
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
        f = set_flag8(f, FLAG_PV, parity(a));
        f = set_flag8(f, FLAG_Z, a == 0);
        f = set_flag8(f, FLAG_S, flag8(a, 0x80));
        self.set_a(a);
        self.set_f(f);
    }
    pub fn exec(&mut self, mem: &mut Memory, io: &mut dyn InOut) -> u32 {
        let c = match self.next_op {
            NextOp::Fetch => {
                self.inc_r();
                self.fetch(mem)
            }
            NextOp::Halt => 0x00, //NOP
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
                        let v = (self.i as u16) << 8; //assume 0x00 in the data bus
                        let v = mem.peek_u16(v);
                        let pc = self.pc;
                        self.push(mem, pc);
                        self.pc.set(v);
                        return 0;
                    }
                }
            }
        };
        let c = match c {
            0xdd => {
                self.prefix = XYPrefix::IX;
                self.inc_r();
                self.fetch(mem)
            }
            0xfd => {
                self.prefix = XYPrefix::IY;
                self.inc_r();
                self.fetch(mem)
            }
            _ => {
                self.prefix = XYPrefix::None;
                c
            }
        };
        match c {
            0xcb => { self.exec_cb(mem, io); }
            0xed => { self.exec_ed(mem, io); }

            0x00 => { //NOP
            }
            0x01 => { //LD BC,nn
                let d = self.fetch_u16(mem);
                self.bc.set(d);
            }
            0x02 => { //LD (BC),A
                let a = self.a();
                mem.poke(self.bc, a);
            }
            0x03 => { //INC BC
                self.bc += 1;
            }
            0x04 => { //INC B
                let mut r = self.bc.hi();
                r = self.inc_flags(r);
                self.bc.set_hi(r);
            }
            0x05 => { //DEC B
                let mut r = self.bc.hi();
                r = self.dec_flags(r);
                self.bc.set_hi(r);
            }
            0x06 => { //LD B,n
                let n = self.fetch(mem);
                self.bc.set_hi(n);
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
            }
            0x08 => { //EX AF,AF
                swap(&mut self.af, &mut self.af_);
            }
            0x09 => { //ADD HL,BC
                let mut hl = self.hlx().as_u16();
                let bc = self.bc.as_u16();
                hl = self.add16_flags(hl, bc);
                self.hlx().set(hl);
            }
            0x0a => { //LD A,(BC)
                let a = mem.peek(self.bc);
                self.set_a(a);
            }
            0x0b => { //DEC BC
                self.bc -= 1;
            }
            0x0c => { //INC C
                let mut r = self.bc.lo();
                r = self.inc_flags(r);
                self.bc.set_lo(r);
            }
            0x0d => { //DEC C
                let mut r = self.bc.lo();
                r = self.dec_flags(r);
                self.bc.set_lo(r);
            }
            0x0e => { //LD C,n
                let n = self.fetch(mem);
                self.bc.set_lo(n);
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
            }
            0x10 => { //DJNZ d
                let d = self.fetch(mem);
                let mut b = self.bc.hi();
                b = b.wrapping_sub(1);
                self.bc.set_hi(b);
                if b != 0 {
                    self.pc += d as i8 as i16 as u16;
                }
            }
            0x11 => { //LD DE,nn
                let nn = self.fetch_u16(mem);
                self.de.set(nn);
            }
            0x12 => { //LD (DE),A
                let a = self.a();
                mem.poke(self.de, a);
            }
            0x13 => { //INC DE
                self.de += 1;
            }
            0x14 => { //INC D
                let mut r = self.de.hi();
                r = self.inc_flags(r);
                self.de.set_hi(r);
            }
            0x15 => { //DEC D
                let mut r = self.de.hi();
                r = self.dec_flags(r);
                self.de.set_hi(r);
            }
            0x16 => { //LD D,n
                let n = self.fetch(mem);
                self.de.set_hi(n);
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
            }
            0x18 => { //JR d
                let d = self.fetch(mem);
                self.pc += d as i8 as i16 as u16;
            }
            0x19 => { //ADD HL,DE
                let mut hl = self.hlx().as_u16();
                let de = self.de.as_u16();
                hl = self.add16_flags(hl, de);
                self.hlx().set(hl);
            }
            0x1a => { //LD A,(DE)
                let a = mem.peek(self.de);
                self.set_a(a);
            }
            0x1b => { //DEC DE
                self.de -= 1;
            }
            0x1c => { //INC E
                let mut r = self.de.lo();
                r = self.inc_flags(r);
                self.de.set_lo(r);
            }
            0x1d => { //DEC E
                let mut r = self.de.lo();
                r = self.dec_flags(r);
                self.de.set_lo(r);
            }
            0x1e => { //LD E,n
                let n = self.fetch(mem);
                self.de.set_lo(n);
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
            }
            0x20 => { //JR NZ,d
                let d = self.fetch(mem);
                if !flag8(self.f(), FLAG_Z) {
                    self.pc += d as i8 as i16 as u16;
                }
            }
            0x21 => { //LD HL,nn
                let d = self.fetch_u16(mem);
                self.hlx().set(d);
            }
            0x22 => { //LD (nn),HL
                let addr = self.fetch_u16(mem);
                mem.poke_u16(addr, self.hlx().as_u16());
            }
            0x23 => { //INC HL
                *self.hlx() += 1;
            }
            0x24 => { //INC H
                let mut r = self.hlx().hi();
                r = self.inc_flags(r);
                self.hlx().set_hi(r);
            }
            0x25 => { //DEC H
                let mut r = self.hlx().hi();
                r = self.dec_flags(r);
                self.hlx().set_hi(r);
            }
            0x26 => { //LD H,n
                let n = self.fetch(mem);
                self.hlx().set_hi(n);
            }
            0x27 => { //DAA
                self.daa();
            }
            0x28 => { //JR Z,d
                let d = self.fetch(mem);
                if flag8(self.f(), FLAG_Z) {
                    self.pc += d as i8 as i16 as u16;
                }
            }
            0x29 => { //ADD HL,HL
                let mut hl = self.hlx().as_u16();
                hl = self.add16_flags(hl, hl);
                self.hlx().set(hl);
            }
            0x2a => { //LD HL,(nn)
                let addr = self.fetch_u16(mem);
                let d = mem.peek_u16(addr);
                self.hlx().set(d);
            }
            0x2b => { //DEC HL
                *self.hlx() -= 1;
            }
            0x2c => { //INC L
                let mut r = self.hlx().lo();
                r = self.inc_flags(r);
                self.hlx().set_lo(r);
            }
            0x2d => { //DEC L
                let mut r = self.hlx().lo();
                r = self.dec_flags(r);
                self.hlx().set_lo(r);
            }
            0x2e => { //LD L,n
                let n = self.fetch(mem);
                self.hlx().set_lo(n);
            }
            0x2f => { //CPL
                let mut a = self.a();
                let mut f = self.f();
                a ^= 0xff;
                f = set_flag8(f, FLAG_H, true);
                f = set_flag8(f, FLAG_N, true);
                self.set_a(a);
                self.set_f(f);
            }
            0x30 => { //JR NC,d
                let d = self.fetch(mem);
                if !flag8(self.f(), FLAG_C) {
                    self.pc += d as i8 as i16 as u16;
                }
            }
            0x31 => { //LD SP,nn
                let nn = self.fetch_u16(mem);
                self.sp.set(nn);
            }
            0x32 => { //LD (nn),A
                let addr = self.fetch_u16(mem);
                mem.poke(addr, self.a());
            }
            0x33 => { //INC SP
                self.sp += 1;
            }
            0x34 => { //INC (HL)
                let addr = self.hlx_addr(mem);
                let mut b = mem.peek(addr);
                b = self.inc_flags(b);
                mem.poke(addr, b);
            }
            0x35 => { //DEC (HL)
                let addr = self.hlx_addr(mem);
                let mut b = mem.peek(addr);
                b = self.dec_flags(b);
                mem.poke(addr, b);
            }
            0x36 => { //LD (HL),n
                let addr = self.hlx_addr(mem);
                let n = self.fetch(mem);
                mem.poke(addr, n);
            }
            0x37 => { //SCF
                let mut f = self.f();
                f = set_flag8(f, FLAG_N, false);
                f = set_flag8(f, FLAG_H, false);
                f = set_flag8(f, FLAG_C, true);
                self.set_f(f);
            }
            0x38 => { //JR C,d
                let d = self.fetch(mem);
                if flag8(self.f(), FLAG_C) {
                    self.pc += d as i8 as i16 as u16;
                }
            }
            0x39 => { //ADD HL,SP
                let mut hl = self.hlx().as_u16();
                let sp = self.sp.as_u16();
                hl = self.add16_flags(hl, sp);
                self.hlx().set(hl);
            }
            0x3a => { //LD A,(nn)
                let addr = self.fetch_u16(mem);
                let x = mem.peek(addr);
                self.set_a(x);
            }
            0x3b => { //DEC SP
                self.sp -= 1;
            }
            0x3c => { //INC A
                let mut r = self.a();
                r = self.inc_flags(r);
                self.set_a(r);
            }
            0x3d => { //DEC A
                let mut r = self.a();
                r = self.dec_flags(r);
                self.set_a(r);
            }
            0x3e => { //LD A,n
                let n = self.fetch(mem);
                self.set_a(n);
            }
            0x3f => { //CCF
                let mut f = self.f();
                let c = flag8(f, FLAG_C);
                f = set_flag8(f, FLAG_N, false);
                f = set_flag8(f, FLAG_H, c);
                f = set_flag8(f, FLAG_C, !c);
                self.set_f(f);
            }
            0x76 => { //HALT
                log!("HALT");
                if !self.iff1 {
                    log!("DI/HALT deadlock!");
                }
                self.next_op = NextOp::Halt;
            }
            0xc0 => { //RET NZ 
                if !flag8(self.f(), FLAG_Z) {
                    let pc = self.pop(mem);
                    self.pc.set(pc);
                }
            }
            0xc1 => { //POP BC
                let bc = self.pop(mem);
                self.bc.set(bc);
            }
            0xc2 => { //JP NZ,nn
                let addr = self.fetch_u16(mem);
                if !flag8(self.f(), FLAG_Z) {
                    self.pc.set(addr);
                }
            }
            0xc3 => { //JP nn
                let pc = self.fetch_u16(mem);
                self.pc.set(pc);
            }
            0xc4 => { //CALL NZ,nn
                let addr = self.fetch_u16(mem);
                if !flag8(self.f(), FLAG_Z) {
                    let pc = self.pc;
                    self.push(mem, pc);
                    self.pc.set(addr);
                }
            }
            0xc5 => { //PUSH BC
                let bc = self.bc;
                self.push(mem, bc);
            }
            0xc6 => { //ADD n
                let n = self.fetch(mem);
                let a = self.a();
                let a = self.add_flags(a, n, false);
                self.set_a(a);
            }
            0xc7 => { //RST 00
                let pc = self.pc;
                self.push(mem, pc);
                self.pc.set(0x00);
            }
            0xc8 => { //RET Z 
                if flag8(self.f(), FLAG_Z) {
                    let pc = self.pop(mem);
                    self.pc.set(pc);
                }
            }
            0xc9 => { //RET
                let pc = self.pop(mem);
                self.pc.set(pc);
            }
            0xca => { //JP Z,nn
                let addr = self.fetch_u16(mem);
                if flag8(self.f(), FLAG_Z) {
                    self.pc.set(addr);
                }
            }
            0xcc => { //CALL Z,nn
                let addr = self.fetch_u16(mem);
                if flag8(self.f(), FLAG_Z) {
                    let pc = self.pc;
                    self.push(mem, pc);
                    self.pc.set(addr);
                }
            }
            0xcd => { //CALL nn
                let addr = self.fetch_u16(mem);
                let pc = self.pc;
                self.push(mem, pc);
                self.pc.set(addr);
            }
            0xce => { //ADC n
                let n = self.fetch(mem);
                let a = self.a();
                let a = self.add_flags(a, n, true);
                self.set_a(a);
            }
            0xcf => { //RST 08
                let pc = self.pc;
                self.push(mem, pc);
                self.pc.set(0x08);
            }
            0xd0 => { //RET NC 
                if !flag8(self.f(), FLAG_C) {
                    let pc = self.pop(mem);
                    self.pc.set(pc);
                }
            }
            0xd1 => { //POP DE
                let de = self.pop(mem);
                self.de.set(de);
            }
            0xd2 => { //JP NC,nn
                let addr = self.fetch_u16(mem);
                if !flag8(self.f(), FLAG_C) {
                    self.pc.set(addr);
                }
            }
            0xd3 => { //OUT (n),A
                let n = self.fetch(mem);
                let a = self.a();
                let n = ((a as u16) << 8) | n as u16;
                io.do_out(n, a);
            }
            0xd4 => { //CALL NC,nn
                let addr = self.fetch_u16(mem);
                if !flag8(self.f(), FLAG_C) {
                    let pc = self.pc;
                    self.push(mem, pc);
                    self.pc.set(addr);
                }
            }
            0xd5 => { //PUSH DE
                let de = self.de;
                self.push(mem, de);
            }
            0xd6 => { //SUB n
                let n = self.fetch(mem);
                let a = self.a();
                let a = self.sub_flags(a, n, false);
                self.set_a(a);
            }
            0xd7 => { //RST 10
                let pc = self.pc;
                self.push(mem, pc);
                self.pc.set(0x10);
            }
            0xd8 => { //RET C 
                if flag8(self.f(), FLAG_C) {
                    let pc = self.pop(mem);
                    self.pc.set(pc);
                }
            }
            0xd9 => { //EXX
                swap(&mut self.bc, &mut self.bc_);
                swap(&mut self.de, &mut self.de_);
                swap(&mut self.hl, &mut self.hl_);
            }
            0xda => { //JP C,nn
                let addr = self.fetch_u16(mem);
                if flag8(self.f(), FLAG_C) {
                    self.pc.set(addr);
                }
            }
            0xdb => { //IN A,(n)
                let n = self.fetch(mem);
                let a = self.a();
                let port = ((a as u16) << 8) | (n as u16); 
                let a = io.do_in(port);
                self.set_a(a);
            }
            0xdc => { //CALL C,nn
                let addr = self.fetch_u16(mem);
                if flag8(self.f(), FLAG_C) {
                    let pc = self.pc;
                    self.push(mem, pc);
                    self.pc.set(addr);
                }
            }
            0xde => { //SBC n
                let mut n = self.fetch(mem);
                let a = self.a();
                let a = self.sub_flags(a, n, true);
                self.set_a(a);
            }
            0xdf => { //RST 18
                let pc = self.pc;
                self.push(mem, pc);
                self.pc.set(0x18);
            }
            0xe0 => { //RET PO 
                if !flag8(self.f(), FLAG_PV) {
                    let pc = self.pop(mem);
                    self.pc.set(pc);
                }
            }
            0xe1 => { //POP HL
                let hl = self.pop(mem);
                self.hlx().set(hl);
            }
            0xe2 => { //JP PO,nn
                let addr = self.fetch_u16(mem);
                if !flag8(self.f(), FLAG_PV) {
                    self.pc.set(addr);
                }
            }
            0xe3 => { //EX (SP),HL
                let x = mem.peek_u16(self.sp);
                mem.poke_u16(self.sp, self.hlx().as_u16());
                self.hlx().set(x);
            }
            0xe4 => { //CALL PO,nn
                let addr = self.fetch_u16(mem);
                if !flag8(self.f(), FLAG_PV) {
                    let pc = self.pc;
                    self.push(mem, pc);
                    self.pc.set(addr);
                }
            }
            0xe5 => { //PUSH HL
                let hl = *self.hlx();
                self.push(mem, hl);
            }
            0xe6 => { //AND n
                let n = self.fetch(mem);
                let mut a = self.a();
                a = self.and_flags(a, n);
                self.set_a(a);
            }
            0xe7 => { //RST 20
                let pc = self.pc;
                self.push(mem, pc);
                self.pc.set(0x20);
            }
            0xe8 => { //RET PE 
                if flag8(self.f(), FLAG_PV) {
                    let pc = self.pop(mem);
                    self.pc.set(pc);
                }
            }
            0xe9 => { //JP (HL)
                self.pc = *self.hlx();
            }
            0xea => { //JP PE,nn
                let addr = self.fetch_u16(mem);
                if flag8(self.f(), FLAG_PV) {
                    self.pc.set(addr);
                }
            }
            0xeb => { //EX DE,HL
                swap(&mut self.de, &mut self.hl);
            }
            0xec => { //CALL PE,nn
                let addr = self.fetch_u16(mem);
                if flag8(self.f(), FLAG_PV) {
                    let pc = self.pc;
                    self.push(mem, pc);
                    self.pc.set(addr);
                }
            }
            0xee => { //XOR n
                let n = self.fetch(mem);
                let a = self.a();
                let a = self.xor_flags(a, n);
                self.set_a(a);
            }
            0xef => { //RST 28
                let pc = self.pc;
                self.push(mem, pc);
                self.pc.set(0x28);
            }
            0xf0 => { //RET P 
                if !flag8(self.f(), FLAG_S) {
                    let pc = self.pop(mem);
                    self.pc.set(pc);
                }
            }
            0xf1 => { //POP AF
                let af = self.pop(mem);
                self.af.set(af);
            }
            0xf2 => { //JP P,nn
                let addr = self.fetch_u16(mem);
                if !flag8(self.f(), FLAG_S) {
                    self.pc.set(addr);
                }
            }
            0xf3 => { //DI
                self.iff1 = false;
            }
            0xf4 => { //CALL P,nn
                let addr = self.fetch_u16(mem);
                if !flag8(self.f(), FLAG_S) {
                    let pc = self.pc;
                    self.push(mem, pc);
                    self.pc.set(addr);
                }
            }
            0xf5 => { //PUSH af
                let af = self.af;
                self.push(mem, af);
            }
            0xf6 => { //OR n
                let n = self.fetch(mem);
                let mut a = self.a();
                a = self.or_flags(a, n);
                self.set_a(a);
            }
            0xf7 => { //RST 30
                let pc = self.pc;
                self.push(mem, pc);
                self.pc.set(0x30);
            }
            0xf8 => { //RET M 
                if flag8(self.f(), FLAG_S) {
                    let pc = self.pop(mem);
                    self.pc.set(pc);
                }
            }
            0xf9 => { //LD SP,HL
                self.sp = *self.hlx();
            }
            0xfa => { //JP M,nn
                let addr = self.fetch_u16(mem);
                if flag8(self.f(), FLAG_S) {
                    self.pc.set(addr);
                }
            }
            0xfb => { //EI
                self.iff1 = true;
            }
            0xfc => { //CALL M,nn
                let addr = self.fetch_u16(mem);
                if flag8(self.f(), FLAG_S) {
                    let pc = self.pc;
                    self.push(mem, pc);
                    self.pc.set(addr);
                }
            }
            0xfe => { //CP n
                let n = self.fetch(mem);
                let a = self.a();
                self.sub_flags(a, n, false);
            }
            0xff => { //RST 38
                let pc = self.pc;
                self.push(mem, pc);
                self.pc.set(0x38);
            }
            _ => {
                let rs = c & 0x07;
                let rd = (c >> 3) & 0x07;
                match c & 0b1100_0000 {
                    0x40 => { //LD r,r
                        if rs == 6 {
                            let addr = self.hlx_addr(mem);
                            let r = self.reg_by_num(rs, mem, addr);
                            self.set_reg_by_num_no_pre(rd, mem, r);
                        } else if rd == 6 {
                            let addr = self.hlx_addr(mem);
                            let r = self.reg_by_num_no_pre(rs, mem);
                            self.set_reg_by_num(rd, mem, r, addr);
                        } else {
                            let r = self.reg_by_num(rs, mem, 0);
                            self.set_reg_by_num(rd, mem, r, 0);
                        }
                    }
                    _ => {
                        match c & 0b1111_1000 {
                            0x80 => { //ADD r
                                let addr = self.hlx_addr(mem);
                                let a = self.a();
                                let r = self.reg_by_num(rs, mem, addr);
                                let a = self.add_flags(a, r, false);
                                self.set_a(a);
                            }
                            0x88 => { //ADC r
                                let addr = self.hlx_addr(mem);
                                let a = self.a();
                                let r = self.reg_by_num(rs, mem, addr);
                                let a = self.add_flags(a, r, true);
                                self.set_a(a);
                            }
                            0x90 => { //SUB r
                                let addr = self.hlx_addr(mem);
                                let a = self.a();
                                let r = self.reg_by_num(rs, mem, addr);
                                let a = self.sub_flags(a, r, false);
                                self.set_a(a);
                            }
                            0x98 => { //SBC r
                                let addr = self.hlx_addr(mem);
                                let a = self.a();
                                let mut r = self.reg_by_num(rs, mem, addr);
                                let a = self.sub_flags(a, r, true);
                                self.set_a(a);
                            }
                            0xa0 => { //AND r
                                let addr = self.hlx_addr(mem);
                                let a = self.a();
                                let r = self.reg_by_num(rs, mem, addr);
                                let a = self.and_flags(a, r);
                                self.set_a(a);
                            }
                            0xa8 => { //XOR r
                                let addr = self.hlx_addr(mem);
                                let a = self.a();
                                let r = self.reg_by_num(rs, mem, addr);
                                let a = self.xor_flags(a, r);
                                self.set_a(a);
                            }
                            0xb0 => { //OR r
                                let addr = self.hlx_addr(mem);
                                let a = self.a();
                                let r = self.reg_by_num(rs, mem, addr);
                                let a = self.or_flags(a, r);
                                self.set_a(a);
                            }
                            0xb8 => { //CP r
                                let addr = self.hlx_addr(mem);
                                let a = self.a();
                                let r = self.reg_by_num(rs, mem, addr);
                                self.sub_flags(a, r, false);
                            }
                            _ => {
                                log!("unimplemented opcode {:02x}", c);
                            }
                        }
                    }
                }
            },
        }
        6
    }
    pub fn exec_cb(&mut self, mem: &mut Memory, io: &mut dyn InOut) {
        let addr = self.hlx_addr(mem);
        let c = self.fetch(mem);
        if self.prefix == XYPrefix::None {
            self.inc_r();
        }
        let r = c & 0x07;
        let n = (c >> 3) & 0x07;
        match c & 0b1100_0000 {
            0x40 => { //BIT n,r
                let b = self.reg_by_num(r, mem, addr);
                let r = b & (1 << n);
                let mut f = self.f();
                f = set_flag8(f, FLAG_S, flag8(r, 0x80));
                f = set_flag8(f, FLAG_Z, r == 0);
                f = set_flag8(f, FLAG_PV, parity(r));
                f = set_flag8(f, FLAG_H, true);
                f = set_flag8(f, FLAG_N, false);
                self.set_f(f);
            }
            0x80 => { //RES n,r
                let mut b = self.reg_by_num(r, mem, addr);
                b = set_flag8(b, 1 << n, false);
                self.set_reg_by_num(r, mem, b, addr);
            }
            0xc0 => { //SET n,r
                let mut b = self.reg_by_num(r, mem, addr);
                b = set_flag8(b, 1 << n, true);
                self.set_reg_by_num(r, mem, b, addr);
            }
            _ => match c & 0b1111_1000 {
                0x00 => { //RLC r
                    let mut b = self.reg_by_num(r, mem, addr);
                    let mut f = self.f();
                    let b7 = flag8(b, 0x80);
                    b = b.rotate_left(1);
                    f = set_flag8(f, FLAG_C, b7);
                    f = set_flag8(f, FLAG_N, false);
                    f = set_flag8(f, FLAG_H, false);
                    f = set_flag8(f, FLAG_PV, parity(b));
                    f = set_flag8(f, FLAG_Z, b == 0);
                    f = set_flag8(f, FLAG_S, flag8(b, 0x80));
                    self.set_reg_by_num(r, mem, b, addr);
                    self.set_f(f);
                }
                0x08 => { //RRC r
                    let mut b = self.reg_by_num(r, mem, addr);
                    let mut f = self.f();
                    let b0 = flag8(b, 0x01);
                    b = b.rotate_right(1);
                    f = set_flag8(f, FLAG_C, b0);
                    f = set_flag8(f, FLAG_N, false);
                    f = set_flag8(f, FLAG_H, false);
                    f = set_flag8(f, FLAG_PV, parity(b));
                    f = set_flag8(f, FLAG_Z, b == 0);
                    f = set_flag8(f, FLAG_S, flag8(b, 0x80));
                    self.set_reg_by_num(r, mem, b, addr);
                    self.set_f(f);
                }
                0x10 => { //RL r
                    let mut b = self.reg_by_num(r, mem, addr);
                    let mut f = self.f();
                    let b7 = flag8(b, 0x80);
                    let c = flag8(f, FLAG_C);
                    b <<= 1;
                    b = set_flag8(b, 1, c);
                    f = set_flag8(f, FLAG_C, b7);
                    f = set_flag8(f, FLAG_N, false);
                    f = set_flag8(f, FLAG_H, false);
                    f = set_flag8(f, FLAG_PV, parity(b));
                    f = set_flag8(f, FLAG_Z, b == 0);
                    f = set_flag8(f, FLAG_S, flag8(b, 0x80));
                    self.set_reg_by_num(r, mem, b, addr);
                    self.set_f(f);
                }
                0x18 => { //RR r
                    let mut b = self.reg_by_num(r, mem, addr);
                    let mut f = self.f();
                    let b0 = flag8(b, 0x01);
                    let c = flag8(f, FLAG_C);
                    b >>= 1;
                    b = set_flag8(b, 0x80, c);
                    f = set_flag8(f, FLAG_C, b0);
                    f = set_flag8(f, FLAG_N, false);
                    f = set_flag8(f, FLAG_H, false);
                    f = set_flag8(f, FLAG_PV, parity(b));
                    f = set_flag8(f, FLAG_Z, b == 0);
                    f = set_flag8(f, FLAG_S, flag8(b, 0x80));
                    self.set_reg_by_num(r, mem, b, addr);
                    self.set_f(f);
                }
                0x20 => { //SLA r
                    let mut b = self.reg_by_num(r, mem, addr);
                    let mut f = self.f();
                    let b7 = flag8(b, 0x80);
                    b <<= 1;
                    f = set_flag8(f, FLAG_C, b7);
                    f = set_flag8(f, FLAG_N, false);
                    f = set_flag8(f, FLAG_H, false);
                    f = set_flag8(f, FLAG_PV, parity(b));
                    f = set_flag8(f, FLAG_Z, b == 0);
                    f = set_flag8(f, FLAG_S, flag8(b, 0x80));
                    self.set_reg_by_num(r, mem, b, addr);
                    self.set_f(f);
                }
                0x28 => { //SRA r
                    let mut b = self.reg_by_num(r, mem, addr);
                    let mut f = self.f();
                    let b0 = flag8(b, 0x01);
                    b = ((b as i8) >> 1) as u8;
                    f = set_flag8(f, FLAG_C, b0);
                    f = set_flag8(f, FLAG_N, false);
                    f = set_flag8(f, FLAG_H, false);
                    f = set_flag8(f, FLAG_PV, parity(b));
                    f = set_flag8(f, FLAG_Z, b == 0);
                    f = set_flag8(f, FLAG_S, flag8(b, 0x80));
                    self.set_reg_by_num(r, mem, b, addr);
                    self.set_f(f);
                }
                0x30 => { //SL1 r (undoc)
                    let mut b = self.reg_by_num(r, mem, addr);
                    let mut f = self.f();
                    let b7 = flag8(b, 0x80);
                    b = (b << 1) | 1;
                    f = set_flag8(f, FLAG_C, b7);
                    f = set_flag8(f, FLAG_N, false);
                    f = set_flag8(f, FLAG_H, false);
                    f = set_flag8(f, FLAG_PV, parity(b));
                    f = set_flag8(f, FLAG_Z, b == 0);
                    f = set_flag8(f, FLAG_S, flag8(b, 0x80));
                    self.set_reg_by_num(r, mem, b, addr);
                    self.set_f(f);
                }
                0x38 => { //SRL r
                    let mut b = self.reg_by_num(r, mem, addr);
                    let mut f = self.f();
                    let b0 = flag8(b, 0x01);
                    b >>= 1;
                    f = set_flag8(f, FLAG_C, b0);
                    f = set_flag8(f, FLAG_N, false);
                    f = set_flag8(f, FLAG_H, false);
                    f = set_flag8(f, FLAG_PV, parity(b));
                    f = set_flag8(f, FLAG_Z, b == 0);
                    f = set_flag8(f, FLAG_S, flag8(b, 0x80));
                    self.set_reg_by_num(r, mem, b, addr);
                    self.set_f(f);
                }
                _ => {
                    log!("unimplemented opcode CB {:02x}", c);
                },
            }
        }
    }
    pub fn exec_ed(&mut self, mem: &mut Memory, io: &mut dyn InOut) {
        let c = self.fetch(mem);
        if self.prefix == XYPrefix::None {
            self.inc_r();
        }
        match c {
            0x40 => { //IN B,(C)
                let bc = self.bc.as_u16();
                let mut f = self.f();
                let b = io.do_in(bc);
                f = set_flag8(f, FLAG_N, false);
                f = set_flag8(f, FLAG_PV, parity(b));
                f = set_flag8(f, FLAG_Z, b == 0);
                f = set_flag8(f, FLAG_S, flag8(b, 0x80));
                //FLAG_H
                self.bc.set_hi(b);
                self.set_f(f);
            }
            0x41 => { //OUT (C),B
                let bc = self.bc.as_u16();
                io.do_out(bc, self.bc.hi());
            }
            0x42 => { //SBC HL,BC
                let mut hl = self.hl.as_u16();
                let mut bc = self.bc.as_u16();
                if flag8(self.f(), FLAG_C) {
                    bc = bc.wrapping_add(1);
                }
                hl = self.sub16_flags(hl, bc);
                self.hl.set(hl);
            }
            0x43 => { //LD (nn),BC
                let addr = self.fetch_u16(mem);
                mem.poke_u16(addr, self.bc.as_u16());
            }
            0x44 => { //NEG
                let a = self.a();
                let a = self.sub_flags(0, a, false);
                self.set_a(a);
            }
            0x45 => { //RETN
                let pc = self.pop(mem);
                self.pc.set(pc);
                //TODO iff2
            }
            0x46 => { // /IM 0
                self.im = InterruptMode::IM0;
            }
            0x47 => { //LD I,A
                self.i = self.a();
            }
            0x48 => { //IN C,(C)
                let bc = self.bc.as_u16();
                let mut f = self.f();
                let b = io.do_in(bc);
                f = set_flag8(f, FLAG_N, false);
                f = set_flag8(f, FLAG_PV, parity(b));
                f = set_flag8(f, FLAG_Z, b == 0);
                f = set_flag8(f, FLAG_S, flag8(b, 0x80));
                //FLAG_H
                self.bc.set_lo(b);
                self.set_f(f);
            }
            0x49 => { //OUT (C),C
                let bc = self.bc.as_u16();
                io.do_out(bc, self.bc.lo());
            }
            0x4a => { //ADC HL,BC
                let mut hl = self.hl.as_u16();
                let mut bc = self.bc.as_u16();
                if flag8(self.f(), FLAG_C) {
                    bc = bc.wrapping_add(1);
                }
                hl = self.add16_flags(hl, bc);
                self.hl.set(hl);
            }
            0x4b => { //LD BC,(nn)
                let addr = self.fetch_u16(mem);
                self.bc.set(mem.peek_u16(addr));
            }
            0x4d => { //RETI
                let pc = self.pop(mem);
                self.pc.set(pc);
            }
            0x4f => { //LD R,A
                let a = self.a();
                self.set_r(a);
            }
            0x50 => { //IN D,(C)
                let bc = self.bc.as_u16();
                let mut f = self.f();
                let b = io.do_in(bc);
                f = set_flag8(f, FLAG_N, false);
                f = set_flag8(f, FLAG_PV, parity(b));
                f = set_flag8(f, FLAG_Z, b == 0);
                f = set_flag8(f, FLAG_S, flag8(b, 0x80));
                //FLAG_H
                self.de.set_hi(b);
                self.set_f(f);
            }
            0x51 => { //OUT (C),D
                let bc = self.bc.as_u16();
                io.do_out(bc, self.de.hi());
            }
            0x52 => { //SBC HL,DE
                let mut hl = self.hl.as_u16();
                let mut de = self.de.as_u16();
                if flag8(self.f(), FLAG_C) {
                    de = de.wrapping_add(1);
                }
                hl = self.sub16_flags(hl, de);
                self.hl.set(hl);
            }
            0x53 => { //LD (nn),DE
                let addr = self.fetch_u16(mem);
                mem.poke_u16(addr, self.de.as_u16());
            }
            0x56 => { //IM 1
                self.im = InterruptMode::IM1;
            }
            0x57 => { //LD A,I
                let i = self.i;
                let mut f = self.f();
                f = set_flag8(f, FLAG_H, false);
                f = set_flag8(f, FLAG_N, false);
                f = set_flag8(f, FLAG_PV, self.iff1);
                f = set_flag8(f, FLAG_Z, i == 0);
                f = set_flag8(f, FLAG_S, flag8(i, 0x80));
                self.set_a(i);
                self.set_f(f);
            }
            0x58 => { //IN H,(C)
                let bc = self.bc.as_u16();
                let mut f = self.f();
                let b = io.do_in(bc);
                f = set_flag8(f, FLAG_N, false);
                f = set_flag8(f, FLAG_PV, parity(b));
                f = set_flag8(f, FLAG_Z, b == 0);
                f = set_flag8(f, FLAG_S, flag8(b, 0x80));
                //FLAG_H
                self.de.set_lo(b);
                self.set_f(f);
            }
            0x59 => { //OUT (C),H
                let bc = self.bc.as_u16();
                io.do_out(bc, self.hl.hi());
            }
            0x5a => { //ADC HL,DE
                let mut hl = self.hl.as_u16();
                let mut de = self.de.as_u16();
                if flag8(self.f(), FLAG_C) {
                    de = de.wrapping_add(1);
                }
                hl = self.add16_flags(hl, de);
                self.hl.set(hl);
            }
            0x5b => { //LD DE,(nn)
                let addr = self.fetch_u16(mem);
                self.de.set(mem.peek_u16(addr));
            }
            0x5e => { //IM 2
                self.im = InterruptMode::IM2;
            }
            0x5f => { //LD A,R
                let r = self.r();
                let mut f = self.f();
                f = set_flag8(f, FLAG_H, false);
                f = set_flag8(f, FLAG_N, false);
                f = set_flag8(f, FLAG_PV, self.iff1);
                f = set_flag8(f, FLAG_Z, r == 0);
                f = set_flag8(f, FLAG_S, flag8(r, 0x80));
                self.set_a(r);
                self.set_f(f);
            }
            0x60 => { //IN H,(C)
                let bc = self.bc.as_u16();
                let mut f = self.f();
                let b = io.do_in(bc);
                f = set_flag8(f, FLAG_N, false);
                f = set_flag8(f, FLAG_PV, parity(b));
                f = set_flag8(f, FLAG_Z, b == 0);
                f = set_flag8(f, FLAG_S, flag8(b, 0x80));
                //FLAG_H
                self.hl.set_hi(b);
                self.set_f(f);
            }
            0x61 => { //OUT (C),H
                let bc = self.bc.as_u16();
                io.do_out(bc, self.hl.hi());
            }
            0x62 => { //SBC HL,HL
                let mut hl = self.hl.as_u16();
                let mut rr = self.hl.as_u16();
                if flag8(self.f(), FLAG_C) {
                    rr = rr.wrapping_add(1);
                }
                hl = self.sub16_flags(hl, rr);
                self.hl.set(hl);
            }
            0x63 => { //LD (nn),HL
                let addr = self.fetch_u16(mem);
                mem.poke_u16(addr, self.hl.as_u16());
            }
            0x67 => { //RRD
                let x = mem.peek(self.hl);
                let a = self.a();
                let mut f = self.f();
                let new_a = (a & 0xf0) | (x & 0x0f);
                let new_x = ((a & 0x0f) << 4) | ((x & 0xf0) >> 4);
                f = set_flag8(f, FLAG_H, false);
                f = set_flag8(f, FLAG_N, false);
                f = set_flag8(f, FLAG_PV, parity(new_a));
                f = set_flag8(f, FLAG_Z, new_a == 0);
                f = set_flag8(f, FLAG_S, flag8(new_a, 0x80));
                self.set_a(new_a);
                self.set_f(f);
                mem.poke(self.hl, new_x);
            }
            0x68 => { //IN L,(C)
                let bc = self.bc.as_u16();
                let mut f = self.f();
                let b = io.do_in(bc);
                f = set_flag8(f, FLAG_N, false);
                f = set_flag8(f, FLAG_PV, parity(b));
                f = set_flag8(f, FLAG_Z, b == 0);
                f = set_flag8(f, FLAG_S, flag8(b, 0x80));
                //FLAG_H
                self.hl.set_lo(b);
                self.set_f(f);
            }
            0x69 => { //OUT (C),L
                let bc = self.bc.as_u16();
                io.do_out(bc, self.hl.lo());
            }
            0x6a => { //ADC HL,HL
                let mut hl = self.hl.as_u16();
                let mut hl2 = hl;
                if flag8(self.f(), FLAG_C) {
                    hl2 = hl2.wrapping_add(1);
                }
                hl = self.add16_flags(hl, hl2);
                self.hl.set(hl);
            }
            0x6b => { //LD HL,(nn)
                let addr = self.fetch_u16(mem);
                self.hl.set(mem.peek_u16(addr));
            }
            0x6f => { //RLD
                let x = mem.peek(self.hl);
                let a = self.a();
                let mut f = self.f();
                let new_a = (a & 0xf0) | ((x & 0xf0) >> 4);
                let new_x = (a & 0x0f) | ((x & 0x0f) << 4);
                f = set_flag8(f, FLAG_H, false);
                f = set_flag8(f, FLAG_N, false);
                f = set_flag8(f, FLAG_PV, parity(new_a));
                f = set_flag8(f, FLAG_Z, new_a == 0);
                f = set_flag8(f, FLAG_S, flag8(new_a, 0x80));
                self.set_a(new_a);
                self.set_f(f);
                mem.poke(self.hl, new_x);
            }
            0x70 => { //IN F,(C)
                let bc = self.bc.as_u16();
                let b = io.do_in(bc);
                self.set_f(b);
            }
            0x71 => { //OUT (C),F
                let bc = self.bc.as_u16();
                io.do_out(bc, self.f());
            }
            0x72 => { //SBC HL,SP
                let mut hl = self.hl.as_u16();
                let mut sp : u16 = self.sp.as_u16();
                if flag8(self.f(), FLAG_C) {
                    sp = sp.wrapping_add(1);
                }
                hl = self.sub16_flags(hl, sp);
                self.hl.set(hl);
            }
            0x73 => { //LD (nn),SP
                let addr = self.fetch_u16(mem);
                mem.poke_u16(addr, self.sp.as_u16());
            }
            0x78 => { //IN A,(C)
                let bc = self.bc.as_u16();
                let mut f = self.f();
                let b = io.do_in(bc);
                f = set_flag8(f, FLAG_N, false);
                f = set_flag8(f, FLAG_PV, parity(b));
                f = set_flag8(f, FLAG_Z, b == 0);
                f = set_flag8(f, FLAG_S, flag8(b, 0x80));
                f = set_flag8(f, FLAG_H, false);
                //FLAG_H
                self.set_a(b);
                self.set_f(f);
            }
            0x79 => { //OUT (C),A
                let bc = self.bc.as_u16();
                io.do_out(bc, self.a());
            }
            0x7a => { //ADC HL,SP
                let mut hl = self.hl.as_u16();
                let mut sp = self.sp.as_u16();
                if flag8(self.f(), FLAG_C) {
                    sp = sp.wrapping_add(1);
                }
                hl = self.add16_flags(hl, sp);
                self.hl.set(hl);
            }
            0x7b => { //LD SP,(nn)
                let addr = self.fetch_u16(mem);
                self.sp.set(mem.peek_u16(addr));
            }
            0xa0 => { //LDI
                self.ldi(mem);
            }
            0xa1 => { //CPI
                self.cpi(mem);
            }
            0xa8 => { //LDD
                self.ldd(mem);
            }
            0xa9 => { //CPD
                self.cpd(mem);
            }
            0xb0 => { //LDIR
                self.ldi(mem);
                if self.bc.as_u16() != 0 {
                    self.pc -= 2;
                }
            }
            0xb1 => { //CPIR
                let r = self.cpi(mem);
                if self.bc.as_u16() != 0 && r != 0 {
                    self.pc -= 2;
                }
            }
            0xb8 => { //LDDR
                self.ldd(mem);
                if self.bc.as_u16() != 0 {
                    self.pc -= 2;
                }
            }
            0xb9 => { //CPDR
                let r = self.cpd(mem);
                if self.bc.as_u16() != 0 && r != 0 {
                    self.pc -= 2;
                }
            }
            _ => {
                log!("unimplemented opcode ED {:02x}", c);
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

