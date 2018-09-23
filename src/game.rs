use js;
use z80::{Z80, InOut};
use memory::Memory;
use tape::{Tape, TapePos};
use std::io::Cursor;

#[repr(C)]
#[derive(Copy, Clone)]
struct Pixel(u8,u8,u8,u8);

static PIXELS : [[Pixel; 8]; 2] = [
    [Pixel(0,0,0,0xff), Pixel(0,0,0xd7,0xff), Pixel(0xd7,0,0,0xff), Pixel(0xd7,0,0xd7,0xff), Pixel(0,0xd7,0,0xff), Pixel(0,0xd7,0xd7,0xff), Pixel(0xd7,0xd7,0,0xff), Pixel(0xd7,0xd7,0xd7,0xff)],
    [Pixel(0,0,0,0xff), Pixel(0,0,0xff,0xff), Pixel(0xff,0,0,0xff), Pixel(0xff,0,0xff,0xff), Pixel(0,0xff,0,0xff), Pixel(0,0xff,0xff,0xff), Pixel(0xff,0xff,0,0xff), Pixel(0xff,0xff,0xff,0xff)],
];

//margins
const BX0: usize = 5;
const BX1: usize = 5;
const BY0: usize = 4;
const BY1: usize = 4;

struct IO {
    keys: [[bool; 5]; 9], //8 semirows plus joystick
    delay: u32,
    frame_counter: u32, time: i32,
    tape: Option<(Tape, i32, TapePos)>,
    border: Pixel,
    ear: bool,
}

impl IO {
    pub fn take_delay(&mut self) -> u32 {
        let r = self.delay;
        self.delay = 0;
        r
    }
}

impl InOut for IO {
    fn do_in(&mut self, port: u16, mem: &mut Memory, _cpu: &Z80) -> u8 {
        let lo = port as u8;
        let hi = (port >> 8) as u8;
        let mut r = 0xff;
        //ULA IO port
        if lo & 1 == 0 {
            self.delay = self.delay.wrapping_add(1);
            if port >= 0x4000 && port < 0x8000 {
                self.delay = self.delay.wrapping_add(1);
            }
            for i in 0..8 { //half row keyboard
                if hi & (1 << i) == 0 {
                    for j in 0..5 { //keys
                        if self.keys[i][j] {
                            r &= !(1 << j);
                        }
                    }
                }
            }

            if let Some((tape, last_time, pos)) = self.tape.take() {
                let delta = self.time - last_time;
                let delta = if delta > 0 { delta as u32 } else { 0 };
                let mic = match tape.play(delta, pos) {
                    Some(next) => {
                        let mic = next.mic();
                        self.tape = Some((tape, self.time, next));
                        self.ear = mic;
                        mic
                    }
                    None => false
                };
                if mic {
                    r &= 0b1011_1111;
                }
            }
        } else {
            if port >= 0x4000 && port < 0x8000 {
                self.delay = self.delay.wrapping_add(4);
            }
            match lo {
                0xfd => { //Programmable Sound Generator
                    match hi {
                        0xff => {
                            r = 0;
                            //log!("PSG IN {:04x}, {:02x}", port, r);
                        }
                        _ => {
                            log!("FD IN {:04x}, {:02x}", port, r);
                        }
                    }
                }
                0xff => { //reads stale data from the bus (last attr byte?)
                    let row = self.time / 224;
                    let ofs = self.time % 224;
                    r = if row >= 64 && row < 256 && ofs < 128 {
                        let row = row - 64;
                        let ofs = ofs / 8 * 2 + 1; //attrs are read in pairs each 8 T, more or less
                        let addr = (0x4000 + 192 * 32) + 32 * row + ofs;
                        mem.peek_no_delay(addr as u16)
                    } else { //borders or retraces
                        0xff
                    }
                }
                0x1f => { //kempston joystick
                    let ref joy = self.keys[8];
                    r = 0;
                    for j in 0..5 {
                        if joy[j] {
                            r |= 1 << j;
                        }
                    }
                }
                _ => {
                    //log!("IN {:04x}, {:02x}", port, r);
                }
            }
        }
        r
    }
    fn do_out(&mut self, port: u16, value: u8, mem: &mut Memory, _cpu: &Z80) {
        let lo = port as u8;
        let hi = (port >> 8) as u8;
        if lo & 1 == 0 {
            //ULA IO port
            self.delay = self.delay.wrapping_add(1);
            if port >= 0x4000 && port < 0x8000 {
                self.delay = self.delay.wrapping_add(1);
            }
            let border = value & 7;
            self.border = PIXELS[0][border as usize];
            self.ear = value & 0x10 != 0;
            //log!("EAR {:02x} {:02x} {:02x} {}", hi, lo, value, ear);
            //log!("OUT {:04x}, {:02x}", port, value);
        } else {
            //log!("OUT {:04x}, {:02x}", port, value);
            if port >= 0x4000 && port < 0x8000 {
                self.delay = self.delay.wrapping_add(4);
            }
            match lo {
                0xfd => { //128 stuff
                    match hi {
                        0x7f => { //Memory banks
                            //log!("MEM {:04x}, {:02x}", port, value);
                            mem.switch_banks(value);
                        }
                        0x1f => { //+2 Memory banks (TODO)
                            log!("MEM+2 {:04x}, {:02x}", port, value);
                        }
                        0xff | 0xbf => { //PSG
                            //log!("PSG OUT {:04x}, {:02x}", port, value);
                        }
                        _ => {
                            log!("FD OUT {:04x}, {:02x}", port, value);
                        }
                    }
                }
                _ => {
                }
            }
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

fn write_border_row(y: usize, border: Pixel, ps: &mut [Pixel]) {
    let prow = &mut ps[(BX0 + 256 + BX1) * y .. (BX0 + 256 + BX1) * (y+1)];
    for x in 0..BX0 + 256 + BX1 {
        prow[x] = border;
    }
}

fn write_screen_row(y: usize, border: Pixel, inv: bool, data: &[u8], ps: &mut [Pixel]) {
    let y = y as usize;
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
    let ym = y + BY0;
    let prow = &mut ps[(BX0 + 256 + BX1) * ym .. (BX0 + 256 + BX1) * (ym + 1)];
    for x in 0..BX0 {
        prow[x] = border;
    }
    for x in 0..BX1 {
        prow[BX0 + 256 + x] = border;
    }
    for x in 0..32 {
        let attr = data[192 * 32 + (y / 8) * 32 + x];
        let d = data[orow + x];
        for b in 0..8 {
            let pix = ((d >> (7-b)) & 1) != 0;
            let bright = (attr & 0b0100_0000) != 0;
            let blink = inv && (attr & 0b1000_0000) != 0;

            let c = if pix ^ blink {
                (attr & 0b0000_0111)
            } else {
                (attr & 0b0011_1000) >> 3
            };
            let offs = BX0 + 8 * x + b;
            prow[offs] = PIXELS[bright as usize][c as usize];
        }
    }
}

fn write_screen(border: Pixel, inv: bool, data: &[u8], ps: &mut [Pixel]) {
    for y in 0..BY0 {
        write_border_row(y, border, ps);
    }
    for y in 0..192 {
        write_screen_row(y, border, inv, data, ps);
    }
    for y in 0..BY1 {
        write_border_row(BY0 + 192 + y, border, ps);
    }
}

impl Game {
    pub fn new(is128k: bool) -> Box<Game> {
        log!("Go!");
        let memory = if is128k {
            Memory::new_from_bytes(include_bytes!("128-0.rom"), Some(include_bytes!("128-1.rom")))
        } else {
            Memory::new_from_bytes(include_bytes!("48k.rom"), None)
        };
        let z80 = Z80::new();
        let game = Game{
            memory, z80,
            io: IO { keys: Default::default(), delay: 0, frame_counter: 0, time: 0, tape: None, border: PIXELS[0][0], ear: false },
            image: vec![PIXELS[0][0]; (BX0 + 256 + BX1) * (BY0 + 192 + BY1)], //256x192 plus border
            audio: vec![],
        };
        game.into()
    }
    pub fn draw_frame(&mut self, turbo: bool) {
        //log!("Draw!");

        let n = if turbo { 100 } else { 1 };
        const TIME_TO_INT : i32 = 69888;
        const AUDIO_SAMPLE : i32 = 168;

        self.audio.clear();
        let mut inverted = false;
        for _ in 0..n {
            self.io.frame_counter = self.io.frame_counter.wrapping_add(1);
            inverted = self.io.frame_counter % 32 < 16;
            self.io.time = 0;
            let mut audio_time = 0;
            let mut screen_time = 0;
            let mut screen_row = 0;
            while self.io.time < TIME_TO_INT {
                let mut t = self.z80.exec(&mut self.memory, &mut self.io);
                let delay = self.memory.take_delay() + self.io.take_delay();
                //contended memory
                if self.io.time >= 224*64 && self.io.time < 224*256 {
                    //each row is 224 T, 128 are the real pixels where contention occurs
                    let offs = self.io.time % 224;
                    if offs < 128 {
                        t += (delay * 21) / 8;
                    }
                }
                self.io.time += t as i32;
                if !turbo {
                    audio_time += t as i32;
                    while audio_time > AUDIO_SAMPLE {
                        audio_time -= AUDIO_SAMPLE;
                        self.audio.push(self.io.ear as u8);
                    }
                    screen_time += t as i32;
                    while screen_time > 224 {
                        screen_time -= 224;
                        match screen_row {
                            60..=63 | 256..=259 => {
                                write_border_row(screen_row - 60, self.io.border, &mut self.image);
                            }
                            64..=255 => {
                                let screen = self.memory.video_memory();
                                write_screen_row(screen_row - 64, self.io.border, inverted, screen, &mut self.image);
                            }
                            _ => {}
                        }
                        if screen_row >= 64 && screen_row < 256 {
                        }
                        screen_row += 1;
                    }
                }
            }
            self.z80.interrupt(&mut self.memory);
        }
        if turbo {
            let screen = self.memory.video_memory();
            write_screen(self.io.border, inverted, screen, &mut self.image);
        } else {
            while self.audio.len() < (TIME_TO_INT / AUDIO_SAMPLE) as usize {
                self.audio.push(self.io.ear as u8);
            }
            js::putSoundData(&self.audio);
        }
        js::putImageData((BX0 + 256 + BX1) as i32, (BY0 + 192 + BY1) as i32, &self.image);
    }
    pub fn key_up(&mut self, mut key: usize) {
        while key != 0 {
            let k = key & 0x07;
            let r = match (key >> 4) & 0x0f {
                0x0f => 0,
                r => r
            };
            self.io.keys[r][k] = false;
            key >>= 8;
        }
    }
    pub fn key_down(&mut self, mut key: usize) {
        while key != 0 {
            let k = key & 0x07;
            let r = match (key >> 4) & 0x0f {
                0x0f => 0,
                r => r
            };
            self.io.keys[r][k] = true;
            key >>= 8;
        }
    }
    pub fn reset_input(&mut self) {
        for r in self.io.keys.iter_mut() {
            for k in r.iter_mut() {
                *k = false;
            }
        }
    }
    pub fn load_tape(&mut self, data: Vec<u8>) {
        match Tape::new(data) {
            Ok(tape) => {
                self.io.time = 0;
                self.io.tape = Some((tape, self.io.time, TapePos::new()));
            }
            Err(e) => alert!("{}", e),
        }
    }
    pub fn snapshot(&self) -> Vec<u8> {
        let mut data = Vec::new();
        self.memory.save(&mut data).unwrap();
        self.z80.save(&mut data).unwrap();
        data
    }
    pub fn load_snapshot(&mut self, data: Vec<u8>) {
        let mut load = Cursor::new(data);
        self.memory = Memory::load(&mut load).unwrap();
        self.z80.load(&mut load).unwrap();
    }
}

