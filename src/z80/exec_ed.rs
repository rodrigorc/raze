use super::*;

impl Z80 {
    pub(super) fn exec_ed(&mut self, prefix: XYPrefix, bus: &mut impl Bus) -> u32 {
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
                self.set_b(b);
                self.set_f(f);
                12
            }
            0x41 => { //OUT (C),B
                let bc = self.bc.as_u16();
                bus.do_out(bc, self.b());
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
                self.set_c(b);
                self.set_f(f);
                12
            }
            0x49 => { //OUT (C),C
                let bc = self.bc.as_u16();
                bus.do_out(bc, self.c());
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
                self.set_d(b);
                self.set_f(f);
                12
            }
            0x51 => { //OUT (C),D
                let bc = self.bc.as_u16();
                bus.do_out(bc, self.d());
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
                self.set_e(b);
                self.set_f(f);
                12
            }
            0x59 => { //OUT (C),E
                let bc = self.bc.as_u16();
                bus.do_out(bc, self.e());
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
                self.set_h(b);
                self.set_f(f);
                12
            }
            0x61 => { //OUT (C),H
                let bc = self.bc.as_u16();
                bus.do_out(bc, self.h());
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
                self.set_l(b);
                self.set_f(f);
                12
            }
            0x69 => { //OUT (C),L
                let bc = self.bc.as_u16();
                bus.do_out(bc, self.l());
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
                let sp = self.sp.as_u16();
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
