//Emulation of the AY-3-8910 programmable sound generator

struct FreqGen {
    divisor: i32,
    phase: i32,
}

impl FreqGen {
    fn new() -> FreqGen {
        FreqGen {
            divisor: 32,
            phase: 0,
        }
    }
    fn set_freq(&mut self, freq: u16) {
        self.divisor = 32 * i32::from(freq);
        self.phase = 0;
    }
    fn next_sample(&mut self, t: i32) -> bool {
        self.phase += t;
        while self.phase > self.divisor {
            self.phase -= self.divisor;
        }
        self.phase < self.divisor / 2
    }
}

struct NoiseGen {
    divisor: i32,
    shift: u32,
    level: bool,
    phase: i32,
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
        self.divisor = 32 * i32::from(freq);
        //log!("noise div {}", self.divisor);
    }
    fn next_sample(&mut self, t: i32) -> bool {
        self.phase += t;
        while self.phase > self.divisor {
            self.phase -= self.divisor;
            let bit0 = (self.shift & 1) != 0;
            let bit3 = (self.shift & 8) != 0;
            self.level ^= bit0;
            if bit0 ^ bit3 {
                self.shift ^= 0x10000;
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
    divisor: i32,
    phase: i32,
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
        self.divisor = 32 * i32::from(freq);
        self.phase = 0;
        self.step = 0;
        let (shape, block) = match shape & 0x0f {
            0x00 | 0x01 | 0x02 | 0x03 | 0x09 => (LowerLow, Lower),
            0x04 | 0x05 | 0x06 | 0x07 | 0x0f => (RaiseLow, Raise),
            0x08 => (LowerLoop, Lower),
            0x0a => (LowerRaiseLoop, Lower),
            0x0b => (LowerHigh, Lower),
            0x0c => (RaiseLoop, Raise),
            0x0d => (RaiseHigh, Raise),
            0x0e => (RaiseLowerLoop, Raise),
            _ => unreachable!(),
        };
        self.shape = shape;
        self.block = block;
    }
    fn next_sample(&mut self, t: i32) -> u8 {
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

pub struct PSG {
    reg_sel: u8,
    reg: [u8; 16],
    freq_a: FreqGen,
    freq_b: FreqGen,
    freq_c: FreqGen,
    noise: NoiseGen,
    envelope: Envelope,
}

impl PSG {
    pub fn new() -> PSG {
        PSG {
            reg_sel: 0,
            reg: Default::default(),
            freq_a: FreqGen::new(),
            freq_b: FreqGen::new(),
            freq_c: FreqGen::new(),
            noise: NoiseGen::new(),
            envelope: Envelope::new(),
        }
    }
    pub fn load_snapshot(data: &[u8]) -> PSG {
        let mut psg = Self::new();
        for (r, &v) in (0..16).zip(&data[1..17]) {
            psg.reg_sel = r;
            psg.write_reg(v);
        }
        psg.reg_sel = data[0];
        psg
    }
    pub fn snapshot(&self, data: &mut [u8]) {
        data[0] = self.reg_sel;
        data[1..17].copy_from_slice(&self.reg);
    }
    pub fn select_reg(&mut self, reg: u8) {
        if let 0..=0x0f = reg {
            self.reg_sel = reg;
        }
    }
    pub fn read_reg(&self) -> u8 {
        //log!("PSG read {:02x} <- {:02x}", self.psg_sel, r);
        self.reg[usize::from(self.reg_sel)]
    }
    pub fn write_reg(&mut self, x: u8) {
        self.reg[usize::from(self.reg_sel)] = x;
        //log!("PSG write {:02x} <- {:02x}", self.reg_sel, x);
        match self.reg_sel {
            0x00 | 0x01 => {
                let freq = Self::freq_12(self.reg[0x00], self.reg[0x01]);
                self.freq_a.set_freq(freq);
                //log!("Tone A: {}", freq);
            }
            0x02 | 0x03 => {
                let freq = Self::freq_12(self.reg[0x02], self.reg[0x03]);
                self.freq_b.set_freq(freq);
                //log!("Tone B: {}", freq);
            }
            0x04 | 0x05 => {
                let freq = Self::freq_12(self.reg[0x04], self.reg[0x05]);
                self.freq_c.set_freq(freq);
                //log!("Tone C: {}", freq);
            }
            0x06 => {
                let noise = self.reg[0x06] & 0x1f;
                self.noise.set_freq(if noise == 0 { 1 } else { noise });
                //log!("Noise A: {}", noise);
            }
            0x0b | 0x0c | 0x0d=> {
                let freq = Self::freq_16(self.reg[0x0b], self.reg[0x0c]);
                let shape = self.reg[0x0d];
                self.envelope.set_freq_shape(freq, shape);
                //log!("Envel: {} {}", freq, shape);
            }
            _ => {}
        }
    }
    pub fn next_sample(&mut self, t: i32) -> i16 {
        let mix = self.reg[0x07];
        let mut res : i16 = 0;
        let noise = if mix & 0x38 != 0x38 {
            self.noise.next_sample(t)
        } else {
            false
        };
        let tone_a = (mix & 0x01) == 0;
        let tone_b = (mix & 0x02) == 0;
        let tone_c = (mix & 0x04) == 0;
        let noise_a = (mix & 0x08) == 0;
        let noise_b = (mix & 0x10) == 0;
        let noise_c = (mix & 0x20) == 0;

        let chan_a = Self::channel(tone_a, noise_a, &mut self.freq_a, noise, t);
        let chan_b = Self::channel(tone_b, noise_b, &mut self.freq_b, noise, t);
        let chan_c = Self::channel(tone_c, noise_c, &mut self.freq_c, noise, t);
        let env = self.envelope.next_sample(t);

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
    fn volume(v: u8, env: u8) -> i16 {
        let v = if v & 0x10 != 0 {
            env
        } else {
            v & 0x0f
        };
        //The volume curve is an exponential where each level is sqrt(2) lower than the next,
        //but with an offset so that the first one is 0. computed with this python line:
        //>>> [round(8192*exp(i/2-7.5)) for i in range(0, 16)]
        const LEVELS: [i16; 16] = [5, 7, 12, 20, 33, 55, 91, 150, 247, 408, 672, 1109, 1828, 3014, 4969, 8192];
        LEVELS[usize::from(v)]
    }
    fn freq_12(a: u8, b: u8) -> u16 {
        let n = u16::from(a) | (u16::from(b & 0x0f) << 8);
        if n == 0 { 1 } else { n }
    }
    fn freq_16(a: u8, b: u8) -> u16 {
        let n = u16::from(a) | (u16::from(b) << 8);
        if n == 0 { 1 } else { n }
    }
    fn channel(tone_enabled: bool, noise_enabled: bool, freq: &mut FreqGen, noise: bool, t: i32) -> bool {
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
