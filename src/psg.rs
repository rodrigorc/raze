//Emulation of the AY-3-8910 programmable sound generator

struct FreqGen {
    divisor: u32,
    phase: u32,
}

impl FreqGen {
    fn new() -> FreqGen {
        FreqGen {
            divisor: 32,
            phase: 0,
        }
    }
    fn set_freq(&mut self, freq: u16) {
        self.divisor = 32 * u32::from(freq);
    }
    fn next_sample(&mut self, t: u32) -> bool {
        self.phase += t;
        while self.phase > self.divisor {
            self.phase -= self.divisor;
        }
        self.phase < self.divisor / 2
    }
}

struct NoiseGen {
    divisor: u32,
    shift: u32,
    level: bool,
    phase: u32,
}

impl NoiseGen {
    fn new() -> NoiseGen {
        NoiseGen {
            divisor: 32,
            shift: 1,
            level: false,
            phase: 0,
        }
    }
    fn set_freq(&mut self, freq: u8) {
        self.divisor = 32 * u32::from(freq);
    }
    fn next_sample(&mut self, t: u32) -> bool {
        self.phase += t;
        if self.phase >= self.divisor {
            self.phase = 0;
            let bit0 = (self.shift & 1) != 0;
            let bit3 = (self.shift & 8) != 0;
            self.level = bit0;
            if bit0 ^ bit3 {
                self.shift ^= 0x20000;
            }
            self.shift >>= 1;
        }
        self.level
    }
}

enum EnvBlock {
    High,
    Low,
    Raise,
    Lower,
}

enum EnvShape {
    LowerLow,
    RaiseLow,
    LowerLoop,
    LowerRaiseLoop,
    LowerHigh,
    RaiseLoop,
    RaiseHigh,
    RaiseLowerLoop,
}

struct Envelope {
    divisor: u32,
    phase: u32,
    shape: EnvShape,
    step: u8,
    block: EnvBlock,
}


impl Envelope {
    fn new() -> Envelope {
        Envelope {
            divisor: 32,
            phase: 0,
            step: 0,
            shape: EnvShape::LowerLow,
            block: EnvBlock::Low,
        }
    }
    fn set_freq_shape(&mut self, freq: u16, shape: u8) {
        use self::{EnvShape::*, EnvBlock::*};
        self.divisor = 32 * u32::from(freq);
        self.phase = 0;
        self.step = 0;
        let (shape, block) = match shape {
            0x00 | 0x01 | 0x02 | 0x03 | 0x09 => (LowerLow, Lower),
            0x04 | 0x05 | 0x06 | 0x07 | 0x0f => (RaiseLow, Raise),
            0x08 => (LowerLoop, Lower),
            0x0a => (LowerRaiseLoop, Lower),
            0x0b => (LowerHigh, Lower),
            0x0c => (RaiseLoop, Raise),
            0x0d => (RaiseHigh, Raise),
            0x0e => (RaiseLowerLoop, Raise),
            // shape is only 4 bits
            0x10 ..= 0xff => unreachable!(),
        };
        self.shape = shape;
        self.block = block;
    }
    fn next_sample(&mut self, t: u32) -> u8 {
        use self::{EnvShape::*, EnvBlock::*};
        self.phase += t;
        while self.phase > self.divisor {
            self.phase -= self.divisor;
            self.step += 1;
            if self.step == 16 {
                self.step = 0;

                self.block = match self.shape {
                    LowerLow | RaiseLow => Low,
                    LowerHigh | RaiseHigh => High,
                    LowerLoop => Lower,
                    RaiseLoop => Raise,
                    LowerRaiseLoop | RaiseLowerLoop => match self.block {
                        Lower => Raise,
                        Raise => Lower,
                        _ => unreachable!(),
                    }
                };
            }
        }
        match self.block {
            Low => 0,
            High => 15,
            Raise => self.step,
            Lower => 15 - self.step,
        }
    }
}

// Some fancy programs, such as demos, try do detect if this is an original AY-3-8910 or
// some clone like YM2149. They seem to regard the original as superior, so we try to
// pose as such.
// The trick is that some registers do not use the full 8-bits. In those the original chip
// only stores the necessary bits while the clones keep them all. The program will
// write a value such as 0xff and then read it back: if it gets the whole value it is
// a clone.
// This array contains the bitmask for each register:
static REG_MASK: [u8; 16] = [
// Tone A,B,C freq. are 12 bits each (8+4)
    /*0x00, 0x01*/ 0xff, 0x0f,
    /*0x02, 0x03*/ 0xff, 0x0f,
    /*0x04, 0x05*/ 0xff, 0x0f,
// Noise freq. has 5 bits:
    /*0x06      */ 0x1f,
// Flags: 8 bits
    /*0x07      */ 0xff,
// Channel A,B,C volume: (1+4) bits each
    /*0x08      */ 0x1f,
    /*0x09      */ 0x1f,
    /*0x0a      */ 0x1f,
// Envelope freq: 16 bits
    /*0x0b, 0x0c*/ 0xff, 0xff,
// Envelope shape: 4 bits
    /*0x0d      */ 0x0f,
// IO ports A,B: 8 bits each
    /*0x0e      */ 0xff,
    /*0x0f      */ 0xff,
];

/// The Programmable Sound Generator: AY-3-8910
pub struct Psg {
    /// The selected register, that will be read/written next
    reg_sel: u8,
    /// There are 16 byte-sized registers
    reg: [u8; 16],
    /// There are 3 frequency generators, this is the FG-A.
    freq_a: FreqGen,
    /// The second frequency generator FG-B
    freq_b: FreqGen,
    /// The third frequency generator FG-C
    freq_c: FreqGen,
    /// There is only one noise generator, shared by all the FG-*
    noise: NoiseGen,
    /// The envelope setup
    envelope: Envelope,
}

impl Psg {
    pub fn new() -> Psg {
        Psg {
            reg_sel: 0,
            reg: [0; 16],
            freq_a: FreqGen::new(),
            freq_b: FreqGen::new(),
            freq_c: FreqGen::new(),
            noise: NoiseGen::new(),
            envelope: Envelope::new(),
        }
    }
    pub fn load_snapshot(data: &[u8]) -> Psg {
        // Go through write_reg() to rebuild the state of the inner generators.
        // Generator phases are not restored, but that should be unnoticeable.
        let mut psg = Self::new();
        for (r, &v) in data[1..17].iter().enumerate() {
            psg.reg_sel = r as u8;
            psg.write_reg(v);
        }
        // Do not trust data[0] is in range
        psg.select_reg(data[0]);
        psg
    }
    pub fn snapshot(&self, data: &mut [u8]) {
        data[0] = self.reg_sel;
        data[1..17].copy_from_slice(&self.reg);
    }
    /// Changes the selected register
    pub fn select_reg(&mut self, reg: u8) {
        if let 0..=0x0f = reg {
            self.reg_sel = reg;
        }
        // Selecting an invalid register has no effect
    }
    /// Reads the selected register, it has no side effects
    pub fn read_reg(&self) -> u8 {
        self.reg[usize::from(self.reg_sel)]
        //log::info!("PSG read {:02x} -> {:02x}", self.reg_sel, r);
    }
    /// Writes the selected register
    pub fn write_reg(&mut self, x: u8) {
        let reg_sel =  usize::from(self.reg_sel);
        self.reg[reg_sel] = x & REG_MASK[reg_sel];
        //log::info!("PSG write {:02x} <- {:02x}", reg_sel, x);
        // Writing to most registers has a side effect
        match reg_sel {
            //Regs 0x00-0x01 set up the frequency for FG-A
            0x00 | 0x01 => {
                let freq = Self::freq(self.reg[0x00], self.reg[0x01]);
                self.freq_a.set_freq(freq);
                //log::info!("Tone A: {}", freq);
            }
            //Regs 0x02-0x03 set up the frequency for FG-B
            0x02 | 0x03 => {
                let freq = Self::freq(self.reg[0x02], self.reg[0x03]);
                self.freq_b.set_freq(freq);
                //log::info!("Tone B: {}", freq);
            }
            //Regs 0x04-0x05 set up the frequency for FG-C
            0x04 | 0x05 => {
            let freq = Self::freq(self.reg[0x04], self.reg[0x05]);
                self.freq_c.set_freq(freq);
                //log::info!("Tone C: {}", freq);
            }
            //Reg 0x06 is the noise frequency
            0x06 => {
                let noise = self.reg[0x06];
                self.noise.set_freq(if noise == 0 { 1 } else { noise });
            }
            //Regs 0x07-0x0a: are used directly in next_sample(), no side effects

            //Regs 0x0b-0x0c set up the envelope frequency; 0x0d is the shape noise
            0x0b | 0x0c | 0x0d => {
                let freq = Self::freq(self.reg[0x0b], self.reg[0x0c]);
                let shape = self.reg[0x0d];
                self.envelope.set_freq_shape(freq, shape);
                //log::info!("Envel: {} {}", freq, shape);
            }
            //Regs 0x0e-0x0f are I/O ports, not used for music

            _ => {}
        }
    }
    pub fn next_sample(&mut self, t: u32) -> u16 {
        //Reg 0x07 is a bitmask that _disables_ what is to be mixed to the final output:
        // * 0b0000_0001: do not mix freq_a
        // * 0b0000_0010: do not mix freq_b
        // * 0b0000_0100: do not mix freq_c
        // * 0b0000_1000: do not mix noise into channel A
        // * 0b0001_0000: do not mix noise into channel B
        // * 0b0010_0000: do not mix noise into channel C
        // * 0b1100_0000: IO ports, unused for music
        let mix = self.reg[0x07];
        let tone_a = (mix & 0x01) == 0;
        let tone_b = (mix & 0x02) == 0;
        let tone_c = (mix & 0x04) == 0;
        let noise_a = (mix & 0x08) == 0;
        let noise_b = (mix & 0x10) == 0;
        let noise_c = (mix & 0x20) == 0;

        // Generate the noise
        let noise = self.noise.next_sample(t);

        // Compute which channels are to be added
        let chan_a = Self::channel(tone_a, noise_a, &mut self.freq_a, noise, t);
        let chan_b = Self::channel(tone_b, noise_b, &mut self.freq_b, noise, t);
        let chan_c = Self::channel(tone_c, noise_c, &mut self.freq_c, noise, t);

        // Envelope is computed even if unused
        let env = self.envelope.next_sample(t);

        // Add the enabled channels, pondering the volume and the envelope
        let mut res : u16 = 0;
        if chan_a {
            let v = self.reg[0x08];
            let vol = Self::volume(v, env);
            res += vol;
        }
        if chan_b {
            let v = self.reg[0x09];
            let vol = Self::volume(v, env);
            res += vol;
        }
        if chan_c {
            let v = self.reg[0x0a];
            let vol = Self::volume(v, env);
            res += vol;
        }
        res
    }
    fn volume(v: u8, env: u8) -> u16 {
        let v = if v & 0x10 != 0 {
            env
        } else {
            v & 0x0f
        };
        static LEVELS: [u16; 16] = [0, 94, 133, 197, 283, 413, 589, 920, 1096, 1759, 2482, 3142, 4164, 5340, 6669, 8192];
        LEVELS[usize::from(v)]
    }
    fn freq(a: u8, b: u8) -> u16 {
        let n = u16::from_le_bytes([a, b]);
        n.max(1)
    }
    fn channel(tone_enabled: bool, noise_enabled: bool, freq: &mut FreqGen, noise: bool, t: u32) -> bool {
        if tone_enabled {
            let tone = freq.next_sample(t);
            if noise_enabled {
                tone && noise
            } else {
                tone
            }
        } else if noise_enabled {
            noise
        } else {
            true
        }
    }
}
