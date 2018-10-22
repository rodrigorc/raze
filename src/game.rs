use js;
use z80::{Z80, Bus};
use memory::Memory;
use tape::{Tape, TapePos};
use psg::PSG;
use std::io::Cursor;

const TIME_TO_INT : i32 = 69888;
const AUDIO_SAMPLE : i32 = 168;

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

struct ULA {
    memory: Memory,
    keys: [[bool; 5]; 9], //8 semirows plus joystick
    delay: u32,
    frame_counter: u32,
    time: i32,
    tape: Option<(Tape, Option<TapePos>)>,
    border: Pixel,
    ear: bool,
    psg: Option<PSG>,
}

impl ULA {
    pub fn take_delay(&mut self) -> u32 {
        let r = self.delay;
        self.delay = 0;
        r
    }
    pub fn add_time(&mut self, t: u32) {
        self.time += t as i32;
        self.tape = match self.tape.take() {
            Some((tape, Some(pos))) => {
                let index_pre = pos.block(&tape);
                let index_post;
                let next = tape.play(t, pos);
                if let Some(p) = &next {
                    self.ear = p.mic();
                    index_post = p.block(&tape);
                } else {
                    self.ear = false;
                    index_post = 0xffff_ffff;
                }
                if index_pre != index_post {
                    js::onTapeBlock(index_post);
                }
                Some((tape, next))
            }
            tape => tape
        };
    }
    pub fn audio_sample(&mut self, t: i32) -> u8 {
        let v : u8 = if self.ear { 0x40 } else { 0x00 };
        match &mut self.psg {
            None => v,
            Some(psg) => v.saturating_add(psg.next_sample(t))
        }
    }
}

impl Bus for ULA {
    fn peek(&mut self, addr: impl Into<u16>) -> u8 {
        self.memory.peek(addr)
    }
    fn poke(&mut self, addr: impl Into<u16>, value: u8) {
        self.memory.poke(addr, value);
    }
    fn do_in(&mut self, port: impl Into<u16>) -> u8 {
        let port = port.into();
        let lo = port as u8;
        let hi = (port >> 8) as u8;
        let mut r = 0xff;
        //ULA IO port
        if lo & 1 == 0 {
            self.delay += 1;
            if port >= 0x4000 && port < 0x8000 {
                self.delay += 1;
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

            if let Some((_, Some(pos))) = &self.tape {
                if pos.mic() {
                    r &= 0b1011_1111;
                }
            }
        } else {
            if port >= 0x4000 && port < 0x8000 {
                self.delay += 4;
            }
            match lo {
                0xfd => { //Programmable Sound Generator
                    if let Some(psg) = &self.psg {
                        match hi {
                            0xff => {
                                r = psg.read_reg();
                            }
                            _ => {
                                //log!("FD IN {:04x}, {:02x}", port, r);
                            }
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
                        self.memory.peek_no_delay(addr as u16)
                    } else { //borders or retraces
                        0xff
                    }
                }
                x if x & 0x20 == 0 => { //kempston joystick (0x1f | 0xdf ...)
                    let joy = &self.keys[8];
                    r = 0;
                    for (j, jj) in joy.iter().enumerate() {
                        if *jj {
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
    fn do_out(&mut self, port: impl Into<u16>, value: u8) {
        let port = port.into();
        let lo = port as u8;
        let hi = (port >> 8) as u8;
        if lo & 1 == 0 {
            //ULA IO port
            self.delay += 1;
            if port >= 0x4000 && port < 0x8000 {
                self.delay += 1;
            }
            let border = value & 7;
            self.border = PIXELS[0][border as usize];
            self.ear = value & 0x10 != 0;
            //log!("EAR {:02x} {:02x} {:02x} {}", hi, lo, value, ear);
            //log!("OUT {:04x}, {:02x}", port, value);
        } else {
            //log!("OUT {:04x}, {:02x}", port, value);
            if port >= 0x4000 && port < 0x8000 {
                self.delay += 4;
            }
            match lo {
                0xfd => { //128 stuff
                    match hi {
                        0x7f => { //Memory banks
                            //log!("MEM {:04x}, {:02x}", port, value);
                            self.memory.switch_banks(value);
                        }
                        0x1f => { //+2 Memory banks (TODO)
                            log!("MEM+2 {:04x}, {:02x}", port, value);
                        }
                        0xff => {
                            if let Some(psg) = &mut self.psg {
                                psg.select_reg(value);
                            }
                        }
                        0xbf => {
                            if let Some(psg) = &mut self.psg {
                                psg.write_reg(value);
                            }
                        }
                        _ => {
                            //log!("FD OUT {:04x}, {:02x}", port, value);
                        }
                    }
                }
                _ => {
                    //log!("OUT {:04x}, {:02x}", port, value);
                }
            }
        }
    }
}

pub struct Game {
    z80: Z80,
    ula: ULA,
    image: Vec<Pixel>,
    audio: Vec<u8>,
}

fn write_border_row(y: usize, border: Pixel, ps: &mut [Pixel]) {
    let prow = &mut ps[(BX0 + 256 + BX1) * y .. (BX0 + 256 + BX1) * (y+1)];
    for x in prow.iter_mut() {
        *x = border;
    }
}

fn write_screen_row(y: usize, border: Pixel, inv: bool, data: &[u8], ps: &mut [Pixel]) {
    let y = y as usize;
    let orow = match y {
        0..=63 => {
            (y % 8) * 256 + (y / 8) * 32
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
    let prow_full = &mut ps[(BX0 + 256 + BX1) * ym .. (BX0 + 256 + BX1) * (ym + 1)];
    for x in &mut prow_full[0..BX0] {
        *x = border;
    }
    for x in &mut prow_full[BX0 + 256 .. BX0 + 256 + BX1] {
        *x = border;
    }
    let prow = &mut prow_full[BX0 .. BX0 + 256];
    let arow = 192 * 32 + (y / 8) * 32;
    for ((&d, &attr), bits) in data[orow .. orow + 32].iter().zip(data[arow .. arow + 32].iter()).zip(prow.chunks_mut(8)) {
        for (b, bo) in bits.iter_mut().enumerate() {
            let pix = ((d >> (7-b)) & 1) != 0;
            let bright = (attr & 0b01_000_000) != 0;
            let blink = inv && (attr & 0b10_000_000) != 0;

            let c = if pix ^ blink {
                (attr & 0b00_000_111)
            } else {
                (attr & 0b00_111_000) >> 3
            };
            *bo = PIXELS[bright as usize][c as usize];
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
        let memory;
        let psg;
        if is128k {
            memory = Memory::new_from_bytes(include_bytes!("128-0.rom"), Some(include_bytes!("128-1.rom")));
            psg = Some(PSG::new());
        } else {
            memory = Memory::new_from_bytes(include_bytes!("48k.rom"), None);
            psg = None;
        };
        let z80 = Z80::new();
        let game = Game {
            z80,
            ula: ULA {
                memory,
                keys: Default::default(),
                delay: 0,
                frame_counter: 0,
                time: 0,
                tape: None,
                border: PIXELS[0][0],
                ear: false,
                psg,
            },
            image: vec![PIXELS[0][0]; (BX0 + 256 + BX1) * (BY0 + 192 + BY1)], //256x192 plus border
            audio: vec![],
        };
        game.into()
    }
    pub fn draw_frame(&mut self, turbo: bool) {
        //log!("Draw!");

        let n = if turbo { 100 } else { 1 };

        self.audio.clear();
        for _ in 0..n {
            self.ula.frame_counter = self.ula.frame_counter.wrapping_add(1);
            let inverted = self.ula.frame_counter % 32 < 16;
            let mut audio_time = 0; //TODO: use ula.time instead
            let mut screen_time = 0;
            let mut screen_row = 0;
            let mut audio_accum : u32 = 0;
            let mut audio_count : u32 = 0;
            while self.ula.time < TIME_TO_INT {
                let mut t = self.z80.exec(&mut self.ula);
                let delay_m = self.ula.memory.take_delay();
                let delay_io = self.ula.take_delay();
                //contended memory
                if self.ula.time >= 224*64 && self.ula.time < 224*256 {
                    //each row is 224 T, 128 are the real pixels where contention occurs
                    let offs = self.ula.time % 224;
                    //TODO: contention should be only in the first 128 T of each row, but that
                    //gives too fast emulation, so we compensate it for now by counting
                    //the whole row
                    if offs < 128 {
                        //we ignore the delay pattern (6,5,4,3,2,1,0,0) and instead do an
                        //average, it seems to be good enough
                        t += 4 * delay_m + 6 * delay_io;
                    }
                }
                self.ula.add_time(2);
                
                if !turbo {
                    audio_time += t as i32;
                    audio_accum += t * self.ula.audio_sample(t as i32) as u32;
                    audio_count += t;
                    if audio_time >= AUDIO_SAMPLE {
                        audio_time -= AUDIO_SAMPLE;
                        let sample = audio_accum / audio_count;
                        self.audio.push(if sample > 0xff { 0xff } else { sample as u8 });
                        audio_accum = 0;
                        audio_count = 0;
                    }
                    screen_time += t as i32;
                    while screen_time > 224 {
                        screen_time -= 224;
                        match screen_row {
                            60..=63 | 256..=259 => {
                                write_border_row(screen_row - 60, self.ula.border, &mut self.image);
                            }
                            64..=255 => {
                                let screen = self.ula.memory.video_memory();
                                write_screen_row(screen_row - 64, self.ula.border, inverted, screen, &mut self.image);
                            }
                            _ => {}
                        }
                        screen_row += 1;
                    }
                }
            }
            self.z80.interrupt(&mut self.ula);
            //we drag the excess T to the next loop
            self.ula.time -= TIME_TO_INT;
        }
        if turbo {
            let screen = self.ula.memory.video_memory();
            write_screen(self.ula.border, false, screen, &mut self.image);
        } else {
            while self.audio.len() < (TIME_TO_INT / AUDIO_SAMPLE) as usize {
                self.audio.push(self.ula.audio_sample(0));
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
            self.ula.keys[r][k] = false;
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
            self.ula.keys[r][k] = true;
            key >>= 8;
        }
    }
    pub fn reset_input(&mut self) {
        for r in self.ula.keys.iter_mut() {
            for k in r.iter_mut() {
                *k = false;
            }
        }
    }
    pub fn tape_load(&mut self, data: Vec<u8>) -> usize {
        match Tape::new(&mut Cursor::new(data)) {
            Ok(tape) => {
                let res = tape.len();
                if res > 0 {
                    self.ula.tape = Some((tape, Some(TapePos::new_at_block(0))));
                } else {
                    self.ula.tape = None;
                }
                res
            }
            Err(e) => {
                alert!("{}", e);
                0
            }
        }
    }
    pub fn tape_name(&self, index: usize) -> &str {
        match &self.ula.tape {
            Some((tape, _)) => {
                tape.block_name(index)
            }
            None => {
                ""
            }
        }
    }
    pub fn tape_selectable(&self, index: usize) -> bool {
        match &self.ula.tape {
            Some((tape, _)) => tape.block_selectable(index),
            None => false
        }
    }
    pub fn tape_seek(&mut self, index: usize) {
        self.ula.tape = match self.ula.tape.take() {
            Some((tape, _)) => {
                js::onTapeBlock(index);
                Some((tape, Some(TapePos::new_at_block(index))))
            }
            None => None
        }
    }
    pub fn tape_stop(&mut self) {
        self.ula.tape = match self.ula.tape.take() {
            Some((tape, _)) => {
                self.ula.ear = false;
                Some((tape, None))
            }
            None => None
        }
    }
    pub fn snapshot(&self) -> Vec<u8> {
        let mut data = Vec::new();
        self.ula.memory.save(&mut data).unwrap();
        self.z80.save(&mut data).unwrap();
        data
    }
    pub fn load_snapshot(&mut self, data: Vec<u8>) {
        let mut load = Cursor::new(data);
        self.ula.memory = Memory::load(&mut load).unwrap();
        self.z80.load(&mut load).unwrap();
    }
}

