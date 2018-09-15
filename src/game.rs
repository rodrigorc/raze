use js;
use z80::{Z80, InOut};
use memory::Memory;
use tape::Tape;

#[repr(C)]
#[derive(Copy, Clone)]
struct Pixel(u8,u8,u8,u8);

enum TapePhase {
    Pause{t: u32}, //500000 T
    Leader{ pulse: u32, t: u32 }, //8063 or 3223 pulses of 2168 T each
    FirstSync{ t: u32 }, //667 T
    SecondSync{ t: u32 }, //735 T
    Data{ pos: u32, bit: u8, t: u32 }, //2 * 855 T or 1710 T
}

struct TapePos {
    block: u32,
    phase: TapePhase,
}

struct IO {
    keys: [[bool; 5]; 8],
    frame_counter: u32,
    time: u32,
    tape: Option<(Tape, TapePos)>,
    border: u8,
    ear: u8,
}

impl IO {
    fn add_time(&mut self, tstates: u32) {
        self.time += tstates;
    }
}

fn play_tape(d: u32, tape: &Tape, pos: TapePos) -> Option<(bool, TapePos)> {
    let mic;
    let TapePos{ mut block, phase } = pos;
    if (block as usize) >= tape.data.len() {
        return None;
    }
    let next = match phase {
        TapePhase::Pause{t} => {
            mic = false;
            if t < 500000 {
                TapePhase::Pause{ t: t + d }
            } else {
                log!("leader");
                TapePhase::Leader{ pulse: 0, t: 0 }
            }
        }
        TapePhase::Leader{pulse, t} => {
            mic = pulse % 2 != 0;
            if t < 2168 {
                TapePhase::Leader{ pulse, t: t + d }
            } else if pulse < 3223 {
                TapePhase::Leader{ pulse: pulse + 1, t: 0 }
            } else {
                log!("firstsync");
                TapePhase::FirstSync{ t: 0 }
            }
        }
        TapePhase::FirstSync{t} => {
            mic = false;
            if t < 667 {
                TapePhase::FirstSync{ t: t + d }
            } else {
                log!("secondsync");
                TapePhase::SecondSync{ t: 0 }
            }
        }
        TapePhase::SecondSync{t} => {
            mic = true;
            if t < 735 {
                TapePhase::SecondSync{ t: t + d }
            } else {
                log!("data");
                TapePhase::Data{ pos: 0, bit: 0, t: 0 }
            }
        }
        TapePhase::Data{pos, bit, t } => {
            let byte = tape.data[block as usize][pos as usize];
            let v = byte & (0x80 >> bit) != 0;
            let len = if v { 1710 } else { 855 };
            if t < len {
                mic = false;
                TapePhase::Data{ pos, bit, t: t + d }
            } else {
                mic = true;
                if t < 2 * len {
                    TapePhase::Data{ pos, bit, t: t + d }
                } else if bit < 8 - 1 {
                    TapePhase::Data{ pos, bit: bit + 1, t: 0 }
                } else if (pos as usize) < tape.data[block as usize].len() - 1 {
                    TapePhase::Data{ pos: pos + 1, bit: 0, t: 0 }
                } else if (block as usize) < tape.data.len() {
                    block += 1;
                    TapePhase::Pause{ t: 0 }
                } else {
                    return None;
                }
            }
        }
    };
    Some((mic, TapePos{ block, phase: next }))
}


impl InOut for IO {
    fn do_in(&mut self, port: u16) -> u8 {
        let lo = port as u8;
        let hi = (port >> 8) as u8;
        let mut r = 0xff;
        //ULA IO port
        if lo & 1 == 0 {
            for i in 0..8 { //half row keyboard
                if hi & (1 << i) == 0 {
                    for j in 0..5 { //keys
                        if self.keys[i][j] {
                            r &= !(1 << j);
                        }
                    }
                }
            }

            if let Some((tape, pos)) = self.tape.take() {
                let delta = self.time;
                self.time = 0;
                let mic = if let Some((mic, next)) = play_tape(delta, &tape, pos) {
                    self.tape = Some((tape, next));
                    mic
                } else {
                    false
                };
                if !mic {
                    r &= 0b1011_1111;
                }
            }
        } else if lo == 0xff {
            r = (self.time >> 8) as u8;
        }
        //log!("IN {:04x}, {:02x}", port, r);
        r
    }
    fn do_out(&mut self, port: u16, value: u8) {
        let lo = port as u8;
        let hi = (port >> 8) as u8;
        //ULA IO port
        if lo & 1 == 0 {
            let border = value & 7;
            if self.border != border {
                self.border = border;
            }
            let ear = value & 0x10 != 0;
            self.ear = if ear { 1 } else { 0 };
            //log!("EAR {:02x} {:02x} {:02x} {}", hi, lo, value, ear);
            //log!("OUT {:04x}, {:02x}", port, value);
        }
    }
}

pub struct Game {
    memory: Memory,
    z80: Z80,
    io: IO,
    image: Vec<Pixel>,
    audio: Vec<u8>,
}

fn write_screen(inv: bool, data: &[u8], ps: &mut [Pixel]) {
    assert!(ps.len() == 256 * 192);
    for y in 0..192 {
        let orow = match y {
            0..=63 => {
                let y = (y % 8) * 256 + (y / 8) * 32;
                y
            }
            64..=127 => {
                let y = y - 64;
                let y = (y % 8) * 256 + (y / 8) * 32;
                y + 64 * 32
            }
            128..=191 => {
                let y = y - 128;
                let y = (y % 8) * 256 + (y / 8) * 32;
                y + 128 * 32
            }
            _ => unreachable!()
        };
        for x in 0..32 {
            let attr = data[192 * 32 + (y / 8) * 32 + x];
            let d = data[orow + x];
            for b in 0..8 {
                let pix = ((d >> (7-b)) & 1) != 0;
                let v = if (attr & 0b0100_0000) != 0 { 0xff } else { 0xd7 };
                let blink = inv && (attr & 0b1000_0000) != 0;

                let c = if pix ^ blink {
                    (attr & 0b0000_0111)
                } else {
                    (attr & 0b0011_1000) >> 3
                };
                let offs = 256 * y + 8 * x + b;
                ps[offs] = Pixel(
                    if (c & 2) != 0 { v } else { 0 },
                    if (c & 4) != 0 { v } else { 0 },
                    if (c & 1) != 0 { v } else { 0 },
                    0xff,
                    );
            }
        }
    }
}

impl Game {
    pub fn new() -> Box<Game> {
        log!("Go!");
        let mut memory = Memory::new_from_bytes(include_bytes!("48k.rom"));
        let mut z80 = Z80::new();
        let game = Game{
            memory, z80,
            io: IO { keys: Default::default(), frame_counter: 0, time: 0, tape: None, border: 0xff, ear: 0 },
            image: vec![Pixel(0,0,0,0xff); 256 * 192],
            audio: vec![],
        };
        game.into()
    }
    pub fn draw_frame(&mut self) {
        //log!("Draw!");

        let n = if self.io.tape.is_some() { 100 } else { 1 };
        const TIME_TO_INT : i32 = 69888;
        const AUDIO_SAMPLE : i32 = 168;

        self.audio.clear();
        for _ in 0..n {
            self.io.frame_counter = self.io.frame_counter.wrapping_add(1);
            let mut time = 0;
            let mut audio_time = 0;
            while time < TIME_TO_INT {
                let t = self.z80.exec(&mut self.memory, &mut self.io);
                self.io.add_time(t);
                time += t as i32;
                if n == 1 {
                    audio_time += t as i32;
                    while audio_time > AUDIO_SAMPLE {
                        audio_time -= AUDIO_SAMPLE;
                        self.audio.push(self.io.ear);
                    }
                }
            }
            self.z80.interrupt(&mut self.memory);
        }
        if n == 1 {
            while self.audio.len() < (TIME_TO_INT / AUDIO_SAMPLE) as usize {
                self.audio.push(self.io.ear);
            }
            js::putSoundData(&self.audio);
        }
        let screen = self.memory.slice(0x4000, 0x4000 + 32 * 192 + 32 * 24);
        const BORDER_COLORS : [&str; 8] = ["#000000", "#0000d7", "#d70000", "#d700d7", "#00d700", "#00d7d7", "#d7d700", "#d7d7d7"];
        write_screen(self.io.frame_counter % 32 < 16, screen, &mut self.image);

        js::putImageData(self.io.border, 256, 192, &self.image);
    }
    pub fn key_up(&mut self, mut key: usize) {
        while key != 0 {
            let k = key & 0x07;
            let r = (key >> 4) & 0x07;
            self.io.keys[r][k] = false;
            key >>= 8;
        }
    }
    pub fn key_down(&mut self, mut key: usize) {
        while key != 0 {
            let k = key & 0x07;
            let r = (key >> 4) & 0x07;
            self.io.keys[r][k] = true;
            key >>= 8;
        }
    }
    pub fn load_file(&mut self, data: Vec<u8>) {
        match Tape::new(data) {
            Ok(t) => {
                self.io.time = 0;
                self.io.tape = Some((t, TapePos { block: 0, phase: TapePhase::Pause { t: 0 } }));
            }
            Err(e) => alert!("{}", e),
        }
    }
    pub fn snapshot(&self) -> Vec<u8> {
        let mut data = Vec::new();
        self.memory.save(&mut data);
        log!("snap 1 {} bytes", data.len());
        self.z80.save(&mut data).unwrap();
        log!("snap 2 {} bytes", data.len());
        data
    }
}

