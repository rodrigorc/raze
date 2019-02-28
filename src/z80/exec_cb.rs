use super::*;

impl Z80 {
    pub(super) fn exec_cb(&mut self, prefix: XYPrefix, bus: &mut impl Bus) -> u32 {
        let (addr, t) = self.hlx_addr(prefix, bus);
        let c = self.fetch(bus);
        if prefix == XYPrefix::None {
            self.inc_r();
        }
        match c {
            0x00 => { //RLC B
                let r = self.b();
                let r = self.rlc_flags(r);
                self.set_b(r);
                8
            }
            0x01 => { //RLC C
                let r = self.c();
                let r = self.rlc_flags(r);
                self.set_c(r);
                8
            }
            0x02 => { //RLC D
                let r = self.d();
                let r = self.rlc_flags(r);
                self.set_d(r);
                8
            }
            0x03 => { //RLC E
                let r = self.e();
                let r = self.rlc_flags(r);
                self.set_e(r);
                8
            }
            0x04 => { //RLC H
                let r = self.hx(prefix);
                let r = self.rlc_flags(r);
                self.set_hx(prefix, r);
                8
            }
            0x05 => { //RLC L
                let r = self.lx(prefix);
                let r = self.rlc_flags(r);
                self.set_lx(prefix, r);
                8
            }
            0x06 => { //RLC (HL)
                let r = bus.peek(addr);
                let r = self.rlc_flags(r);
                bus.poke(addr, r);
                t + 18
            }
            0x07 => { //RLC A
                let r = self.a();
                let r = self.rlc_flags(r);
                self.set_a(r);
                8
            }
            0x08 => { //RRC B
                let r = self.b();
                let r = self.rrc_flags(r);
                self.set_b(r);
                8
            }
            0x09 => { //RRC C
                let r = self.c();
                let r = self.rrc_flags(r);
                self.set_c(r);
                8
            }
            0x0a => { //RRC D
                let r = self.d();
                let r = self.rrc_flags(r);
                self.set_d(r);
                8
            }
            0x0b => { //RRC E
                let r = self.e();
                let r = self.rrc_flags(r);
                self.set_e(r);
                8
            }
            0x0c => { //RRC H
                let r = self.hx(prefix);
                let r = self.rrc_flags(r);
                self.set_hx(prefix, r);
                8
            }
            0x0d => { //RRC L
                let r = self.lx(prefix);
                let r = self.rrc_flags(r);
                self.set_lx(prefix, r);
                8
            }
            0x0e => { //RRC (HL)
                let r = bus.peek(addr);
                let r = self.rrc_flags(r);
                bus.poke(addr, r);
                t + 18
            }
            0x0f => { //RRC A
                let r = self.a();
                let r = self.rrc_flags(r);
                self.set_a(r);
                8
            }
            0x10 => { //RL B
                let r = self.b();
                let r = self.rl_flags(r);
                self.set_b(r);
                8
            }
            0x11 => { //RL C
                let r = self.c();
                let r = self.rl_flags(r);
                self.set_c(r);
                8
            }
            0x12 => { //RL D
                let r = self.d();
                let r = self.rl_flags(r);
                self.set_d(r);
                8
            }
            0x13 => { //RL E
                let r = self.e();
                let r = self.rl_flags(r);
                self.set_e(r);
                8
            }
            0x14 => { //RL H
                let r = self.hx(prefix);
                let r = self.rl_flags(r);
                self.set_hx(prefix, r);
                8
            }
            0x15 => { //RL L
                let r = self.lx(prefix);
                let r = self.rl_flags(r);
                self.set_lx(prefix, r);
                8
            }
            0x16 => { //RL (HL)
                let r = bus.peek(addr);
                let r = self.rl_flags(r);
                bus.poke(addr, r);
                t + 18
            }
            0x17 => { //RL A
                let r = self.a();
                let r = self.rl_flags(r);
                self.set_a(r);
                8
            }
            0x18 => { //RR B
                let r = self.b();
                let r = self.rr_flags(r);
                self.set_b(r);
                8
            }
            0x19 => { //RR C
                let r = self.c();
                let r = self.rr_flags(r);
                self.set_c(r);
                8
            }
            0x1a => { //RR D
                let r = self.d();
                let r = self.rr_flags(r);
                self.set_d(r);
                8
            }
            0x1b => { //RR E
                let r = self.e();
                let r = self.rr_flags(r);
                self.set_e(r);
                8
            }
            0x1c => { //RR H
                let r = self.hx(prefix);
                let r = self.rr_flags(r);
                self.set_hx(prefix, r);
                8
            }
            0x1d => { //RR L
                let r = self.lx(prefix);
                let r = self.rr_flags(r);
                self.set_lx(prefix, r);
                8
            }
            0x1e => { //RR (HL)
                let r = bus.peek(addr);
                let r = self.rr_flags(r);
                bus.poke(addr, r);
                t + 18
            }
            0x1f => { //RR A
                let r = self.a();
                let r = self.rr_flags(r);
                self.set_a(r);
                8
            }
            0x20 => { //SLA B
                let r = self.b();
                let r = self.sla_flags(r);
                self.set_b(r);
                8
            }
            0x21 => { //SLA C
                let r = self.c();
                let r = self.sla_flags(r);
                self.set_c(r);
                8
            }
            0x22 => { //SLA D
                let r = self.d();
                let r = self.sla_flags(r);
                self.set_d(r);
                8
            }
            0x23 => { //SLA E
                let r = self.e();
                let r = self.sla_flags(r);
                self.set_e(r);
                8
            }
            0x24 => { //SLA H
                let r = self.hx(prefix);
                let r = self.sla_flags(r);
                self.set_hx(prefix, r);
                8
            }
            0x25 => { //SLA L
                let r = self.lx(prefix);
                let r = self.sla_flags(r);
                self.set_lx(prefix, r);
                8
            }
            0x26 => { //SLA (HL)
                let r = bus.peek(addr);
                let r = self.sla_flags(r);
                bus.poke(addr, r);
                t + 18
            }
            0x27 => { //SLA A
                let r = self.a();
                let r = self.sla_flags(r);
                self.set_a(r);
                8
            }
            0x28 => { //SRA B
                let r = self.b();
                let r = self.sra_flags(r);
                self.set_b(r);
                8
            }
            0x29 => { //SRA C
                let r = self.c();
                let r = self.sra_flags(r);
                self.set_c(r);
                8
            }
            0x2a => { //SRA D
                let r = self.d();
                let r = self.sra_flags(r);
                self.set_d(r);
                8
            }
            0x2b => { //SRA E
                let r = self.e();
                let r = self.sra_flags(r);
                self.set_e(r);
                8
            }
            0x2c => { //SRA H
                let r = self.hx(prefix);
                let r = self.sra_flags(r);
                self.set_hx(prefix, r);
                8
            }
            0x2d => { //SRA L
                let r = self.lx(prefix);
                let r = self.sra_flags(r);
                self.set_lx(prefix, r);
                8
            }
            0x2e => { //SRA (HL)
                let r = bus.peek(addr);
                let r = self.sra_flags(r);
                bus.poke(addr, r);
                t + 18
            }
            0x2f => { //SRA A
                let r = self.a();
                let r = self.sra_flags(r);
                self.set_a(r);
                8
            }
            0x30 => { //SL1 B
                let r = self.b();
                let r = self.sl1_flags(r);
                self.set_b(r);
                8
            }
            0x31 => { //SL1 C
                let r = self.c();
                let r = self.sl1_flags(r);
                self.set_c(r);
                8
            }
            0x32 => { //SL1 D
                let r = self.d();
                let r = self.sl1_flags(r);
                self.set_d(r);
                8
            }
            0x33 => { //SL1 E
                let r = self.e();
                let r = self.sl1_flags(r);
                self.set_e(r);
                8
            }
            0x34 => { //SL1 H
                let r = self.hx(prefix);
                let r = self.sl1_flags(r);
                self.set_hx(prefix, r);
                8
            }
            0x35 => { //SL1 L
                let r = self.lx(prefix);
                let r = self.sl1_flags(r);
                self.set_lx(prefix, r);
                8
            }
            0x36 => { //SL1 (HL)
                let r = bus.peek(addr);
                let r = self.sl1_flags(r);
                bus.poke(addr, r);
                t + 18
            }
            0x37 => { //SL1 A
                let r = self.a();
                let r = self.sl1_flags(r);
                self.set_a(r);
                8
            }
            0x38 => { //SRL B
                let r = self.b();
                let r = self.srl_flags(r);
                self.set_b(r);
                8
            }
            0x39 => { //SRL C
                let r = self.c();
                let r = self.srl_flags(r);
                self.set_c(r);
                8
            }
            0x3a => { //SRL D
                let r = self.d();
                let r = self.srl_flags(r);
                self.set_d(r);
                8
            }
            0x3b => { //SRL E
                let r = self.e();
                let r = self.srl_flags(r);
                self.set_e(r);
                8
            }
            0x3c => { //SRL H
                let r = self.hx(prefix);
                let r = self.srl_flags(r);
                self.set_hx(prefix, r);
                8
            }
            0x3d => { //SRL L
                let r = self.lx(prefix);
                let r = self.srl_flags(r);
                self.set_lx(prefix, r);
                8
            }
            0x3e => { //SRL (HL)
                let r = bus.peek(addr);
                let r = self.srl_flags(r);
                bus.poke(addr, r);
                t + 18
            }
            0x3f => { //SRL A
                let r = self.a();
                let r = self.srl_flags(r);
                self.set_a(r);
                8
            }
            0x40 => { //BIT 0,B
                let r = self.b();
                self.bit_flags(r, 1 << 0);
                8
            }
            0x41 => { //BIT 0,C
                let r = self.c();
                self.bit_flags(r, 1 << 0);
                8
            }
            0x42 => { //BIT 0,D
                let r = self.d();
                self.bit_flags(r, 1 << 0);
                8
            }
            0x43 => { //BIT 0,E
                let r = self.e();
                self.bit_flags(r, 1 << 0);
                8
            }
            0x44 => { //BIT 0,H
                let r = self.hx(prefix);
                self.bit_flags(r, 1 << 0);
                8
            }
            0x45 => { //BIT 0,L
                let r = self.lx(prefix);
                self.bit_flags(r, 1 << 0);
                8
            }
            0x46 => { //BIT 0,(HL)
                let r = bus.peek(addr);
                self.bit_flags(r, 1 << 0);
                t + 15
            }
            0x47 => { //BIT 0,A
                let r = self.a();
                self.bit_flags(r, 1 << 0);
                8
            }
            0x48 => { //BIT 1,B
                let r = self.b();
                self.bit_flags(r, 1 << 1);
                8
            }
            0x49 => { //BIT 1,C
                let r = self.c();
                self.bit_flags(r, 1 << 1);
                8
            }
            0x4a => { //BIT 1,D
                let r = self.d();
                self.bit_flags(r, 1 << 1);
                8
            }
            0x4b => { //BIT 1,E
                let r = self.e();
                self.bit_flags(r, 1 << 1);
                8
            }
            0x4c => { //BIT 1,H
                let r = self.hx(prefix);
                self.bit_flags(r, 1 << 1);
                8
            }
            0x4d => { //BIT 1,L
                let r = self.lx(prefix);
                self.bit_flags(r, 1 << 1);
                8
            }
            0x4e => { //BIT 1,(HL)
                let r = bus.peek(addr);
                self.bit_flags(r, 1 << 1);
                t + 15
            }
            0x4f => { //BIT 1,A
                let r = self.a();
                self.bit_flags(r, 1 << 1);
                8
            }
            0x50 => { //BIT 2,B
                let r = self.b();
                self.bit_flags(r, 1 << 2);
                8
            }
            0x51 => { //BIT 2,C
                let r = self.c();
                self.bit_flags(r, 1 << 2);
                8
            }
            0x52 => { //BIT 2,D
                let r = self.d();
                self.bit_flags(r, 1 << 2);
                8
            }
            0x53 => { //BIT 2,E
                let r = self.e();
                self.bit_flags(r, 1 << 2);
                8
            }
            0x54 => { //BIT 2,H
                let r = self.hx(prefix);
                self.bit_flags(r, 1 << 2);
                8
            }
            0x55 => { //BIT 2,L
                let r = self.lx(prefix);
                self.bit_flags(r, 1 << 2);
                8
            }
            0x56 => { //BIT 2,(HL)
                let r = bus.peek(addr);
                self.bit_flags(r, 1 << 2);
                t + 15
            }
            0x57 => { //BIT 2,A
                let r = self.a();
                self.bit_flags(r, 1 << 2);
                8
            }
            0x58 => { //BIT 3,B
                let r = self.b();
                self.bit_flags(r, 1 << 3);
                8
            }
            0x59 => { //BIT 3,C
                let r = self.c();
                self.bit_flags(r, 1 << 3);
                8
            }
            0x5a => { //BIT 3,D
                let r = self.d();
                self.bit_flags(r, 1 << 3);
                8
            }
            0x5b => { //BIT 3,E
                let r = self.e();
                self.bit_flags(r, 1 << 3);
                8
            }
            0x5c => { //BIT 3,H
                let r = self.hx(prefix);
                self.bit_flags(r, 1 << 3);
                8
            }
            0x5d => { //BIT 3,L
                let r = self.lx(prefix);
                self.bit_flags(r, 1 << 3);
                8
            }
            0x5e => { //BIT 3,(HL)
                let r = bus.peek(addr);
                self.bit_flags(r, 1 << 3);
                t + 15
            }
            0x5f => { //BIT 3,A
                let r = self.a();
                self.bit_flags(r, 1 << 3);
                8
            }
            0x60 => { //BIT 4,B
                let r = self.b();
                self.bit_flags(r, 1 << 4);
                8
            }
            0x61 => { //BIT 4,C
                let r = self.c();
                self.bit_flags(r, 1 << 4);
                8
            }
            0x62 => { //BIT 4,D
                let r = self.d();
                self.bit_flags(r, 1 << 4);
                8
            }
            0x63 => { //BIT 4,E
                let r = self.e();
                self.bit_flags(r, 1 << 4);
                8
            }
            0x64 => { //BIT 4,H
                let r = self.hx(prefix);
                self.bit_flags(r, 1 << 4);
                8
            }
            0x65 => { //BIT 4,L
                let r = self.lx(prefix);
                self.bit_flags(r, 1 << 4);
                8
            }
            0x66 => { //BIT 4,(HL)
                let r = bus.peek(addr);
                self.bit_flags(r, 1 << 4);
                t + 15
            }
            0x67 => { //BIT 4,A
                let r = self.a();
                self.bit_flags(r, 1 << 4);
                8
            }
            0x68 => { //BIT 5,B
                let r = self.b();
                self.bit_flags(r, 1 << 5);
                8
            }
            0x69 => { //BIT 5,C
                let r = self.c();
                self.bit_flags(r, 1 << 5);
                8
            }
            0x6a => { //BIT 5,D
                let r = self.d();
                self.bit_flags(r, 1 << 5);
                8
            }
            0x6b => { //BIT 5,E
                let r = self.e();
                self.bit_flags(r, 1 << 5);
                8
            }
            0x6c => { //BIT 5,H
                let r = self.hx(prefix);
                self.bit_flags(r, 1 << 5);
                8
            }
            0x6d => { //BIT 5,L
                let r = self.lx(prefix);
                self.bit_flags(r, 1 << 5);
                8
            }
            0x6e => { //BIT 5,(HL)
                let r = bus.peek(addr);
                self.bit_flags(r, 1 << 5);
                t + 15
            }
            0x6f => { //BIT 5,A
                let r = self.a();
                self.bit_flags(r, 1 << 5);
                8
            }
            0x70 => { //BIT 6,B
                let r = self.b();
                self.bit_flags(r, 1 << 6);
                8
            }
            0x71 => { //BIT 6,C
                let r = self.c();
                self.bit_flags(r, 1 << 6);
                8
            }
            0x72 => { //BIT 6,D
                let r = self.d();
                self.bit_flags(r, 1 << 6);
                8
            }
            0x73 => { //BIT 6,E
                let r = self.e();
                self.bit_flags(r, 1 << 6);
                8
            }
            0x74 => { //BIT 6,H
                let r = self.hx(prefix);
                self.bit_flags(r, 1 << 6);
                8
            }
            0x75 => { //BIT 6,L
                let r = self.lx(prefix);
                self.bit_flags(r, 1 << 6);
                8
            }
            0x76 => { //BIT 6,(HL)
                let r = bus.peek(addr);
                self.bit_flags(r, 1 << 6);
                t + 15
            }
            0x77 => { //BIT 6,A
                let r = self.a();
                self.bit_flags(r, 1 << 6);
                8
            }
            0x78 => { //BIT 7,B
                let r = self.b();
                self.bit_flags(r, 1 << 7);
                8
            }
            0x79 => { //BIT 7,C
                let r = self.c();
                self.bit_flags(r, 1 << 7);
                8
            }
            0x7a => { //BIT 7,D
                let r = self.d();
                self.bit_flags(r, 1 << 7);
                8
            }
            0x7b => { //BIT 7,E
                let r = self.e();
                self.bit_flags(r, 1 << 7);
                8
            }
            0x7c => { //BIT 7,H
                let r = self.hx(prefix);
                self.bit_flags(r, 1 << 7);
                8
            }
            0x7d => { //BIT 7,L
                let r = self.lx(prefix);
                self.bit_flags(r, 1 << 7);
                8
            }
            0x7e => { //BIT 7,(HL)
                let r = bus.peek(addr);
                self.bit_flags(r, 1 << 7);
                t + 15
            }
            0x7f => { //BIT 7,A
                let r = self.a();
                self.bit_flags(r, 1 << 7);
                8
            }
            0x80 => { //RES 0,B
                let b = self.b();
                let b = set_flag8(b, 1 << 0, false);
                self.set_b(b);
                8
            }
            0x81 => { //RES 0,C
                let b = self.c();
                let b = set_flag8(b, 1 << 0, false);
                self.set_c(b);
                8
            }
            0x82 => { //RES 0,D
                let b = self.d();
                let b = set_flag8(b, 1 << 0, false);
                self.set_d(b);
                8
            }
            0x83 => { //RES 0,E
                let b = self.e();
                let b = set_flag8(b, 1 << 0, false);
                self.set_e(b);
                8
            }
            0x84 => { //RES 0,H
                let b = self.hx(prefix);
                let b = set_flag8(b, 1 << 0, false);
                self.set_hx(prefix, b);
                8
            }
            0x85 => { //RES 0,L
                let b = self.lx(prefix);
                let b = set_flag8(b, 1 << 0, false);
                self.set_lx(prefix, b);
                8
            }
            0x86 => { //RES 0,(HL)
                let b = bus.peek(addr);
                let b = set_flag8(b, 1 << 0, false);
                bus.poke(addr, b);
                t + 18
            }
            0x87 => { //RES 0,A
                let b = self.a();
                let b = set_flag8(b, 1 << 0, false);
                self.set_a(b);
                8
            }
            0x88 => { //RES 1,B
                let b = self.b();
                let b = set_flag8(b, 1 << 1, false);
                self.set_b(b);
                8
            }
            0x89 => { //RES 1,C
                let b = self.c();
                let b = set_flag8(b, 1 << 1, false);
                self.set_c(b);
                8
            }
            0x8a => { //RES 1,D
                let b = self.d();
                let b = set_flag8(b, 1 << 1, false);
                self.set_d(b);
                8
            }
            0x8b => { //RES 1,E
                let b = self.e();
                let b = set_flag8(b, 1 << 1, false);
                self.set_e(b);
                8
            }
            0x8c => { //RES 1,H
                let b = self.hx(prefix);
                let b = set_flag8(b, 1 << 1, false);
                self.set_hx(prefix, b);
                8
            }
            0x8d => { //RES 1,L
                let b = self.lx(prefix);
                let b = set_flag8(b, 1 << 1, false);
                self.set_lx(prefix, b);
                8
            }
            0x8e => { //RES 1,(HL)
                let b = bus.peek(addr);
                let b = set_flag8(b, 1 << 1, false);
                bus.poke(addr, b);
                t + 18
            }
            0x8f => { //RES 1,A
                let b = self.a();
                let b = set_flag8(b, 1 << 1, false);
                self.set_a(b);
                8
            }
            0x90 => { //RES 2,B
                let b = self.b();
                let b = set_flag8(b, 1 << 2, false);
                self.set_b(b);
                8
            }
            0x91 => { //RES 2,C
                let b = self.c();
                let b = set_flag8(b, 1 << 2, false);
                self.set_c(b);
                8
            }
            0x92 => { //RES 2,D
                let b = self.d();
                let b = set_flag8(b, 1 << 2, false);
                self.set_d(b);
                8
            }
            0x93 => { //RES 2,E
                let b = self.e();
                let b = set_flag8(b, 1 << 2, false);
                self.set_e(b);
                8
            }
            0x94 => { //RES 2,H
                let b = self.hx(prefix);
                let b = set_flag8(b, 1 << 2, false);
                self.set_hx(prefix, b);
                8
            }
            0x95 => { //RES 2,L
                let b = self.lx(prefix);
                let b = set_flag8(b, 1 << 2, false);
                self.set_lx(prefix, b);
                8
            }
            0x96 => { //RES 2,(HL)
                let b = bus.peek(addr);
                let b = set_flag8(b, 1 << 2, false);
                bus.poke(addr, b);
                t + 18
            }
            0x97 => { //RES 2,A
                let b = self.a();
                let b = set_flag8(b, 1 << 2, false);
                self.set_a(b);
                8
            }
            0x98 => { //RES 3,B
                let b = self.b();
                let b = set_flag8(b, 1 << 3, false);
                self.set_b(b);
                8
            }
            0x99 => { //RES 3,C
                let b = self.c();
                let b = set_flag8(b, 1 << 3, false);
                self.set_c(b);
                8
            }
            0x9a => { //RES 3,D
                let b = self.d();
                let b = set_flag8(b, 1 << 3, false);
                self.set_d(b);
                8
            }
            0x9b => { //RES 3,E
                let b = self.e();
                let b = set_flag8(b, 1 << 3, false);
                self.set_e(b);
                8
            }
            0x9c => { //RES 3,H
                let b = self.hx(prefix);
                let b = set_flag8(b, 1 << 3, false);
                self.set_hx(prefix, b);
                8
            }
            0x9d => { //RES 3,L
                let b = self.lx(prefix);
                let b = set_flag8(b, 1 << 3, false);
                self.set_lx(prefix, b);
                8
            }
            0x9e => { //RES 3,(HL)
                let b = bus.peek(addr);
                let b = set_flag8(b, 1 << 3, false);
                bus.poke(addr, b);
                t + 18
            }
            0x9f => { //RES 3,A
                let b = self.a();
                let b = set_flag8(b, 1 << 3, false);
                self.set_a(b);
                8
            }
            0xa0 => { //RES 4,B
                let b = self.b();
                let b = set_flag8(b, 1 << 4, false);
                self.set_b(b);
                8
            }
            0xa1 => { //RES 4,C
                let b = self.c();
                let b = set_flag8(b, 1 << 4, false);
                self.set_c(b);
                8
            }
            0xa2 => { //RES 4,D
                let b = self.d();
                let b = set_flag8(b, 1 << 4, false);
                self.set_d(b);
                8
            }
            0xa3 => { //RES 4,E
                let b = self.e();
                let b = set_flag8(b, 1 << 4, false);
                self.set_e(b);
                8
            }
            0xa4 => { //RES 4,H
                let b = self.hx(prefix);
                let b = set_flag8(b, 1 << 4, false);
                self.set_hx(prefix, b);
                8
            }
            0xa5 => { //RES 4,L
                let b = self.lx(prefix);
                let b = set_flag8(b, 1 << 4, false);
                self.set_lx(prefix, b);
                8
            }
            0xa6 => { //RES 4,(HL)
                let b = bus.peek(addr);
                let b = set_flag8(b, 1 << 4, false);
                bus.poke(addr, b);
                t + 18
            }
            0xa7 => { //RES 4,A
                let b = self.a();
                let b = set_flag8(b, 1 << 4, false);
                self.set_a(b);
                8
            }
            0xa8 => { //RES 5,B
                let b = self.b();
                let b = set_flag8(b, 1 << 5, false);
                self.set_b(b);
                8
            }
            0xa9 => { //RES 5,C
                let b = self.c();
                let b = set_flag8(b, 1 << 5, false);
                self.set_c(b);
                8
            }
            0xaa => { //RES 5,D
                let b = self.d();
                let b = set_flag8(b, 1 << 5, false);
                self.set_d(b);
                8
            }
            0xab => { //RES 5,E
                let b = self.e();
                let b = set_flag8(b, 1 << 5, false);
                self.set_e(b);
                8
            }
            0xac => { //RES 5,H
                let b = self.hx(prefix);
                let b = set_flag8(b, 1 << 5, false);
                self.set_hx(prefix, b);
                8
            }
            0xad => { //RES 5,L
                let b = self.lx(prefix);
                let b = set_flag8(b, 1 << 5, false);
                self.set_lx(prefix, b);
                8
            }
            0xae => { //RES 5,(HL)
                let b = bus.peek(addr);
                let b = set_flag8(b, 1 << 5, false);
                bus.poke(addr, b);
                t + 18
            }
            0xaf => { //RES 5,A
                let b = self.a();
                let b = set_flag8(b, 1 << 5, false);
                self.set_a(b);
                8
            }
            0xb0 => { //RES 6,B
                let b = self.b();
                let b = set_flag8(b, 1 << 6, false);
                self.set_b(b);
                8
            }
            0xb1 => { //RES 6,C
                let b = self.c();
                let b = set_flag8(b, 1 << 6, false);
                self.set_c(b);
                8
            }
            0xb2 => { //RES 6,D
                let b = self.d();
                let b = set_flag8(b, 1 << 6, false);
                self.set_d(b);
                8
            }
            0xb3 => { //RES 6,E
                let b = self.e();
                let b = set_flag8(b, 1 << 6, false);
                self.set_e(b);
                8
            }
            0xb4 => { //RES 6,H
                let b = self.hx(prefix);
                let b = set_flag8(b, 1 << 6, false);
                self.set_hx(prefix, b);
                8
            }
            0xb5 => { //RES 6,L
                let b = self.lx(prefix);
                let b = set_flag8(b, 1 << 6, false);
                self.set_lx(prefix, b);
                8
            }
            0xb6 => { //RES 6,(HL)
                let b = bus.peek(addr);
                let b = set_flag8(b, 1 << 6, false);
                bus.poke(addr, b);
                t + 18
            }
            0xb7 => { //RES 6,A
                let b = self.a();
                let b = set_flag8(b, 1 << 6, false);
                self.set_a(b);
                8
            }
            0xb8 => { //RES 7,B
                let b = self.b();
                let b = set_flag8(b, 1 << 7, false);
                self.set_b(b);
                8
            }
            0xb9 => { //RES 7,C
                let b = self.c();
                let b = set_flag8(b, 1 << 7, false);
                self.set_c(b);
                8
            }
            0xba => { //RES 7,D
                let b = self.d();
                let b = set_flag8(b, 1 << 7, false);
                self.set_d(b);
                8
            }
            0xbb => { //RES 7,E
                let b = self.e();
                let b = set_flag8(b, 1 << 7, false);
                self.set_e(b);
                8
            }
            0xbc => { //RES 7,H
                let b = self.hx(prefix);
                let b = set_flag8(b, 1 << 7, false);
                self.set_hx(prefix, b);
                8
            }
            0xbd => { //RES 7,L
                let b = self.lx(prefix);
                let b = set_flag8(b, 1 << 7, false);
                self.set_lx(prefix, b);
                8
            }
            0xbe => { //RES 7,(HL)
                let b = bus.peek(addr);
                let b = set_flag8(b, 1 << 7, false);
                bus.poke(addr, b);
                t + 18
            }
            0xbf => { //RES 7,A
                let b = self.a();
                let b = set_flag8(b, 1 << 7, false);
                self.set_a(b);
                8
            }
            0xc0 => { //SET 0,B
                let b = self.b();
                let b = set_flag8(b, 1 << 0, true);
                self.set_b(b);
                8
            }
            0xc1 => { //SET 0,C
                let b = self.c();
                let b = set_flag8(b, 1 << 0, true);
                self.set_c(b);
                8
            }
            0xc2 => { //SET 0,D
                let b = self.d();
                let b = set_flag8(b, 1 << 0, true);
                self.set_d(b);
                8
            }
            0xc3 => { //SET 0,E
                let b = self.e();
                let b = set_flag8(b, 1 << 0, true);
                self.set_e(b);
                8
            }
            0xc4 => { //SET 0,H
                let b = self.hx(prefix);
                let b = set_flag8(b, 1 << 0, true);
                self.set_hx(prefix, b);
                8
            }
            0xc5 => { //SET 0,L
                let b = self.lx(prefix);
                let b = set_flag8(b, 1 << 0, true);
                self.set_lx(prefix, b);
                8
            }
            0xc6 => { //SET 0,(HL)
                let b = bus.peek(addr);
                let b = set_flag8(b, 1 << 0, true);
                bus.poke(addr, b);
                t + 18
            }
            0xc7 => { //SET 0,A
                let b = self.a();
                let b = set_flag8(b, 1 << 0, true);
                self.set_a(b);
                8
            }
            0xc8 => { //SET 1,B
                let b = self.b();
                let b = set_flag8(b, 1 << 1, true);
                self.set_b(b);
                8
            }
            0xc9 => { //SET 1,C
                let b = self.c();
                let b = set_flag8(b, 1 << 1, true);
                self.set_c(b);
                8
            }
            0xca => { //SET 1,D
                let b = self.d();
                let b = set_flag8(b, 1 << 1, true);
                self.set_d(b);
                8
            }
            0xcb => { //SET 1,E
                let b = self.e();
                let b = set_flag8(b, 1 << 1, true);
                self.set_e(b);
                8
            }
            0xcc => { //SET 1,H
                let b = self.hx(prefix);
                let b = set_flag8(b, 1 << 1, true);
                self.set_hx(prefix, b);
                8
            }
            0xcd => { //SET 1,L
                let b = self.lx(prefix);
                let b = set_flag8(b, 1 << 1, true);
                self.set_lx(prefix, b);
                8
            }
            0xce => { //SET 1,(HL)
                let b = bus.peek(addr);
                let b = set_flag8(b, 1 << 1, true);
                bus.poke(addr, b);
                t + 18
            }
            0xcf => { //SET 1,A
                let b = self.a();
                let b = set_flag8(b, 1 << 1, true);
                self.set_a(b);
                8
            }
            0xd0 => { //SET 2,B
                let b = self.b();
                let b = set_flag8(b, 1 << 2, true);
                self.set_b(b);
                8
            }
            0xd1 => { //SET 2,C
                let b = self.c();
                let b = set_flag8(b, 1 << 2, true);
                self.set_c(b);
                8
            }
            0xd2 => { //SET 2,D
                let b = self.d();
                let b = set_flag8(b, 1 << 2, true);
                self.set_d(b);
                8
            }
            0xd3 => { //SET 2,E
                let b = self.e();
                let b = set_flag8(b, 1 << 2, true);
                self.set_e(b);
                8
            }
            0xd4 => { //SET 2,H
                let b = self.hx(prefix);
                let b = set_flag8(b, 1 << 2, true);
                self.set_hx(prefix, b);
                8
            }
            0xd5 => { //SET 2,L
                let b = self.lx(prefix);
                let b = set_flag8(b, 1 << 2, true);
                self.set_lx(prefix, b);
                8
            }
            0xd6 => { //SET 2,(HL)
                let b = bus.peek(addr);
                let b = set_flag8(b, 1 << 2, true);
                bus.poke(addr, b);
                t + 18
            }
            0xd7 => { //SET 2,A
                let b = self.a();
                let b = set_flag8(b, 1 << 2, true);
                self.set_a(b);
                8
            }
            0xd8 => { //SET 3,B
                let b = self.b();
                let b = set_flag8(b, 1 << 3, true);
                self.set_b(b);
                8
            }
            0xd9 => { //SET 3,C
                let b = self.c();
                let b = set_flag8(b, 1 << 3, true);
                self.set_c(b);
                8
            }
            0xda => { //SET 3,D
                let b = self.d();
                let b = set_flag8(b, 1 << 3, true);
                self.set_d(b);
                8
            }
            0xdb => { //SET 3,E
                let b = self.e();
                let b = set_flag8(b, 1 << 3, true);
                self.set_e(b);
                8
            }
            0xdc => { //SET 3,H
                let b = self.hx(prefix);
                let b = set_flag8(b, 1 << 3, true);
                self.set_hx(prefix, b);
                8
            }
            0xdd => { //SET 3,L
                let b = self.lx(prefix);
                let b = set_flag8(b, 1 << 3, true);
                self.set_lx(prefix, b);
                8
            }
            0xde => { //SET 3,(HL)
                let b = bus.peek(addr);
                let b = set_flag8(b, 1 << 3, true);
                bus.poke(addr, b);
                t + 18
            }
            0xdf => { //SET 3,A
                let b = self.a();
                let b = set_flag8(b, 1 << 3, true);
                self.set_a(b);
                8
            }
            0xe0 => { //SET 4,B
                let b = self.b();
                let b = set_flag8(b, 1 << 4, true);
                self.set_b(b);
                8
            }
            0xe1 => { //SET 4,C
                let b = self.c();
                let b = set_flag8(b, 1 << 4, true);
                self.set_c(b);
                8
            }
            0xe2 => { //SET 4,D
                let b = self.d();
                let b = set_flag8(b, 1 << 4, true);
                self.set_d(b);
                8
            }
            0xe3 => { //SET 4,E
                let b = self.e();
                let b = set_flag8(b, 1 << 4, true);
                self.set_e(b);
                8
            }
            0xe4 => { //SET 4,H
                let b = self.hx(prefix);
                let b = set_flag8(b, 1 << 4, true);
                self.set_hx(prefix, b);
                8
            }
            0xe5 => { //SET 4,L
                let b = self.lx(prefix);
                let b = set_flag8(b, 1 << 4, true);
                self.set_lx(prefix, b);
                8
            }
            0xe6 => { //SET 4,(HL)
                let b = bus.peek(addr);
                let b = set_flag8(b, 1 << 4, true);
                bus.poke(addr, b);
                t + 18
            }
            0xe7 => { //SET 4,A
                let b = self.a();
                let b = set_flag8(b, 1 << 4, true);
                self.set_a(b);
                8
            }
            0xe8 => { //SET 5,B
                let b = self.b();
                let b = set_flag8(b, 1 << 5, true);
                self.set_b(b);
                8
            }
            0xe9 => { //SET 5,C
                let b = self.c();
                let b = set_flag8(b, 1 << 5, true);
                self.set_c(b);
                8
            }
            0xea => { //SET 5,D
                let b = self.d();
                let b = set_flag8(b, 1 << 5, true);
                self.set_d(b);
                8
            }
            0xeb => { //SET 5,E
                let b = self.e();
                let b = set_flag8(b, 1 << 5, true);
                self.set_e(b);
                8
            }
            0xec => { //SET 5,H
                let b = self.hx(prefix);
                let b = set_flag8(b, 1 << 5, true);
                self.set_hx(prefix, b);
                8
            }
            0xed => { //SET 5,L
                let b = self.lx(prefix);
                let b = set_flag8(b, 1 << 5, true);
                self.set_lx(prefix, b);
                8
            }
            0xee => { //SET 5,(HL)
                let b = bus.peek(addr);
                let b = set_flag8(b, 1 << 5, true);
                bus.poke(addr, b);
                t + 18
            }
            0xef => { //SET 5,A
                let b = self.a();
                let b = set_flag8(b, 1 << 5, true);
                self.set_a(b);
                8
            }
            0xf0 => { //SET 6,B
                let b = self.b();
                let b = set_flag8(b, 1 << 6, true);
                self.set_b(b);
                8
            }
            0xf1 => { //SET 6,C
                let b = self.c();
                let b = set_flag8(b, 1 << 6, true);
                self.set_c(b);
                8
            }
            0xf2 => { //SET 6,D
                let b = self.d();
                let b = set_flag8(b, 1 << 6, true);
                self.set_d(b);
                8
            }
            0xf3 => { //SET 6,E
                let b = self.e();
                let b = set_flag8(b, 1 << 6, true);
                self.set_e(b);
                8
            }
            0xf4 => { //SET 6,H
                let b = self.hx(prefix);
                let b = set_flag8(b, 1 << 6, true);
                self.set_hx(prefix, b);
                8
            }
            0xf5 => { //SET 6,L
                let b = self.lx(prefix);
                let b = set_flag8(b, 1 << 6, true);
                self.set_lx(prefix, b);
                8
            }
            0xf6 => { //SET 6,(HL)
                let b = bus.peek(addr);
                let b = set_flag8(b, 1 << 6, true);
                bus.poke(addr, b);
                t + 18
            }
            0xf7 => { //SET 6,A
                let b = self.a();
                let b = set_flag8(b, 1 << 6, true);
                self.set_a(b);
                8
            }
            0xf8 => { //SET 7,B
                let b = self.b();
                let b = set_flag8(b, 1 << 7, true);
                self.set_b(b);
                8
            }
            0xf9 => { //SET 7,C
                let b = self.c();
                let b = set_flag8(b, 1 << 7, true);
                self.set_c(b);
                8
            }
            0xfa => { //SET 7,D
                let b = self.d();
                let b = set_flag8(b, 1 << 7, true);
                self.set_d(b);
                8
            }
            0xfb => { //SET 7,E
                let b = self.e();
                let b = set_flag8(b, 1 << 7, true);
                self.set_e(b);
                8
            }
            0xfc => { //SET 7,H
                let b = self.hx(prefix);
                let b = set_flag8(b, 1 << 7, true);
                self.set_hx(prefix, b);
                8
            }
            0xfd => { //SET 7,L
                let b = self.lx(prefix);
                let b = set_flag8(b, 1 << 7, true);
                self.set_lx(prefix, b);
                8
            }
            0xfe => { //SET 7,(HL)
                let b = bus.peek(addr);
                let b = set_flag8(b, 1 << 7, true);
                bus.poke(addr, b);
                t + 18
            }
            0xff => { //SET 7,A
                let b = self.a();
                let b = set_flag8(b, 1 << 7, true);
                self.set_a(b);
                8
            }
        }
    }
    fn rlc_flags(&mut self, b: u8) -> u8 {
        let f = self.f();
        let b7 = flag8(b, 0x80);
        let b = b.rotate_left(1);
        let f = set_flag8(f, FLAG_C, b7);
        let f = set_flag8(f, FLAG_N, false);
        let f = set_flag8(f, FLAG_H, false);
        let f = set_flag_szp(f, b);
        self.set_f(f);
        b
    }
    fn rrc_flags(&mut self, b: u8) -> u8 {
        let f = self.f();
        let b0 = flag8(b, 0x01);
        let b = b.rotate_right(1);
        let f = set_flag8(f, FLAG_C, b0);
        let f = set_flag8(f, FLAG_N, false);
        let f = set_flag8(f, FLAG_H, false);
        let f = set_flag_szp(f, b);
        self.set_f(f);
        b
    }
    fn rl_flags(&mut self, b: u8) -> u8 {
        let f = self.f();
        let b7 = flag8(b, 0x80);
        let c = flag8(f, FLAG_C);
        let b = b << 1;
        let b = set_flag8(b, 1, c);
        let f = set_flag8(f, FLAG_C, b7);
        let f = set_flag8(f, FLAG_N, false);
        let f = set_flag8(f, FLAG_H, false);
        let f = set_flag_szp(f, b);
        self.set_f(f);
        b
    }
    fn rr_flags(&mut self, b: u8) -> u8 {
        let f = self.f();
        let b0 = flag8(b, 0x01);
        let c = flag8(f, FLAG_C);
        let b = b >> 1;
        let b = set_flag8(b, 0x80, c);
        let f = set_flag8(f, FLAG_C, b0);
        let f = set_flag8(f, FLAG_N, false);
        let f = set_flag8(f, FLAG_H, false);
        let f = set_flag_szp(f, b);
        self.set_f(f);
        b
    }
    fn sla_flags(&mut self, b: u8) -> u8 {
        let f = self.f();
        let b7 = flag8(b, 0x80);
        let b = b << 1;
        let f = set_flag8(f, FLAG_C, b7);
        let f = set_flag8(f, FLAG_N, false);
        let f = set_flag8(f, FLAG_H, false);
        let f = set_flag_szp(f, b);
        self.set_f(f);
        b
    }
    fn sra_flags(&mut self, b: u8) -> u8 {
        let f = self.f();
        let b0 = flag8(b, 0x01);
        let b = ((b as i8) >> 1) as u8;
        let f = set_flag8(f, FLAG_C, b0);
        let f = set_flag8(f, FLAG_N, false);
        let f = set_flag8(f, FLAG_H, false);
        let f = set_flag_szp(f, b);
        self.set_f(f);
        b
    }
    fn sl1_flags(&mut self, b: u8) -> u8 {
        let f = self.f();
        let b7 = flag8(b, 0x80);
        let b = (b << 1) | 1;
        let f = set_flag8(f, FLAG_C, b7);
        let f = set_flag8(f, FLAG_N, false);
        let f = set_flag8(f, FLAG_H, false);
        let f = set_flag_szp(f, b);
        self.set_f(f);
        b
    }
    fn srl_flags(&mut self, b: u8) -> u8 {
        let f = self.f();
        let b0 = flag8(b, 0x01);
        let b = b >> 1;
        let f = set_flag8(f, FLAG_C, b0);
        let f = set_flag8(f, FLAG_N, false);
        let f = set_flag8(f, FLAG_H, false);
        let f = set_flag_szp(f, b);
        self.set_f(f);
        b
    }
    fn bit_flags(&mut self, b: u8, m: u8) {
        let r = b & m;
        let f = self.f();
        let f = set_flag8(f, FLAG_H, true);
        let f = set_flag8(f, FLAG_N, false);
        let f = set_flag_szp(f, r);
        self.set_f(f);
    }
}
