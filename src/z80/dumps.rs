use super::*;

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
                    println!(
                        "{:02x} {:02x} {:02x} {:02x} {:02x}",
                        a,
                        f,
                        r,
                        a2,
                        self.f() & 0xd7
                    );
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
                    println!(
                        "{:02x} {:02x} {:02x} {:02x} {:02x}",
                        a,
                        f,
                        r,
                        a2,
                        self.f() & 0xd7
                    );
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
                println!(
                    "{:02x} {:02x} {:02x} {:02x}",
                    a,
                    f,
                    self.a(),
                    self.f() & 0xd7
                );
            }
        }
    }
}
