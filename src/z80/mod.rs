use std::mem::swap;

mod r16;

use super::Memory;
use self::r16::R16;

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
fn set_flag8(f: &mut u8, bit: u8, set: bool) {
    if set {
        *f |= bit;
    } else {
        *f &= !bit;
    }
}
#[inline]
fn set_flag16(f: &mut u16, bit: u16, set: bool) {
    if set {
        *f |= bit;
    } else {
        *f &= !bit;
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

enum InterruptMode {
    IM0, IM1, IM2,
}

#[derive(PartialEq, Eq, Debug)]
enum XYPrefix {
    None, IX, IY,
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
    ir: R16,
    iff1: bool,
    im: InterruptMode,
    prefix: XYPrefix,
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
            ir: R16::default(),
            iff1: false,
            im: InterruptMode::IM0,
            prefix: XYPrefix::None,
        }
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
        self.sp -= 2;
        mem.poke_u16(self.sp, x.into());
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
            7 => self.af.hi(),
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
            7 => self.af.set_hi(b),
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
    fn sub_flags(&mut self, a: u8, b: u8) -> u8 {
        let r = a.wrapping_sub(b);
        let mut f = self.af.lo();
        set_flag8(&mut f, FLAG_N, true);
        set_flag8(&mut f, FLAG_C, carry8(r, b, a));
        set_flag8(&mut f, FLAG_PV,
                 (flag8(a, 0x80) == flag8(b, 0x80) && flag8(a, 0x80) != flag8(r, 0x80)));
        set_flag8(&mut f, FLAG_Z, r == 0);
        set_flag8(&mut f, FLAG_S, flag8(r, 0x80));
        //TODO FLAG_H
        self.af.set_lo(f);
        r
    }
    fn sub16_flags(&mut self, a: u16, b: u16) -> u16 {
        let r = a.wrapping_sub(b);
        let mut f = self.af.lo();
        set_flag8(&mut f, FLAG_N, true);
        set_flag8(&mut f, FLAG_C, carry16(r, b, a));
        set_flag8(&mut f, FLAG_PV,
                 flag16(a, 0x8000) == flag16(b, 0x8000) && flag16(a, 0x8000) != flag16(r, 0x8000));
        set_flag8(&mut f, FLAG_Z, r == 0);
        set_flag8(&mut f, FLAG_S, flag16(r, 0x8000));
        self.af.set_lo(f);
        r
    }
    fn add_flags(&mut self, a: u8, b: u8) -> u8 {
        let r = a.wrapping_add(b);
        let mut f = self.af.lo();
        set_flag8(&mut f, FLAG_N, false);
        set_flag8(&mut f, FLAG_C, carry8(a, b, r));
        set_flag8(&mut f, FLAG_PV,
                 (flag8(a, 0x80) == flag8(b, 0x80) && flag8(a, 0x80) != flag8(r, 0x80)));
        set_flag8(&mut f, FLAG_Z, r == 0);
        set_flag8(&mut f, FLAG_S, flag8(r, 0x80));
        //TODO FLAG_H
        self.af.set_lo(f);
        r
    }
    fn add16_flags(&mut self, a: u16, b: u16) -> u16 {
        let r = a.wrapping_add(b);
        let mut f = self.af.lo();
        set_flag8(&mut f, FLAG_N, false);
        set_flag8(&mut f, FLAG_C, carry16(a, b, r));
        self.af.set_lo(f);
        r
    }
    fn inc_flags(&mut self, a: u8) -> u8 {
        let r = a.wrapping_add(1);
        let mut f = self.af.lo();
        set_flag8(&mut f, FLAG_N, false);
        set_flag8(&mut f, FLAG_PV, r == 0x80);
        set_flag8(&mut f, FLAG_Z,
                 r == 0);
        set_flag8(&mut f, FLAG_S, flag8(r, 0x80));
        //TODO FLAG_H
        self.af.set_lo(f);
        r
    }
    fn dec_flags(&mut self, a: u8) -> u8 {
        let r = a.wrapping_sub(1);
        let mut f = self.af.lo();
        set_flag8(&mut f, FLAG_N, true);
        set_flag8(&mut f, FLAG_PV, r == 0x7f);
        set_flag8(&mut f, FLAG_Z, r == 0);
        set_flag8(&mut f, FLAG_S, flag8(r, 0x80));
        //TODO FLAG_H
        self.af.set_lo(f);
        r
    }
    fn and_flags(&mut self, a: u8, b: u8) -> u8 {
        let r = a & b;
        let mut f = self.af.lo();
        set_flag8(&mut f, FLAG_C, false);
        set_flag8(&mut f, FLAG_N, false);
        set_flag8(&mut f, FLAG_PV, parity(r));
        set_flag8(&mut f, FLAG_Z, r == 0);
        set_flag8(&mut f, FLAG_S, flag8(r, 0x80));
        //TODO FLAG_H
        self.af.set_lo(f);
        r
    }
    fn or_flags(&mut self, a: u8, b: u8) -> u8 {
        let r = a | b;
        let mut f = self.af.lo();
        set_flag8(&mut f, FLAG_C, false);
        set_flag8(&mut f, FLAG_N, false);
        set_flag8(&mut f, FLAG_PV, parity(r));
        set_flag8(&mut f, FLAG_Z, r == 0);
        set_flag8(&mut f, FLAG_S, flag8(r, 0x80));
        //TODO FLAG_H
        self.af.set_lo(f);
        r
    }
    fn xor_flags(&mut self, a: u8, b: u8) -> u8 {
        let r = a ^ b;
        let mut f = self.af.lo();
        set_flag8(&mut f, FLAG_C, false);
        set_flag8(&mut f, FLAG_N, false);
        set_flag8(&mut f, FLAG_PV, parity(r));
        set_flag8(&mut f, FLAG_Z, r == 0);
        set_flag8(&mut f, FLAG_S, flag8(r, 0x80));
        //TODO FLAG_H
        self.af.set_lo(f);
        r
    }
    pub fn exec(&mut self, mem: &mut Memory) {
        println!("PC {:4x}", self.pc.as_u16());
        let c = match self.fetch(mem) {
            0xdd => {
                self.prefix = XYPrefix::IX;
                self.fetch(mem)
            }
            0xfd => {
                self.prefix = XYPrefix::IY;
                self.fetch(mem)
            }
            c => {
                self.prefix = XYPrefix::None;
                c
            }
        };
        if self.prefix != XYPrefix::None {
            println!("prefix {:?} {:2x}", self.prefix, c);
        }
        match c {
            0xcb => { self.exec_cb(mem); }
            0xed => { self.exec_ed(mem); }

            0x00 => { //NOP
            }
            0x01 => { //LD BC,nn
                let d = self.fetch_u16(mem);
                self.bc.set(d);
            }
            0x02 => { //LD (BC),A
                let a = self.af.hi();
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
                let mut a = self.af.hi();
                let mut f = self.af.lo();
                let b7 = flag8(a, 7);
                a = a.rotate_left(1);
                set_flag8(&mut f, FLAG_C, b7);
                set_flag8(&mut f, FLAG_N, false);
                set_flag8(&mut f, FLAG_H, false);
                self.af.set_hi(a);
                self.af.set_lo(f);
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
                self.af.set_hi(a);
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
                let mut a = self.af.hi();
                let mut f = self.af.lo();
                let b0 = flag8(a, 0);
                a = a.rotate_right(1);
                set_flag8(&mut f, FLAG_C, b0);
                set_flag8(&mut f, FLAG_N, false);
                set_flag8(&mut f, FLAG_H, false);
                self.af.set_hi(a);
                self.af.set_lo(f);
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
                self.de = self.fetch_u16(mem).into();
            }
            0x12 => { //LD (DE),A
                let a = self.af.hi();
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
                self.af.set_hi(a);
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
            0x20 => { //JR NZ,d
                let d = self.fetch(mem);
                if !flag8(self.af.lo(), FLAG_Z) {
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
            0x28 => { //JR Z,d
                let d = self.fetch(mem);
                if flag8(self.af.lo(), FLAG_Z) {
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
                let mut a = self.af.hi();
                let mut f = self.af.lo();
                a = a ^ 0xff;
                set_flag8(&mut f, FLAG_H, true);
                set_flag8(&mut f, FLAG_N, true);
                self.af.set_hi(a);
            }
            0x30 => { //JR NC,d
                let d = self.fetch(mem);
                if !flag8(self.af.lo(), FLAG_C) {
                    self.pc += d as i8 as i16 as u16;
                }
            }
            0x31 => { //LD SP,nn
                let nn = self.fetch_u16(mem);
                self.sp.set(nn);
            }
            0x32 => { //LD (nn),A
                let addr = self.fetch_u16(mem);
                mem.poke(addr, self.af.hi());
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
                let mut f = self.af.lo();
                set_flag8(&mut f, FLAG_N, false);
                set_flag8(&mut f, FLAG_H, false);
                set_flag8(&mut f, FLAG_C, true);
                self.af.set_lo(f);
            }
            0x38 => { //JR C,d
                let d = self.fetch(mem);
                if flag8(self.af.lo(), FLAG_C) {
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
                self.af.set_hi(x);
            }
            0x3b => { //DEC SP
                self.sp -= 1;
            }
            0x3c => { //INC A
                let mut r = self.af.hi();
                r = self.inc_flags(r);
                self.af.set_hi(r);
            }
            0x3d => { //DEC A
                let mut r = self.af.hi();
                r = self.dec_flags(r);
                self.af.set_hi(r);
            }
            0x3e => { //LD A,n
                let n = self.fetch(mem);
                self.af.set_hi(n);
            }
            0x3f => { //CCF
                let mut f = self.af.lo();
                set_flag8(&mut f, FLAG_N, false);
                set_flag8(&mut f, FLAG_H, false);
                f ^= FLAG_C;
                self.af.set_lo(f);
            }
            0x76 => { //HALT
                println!("unimplemented HALT");
            }
            0xc0 => { //RET NZ 
                if !flag8(self.af.lo(), FLAG_Z) {
                    let pc = self.pop(mem);
                    self.pc.set(pc);
                }
            }
            0xc1 => { //POP BC
                let bc = self.pop(mem);
                self.bc.set(bc);
            }
            0xc3 => { //JP nn
                let pc = self.fetch_u16(mem);
                self.pc.set(pc);
            }
            0xc5 => { //PUSH BC
                let bc = self.bc;
                self.push(mem, bc);
            }
            0xc6 => { //ADD n
                let n = self.fetch(mem);
                let a = self.af.hi();
                let a = self.add_flags(a, n);
                self.af.set_hi(a);
            }
            0xc8 => { //RET Z 
                if flag8(self.af.lo(), FLAG_Z) {
                    let pc = self.pop(mem);
                    self.pc.set(pc);
                }
            }
            0xc9 => { //RET
                let pc = self.pop(mem);
                self.pc.set(pc);
            }
            0xcd => { //CALL nn
                let addr = self.fetch_u16(mem);
                let pc = self.pc;
                self.push(mem, pc);
                self.pc = addr.into();
            }
            0xce => { //ADC n
                let mut n = self.fetch(mem);
                let a = self.af.hi();
                if flag8(self.af.lo(), FLAG_C) {
                    n = n.wrapping_add(1);
                }
                let a = self.add_flags(a, n);
                self.af.set_hi(a);
            }
            0xd0 => { //RET NC 
                if !flag8(self.af.lo(), FLAG_C) {
                    let pc = self.pop(mem);
                    self.pc.set(pc);
                }
            }
            0xd1 => { //POP DE
                let de = self.pop(mem);
                self.de.set(de);
            }
            0xd3 => { //OUT (n),A
                let n = self.fetch(mem);
                println!("OUT {:2x}, {:2x}", n, self.af.hi());
            }
            0xd5 => { //PUSH DE
                let de = self.de;
                self.push(mem, de);
            }
            0xd6 => { //SUB n
                let n = self.fetch(mem);
                let a = self.af.hi();
                let a = self.sub_flags(a, n);
                self.af.set_hi(a);
            }
            0xd8 => { //RET C 
                if flag8(self.af.lo(), FLAG_C) {
                    let pc = self.pop(mem);
                    self.pc.set(pc);
                }
            }
            0xd9 => { //EXX
                swap(&mut self.bc, &mut self.bc_);
                swap(&mut self.de, &mut self.de_);
                swap(&mut self.hl, &mut self.hl_);
            }
            0xde => { //SBC n
                let mut n = self.fetch(mem);
                let a = self.af.hi();
                if flag8(self.af.lo(), FLAG_C) {
                    n = n.wrapping_add(1);
                }
                let a = self.sub_flags(a, n);
                self.af.set_hi(a);
            }
            0xe0 => { //RET PO 
                if !flag8(self.af.lo(), FLAG_PV) {
                    let pc = self.pop(mem);
                    self.pc.set(pc);
                }
            }
            0xe1 => { //POP HL
                let hl = self.pop(mem);
                self.hlx().set(hl);
            }
            0xe3 => { //EX (SP),HL
                let x = mem.peek_u16(self.sp);
                mem.poke_u16(self.sp, self.hlx().as_u16());
                self.hlx().set(x);
            }
            0xe5 => { //PUSH HL
                let hl = *self.hlx();
                self.push(mem, hl);
            }
            0xe6 => { //AND n
                let n = self.fetch(mem);
                let mut a = self.af.hi();
                a = self.and_flags(a, n);
                self.af.set_hi(a);
            }
            0xe8 => { //RET PE 
                if flag8(self.af.lo(), FLAG_PV) {
                    let pc = self.pop(mem);
                    self.pc.set(pc);
                }
            }
            0xe9 => { //JP (HL)
                self.pc = *self.hlx();
            }
            0xeb => { //EX DE,HL
                swap(&mut self.de, &mut self.hl);
            }
            0xee => { //XOR n
                let n = self.fetch(mem);
                let a = self.af.hi();
                let a = self.xor_flags(a, n);
                self.af.set_hi(a);
            }
            0xf0 => { //RET P 
                if !flag8(self.af.lo(), FLAG_S) {
                    let pc = self.pop(mem);
                    self.pc.set(pc);
                }
            }
            0xf1 => { //POP AF
                let af = self.pop(mem);
                self.af.set(af);
            }
            0xf3 => { //DI
                self.iff1 = false;
            }
            0xf5 => { //PUSH af
                let af = self.af;
                self.push(mem, af);
            }
            0xf6 => { //OR n
                let n = self.fetch(mem);
                let mut a = self.af.hi();
                a = self.or_flags(a, n);
                self.af.set_hi(a);
            }
            0xf8 => { //RET M 
                if flag8(self.af.lo(), FLAG_S) {
                    let pc = self.pop(mem);
                    self.pc.set(pc);
                }
            }
            0xf9 => { //LD SP,HL
                self.sp = *self.hlx();
            }
            0xfb => { //EI
                self.iff1 = true;
            }
            0xfe => { //CP n
                let n = self.fetch(mem);
                let a = self.af.hi();
                self.sub_flags(a, n);
            }
            _ => {
                let rs = c & 0x07;
                let rd = (c >> 3) & 0x07;
                match c & 0b1100_0000 {
                    0x40 => { //LD r,r
                        let addr = self.hlx_addr(mem);
                        let r = self.reg_by_num(rs, mem, addr);
                        self.set_reg_by_num(rd, mem, r, addr);
                    }
                    _ => {
                        match c & 0b1111_1000 {
                            0x80 => { //ADD r
                                let addr = self.hlx_addr(mem);
                                let a = self.af.hi();
                                let r = self.reg_by_num(rs, mem, addr);
                                let a = self.add_flags(a, r);
                                self.af.set_hi(a);
                            }
                            0x88 => { //ADC r
                                let addr = self.hlx_addr(mem);
                                let a = self.af.hi();
                                let mut r = self.reg_by_num(rs, mem, addr);
                                if flag8(self.af.lo(), FLAG_C) {
                                    r = r.wrapping_add(1);
                                }
                                let a = self.add_flags(a, r);
                                self.af.set_hi(a);
                            }
                            0x90 => { //SUB r
                                let addr = self.hlx_addr(mem);
                                let a = self.af.hi();
                                let r = self.reg_by_num(rs, mem, addr);
                                let a = self.sub_flags(a, r);
                                self.af.set_hi(a);
                            }
                            0x98 => { //SBC r
                                let addr = self.hlx_addr(mem);
                                let a = self.af.hi();
                                let mut r = self.reg_by_num(rs, mem, addr);
                                if flag8(self.af.lo(), FLAG_C) {
                                    r = r.wrapping_add(1);
                                }
                                let a = self.sub_flags(a, r);
                                self.af.set_hi(a);
                            }
                            0xa0 => { //AND r
                                let addr = self.hlx_addr(mem);
                                let a = self.af.hi();
                                let r = self.reg_by_num(rs, mem, addr);
                                let a = self.and_flags(a, r);
                                self.af.set_hi(a);
                            }
                            0xa8 => { //XOR r
                                let addr = self.hlx_addr(mem);
                                let a = self.af.hi();
                                let r = self.reg_by_num(rs, mem, addr);
                                let a = self.xor_flags(a, r);
                                self.af.set_hi(a);
                            }
                            0xb0 => { //OR r
                                let addr = self.hlx_addr(mem);
                                let a = self.af.hi();
                                let r = self.reg_by_num(rs, mem, addr);
                                let a = self.or_flags(a, r);
                                self.af.set_hi(a);
                            }
                            0xb8 => { //CP r
                                let addr = self.hlx_addr(mem);
                                let a = self.af.hi();
                                let r = self.reg_by_num(rs, mem, addr);
                                self.sub_flags(a, r);
                            }
                            _ => {
                                println!("unimplemented opcode {:02x}", c);
                            }
                        }
                    }
                }
            },
        }
    }
    pub fn exec_cb(&mut self, mem: &mut Memory) {
        let addr = self.hlx_addr(mem);
        let c = self.fetch(mem);
        let r = c & 0x07;
        let n = (c >> 3) & 0x07;
        match c & 0b1100_0000 {
            0x40 => { //BIT n,r
                let b = self.reg_by_num(r, mem, addr);
                self.and_flags(b, 1 << n);
            }
            0x80 => { //RES n,r
                let mut b = self.reg_by_num(r, mem, addr);
                set_flag8(&mut b, n, false);
                self.set_reg_by_num(r, mem, b, addr);
            }
            0xc0 => { //SET n,r
                let mut b = self.reg_by_num(r, mem, addr);
                set_flag8(&mut b, n, true);
                self.set_reg_by_num(r, mem, b, addr);
            }
            _ => {
                println!("unimplemented opcode CB {:02x}", c);
            },
        }
    }
    pub fn exec_ed(&mut self, mem: &mut Memory) {
        let c = self.fetch(mem);
        match c {
            0x43 => { //LD (nn),BC
                let addr = self.fetch_u16(mem);
                mem.poke_u16(addr, self.bc.into());
            }
            0x47 => { //LD I,A
                self.ir.set_hi(self.af.hi());
            }
            0x52 => { //SBC HL,DE
                let mut hl = self.hl.into();
                let mut de : u16 = self.de.into();
                if flag8(self.af.lo(), FLAG_C) {
                    de = de.wrapping_add(1);
                }
                hl = self.sub16_flags(hl, de);
                self.hl = hl.into();
            }
            0x53 => { //LD (nn),DE
                let addr = self.fetch_u16(mem);
                mem.poke_u16(addr, self.de.into());
            }
            0x56 => { //IM 1
                self.im = InterruptMode::IM1;
            }
            0xb0 => { //LDIR
                let hl = self.hl;
                let de = self.de;
                let x = mem.peek(hl);
                mem.poke(de, x);

                self.hl += 1;
                self.de += 1;
                self.bc -= 1;

                let mut f = self.af.lo();
                set_flag8(&mut f, FLAG_N, false);
                set_flag8(&mut f, FLAG_H, false);
                set_flag8(&mut f, FLAG_PV, false);
                self.af.set_lo(f);

                if self.bc.as_u16() != 0 {
                    self.pc -= 2;
                }
            }
            0xb8 => { //LDDR
                let hl = self.hl;
                let de = self.de;
                let x = mem.peek(hl);
                mem.poke(de, x);

                self.hl -= 1;
                self.de -= 1;
                self.bc -= 1;

                let mut f = self.af.lo();
                set_flag8(&mut f, FLAG_N, false);
                set_flag8(&mut f, FLAG_H, false);
                set_flag8(&mut f, FLAG_PV, false);
                self.af.set_lo(f);

                if self.bc.as_u16() != 0 {
                    self.pc -= 2;
                }
            }
            _ => {
                println!("unimplemented opcode ED {:02x}", c);
            },
        }
    }
}
