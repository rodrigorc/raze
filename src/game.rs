use crate::js;
use crate::z80::{Z80, Bus, Z80FileVersion};
use crate::memory::Memory;
use crate::tape::{Tape, TapePos};
use crate::psg::PSG;
use crate::speaker::Speaker;
use std::io::{self, Cursor, Write};
use std::borrow::Cow;

const TIME_TO_INT : i32 = 69888;

static ROM_128_0: &[u8] = include_bytes!("128-0.rom");
static ROM_128_1: &[u8] = include_bytes!("128-1.rom");
static ROM_48: &[u8] = include_bytes!("48k.rom");

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq)]
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
    keys: [u8; 9], //8 semirows plus joystick
    delay: u32,
    frame_counter: u32,
    time: i32,
    tape: Option<(Tape, Option<TapePos>)>,
    border: Pixel,
    ear: bool,
    mic: bool,
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
                    self.mic = p.mic();
                    index_post = p.block(&tape);
                } else {
                    self.mic = false;
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
    pub fn audio_sample(&mut self, t: i32) -> i16 {
        let v = if self.ear { 0x2000 } else { 0 } + if self.mic { 0x1000 } else { 0 };
        match &mut self.psg {
            None => v,
            Some(psg) => v + psg.next_sample(t),
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
                    r &= !self.keys[i];
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
                        #[allow(clippy::single_match)]
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
                    r = self.keys[8];
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
            self.ear = (value & 0x10) != 0;
            self.mic = (value & 0x08) != 0;
        } else {
            //log!("OUT {:04x}, {:02x}", port, value);
            if port >= 0x4000 && port < 0x8000 {
                self.delay += 4;
            }
            #[allow(clippy::single_match)]
            match lo {
                0xfd => { //128 stuff
                    match hi {
                        0x7f => { //Memory banks
                            //log!("MEM {:04x}, {:02x}", port, value);
                            self.memory.switch_banks(value);
                        }
                        0x1f => { //+2 Memory banks
                            //log!("MEM+2 {:04x}, {:02x}", port, value);
                            self.memory.switch_banks_plus2(value);
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
                        hi if (hi & 0x80) == 0 => { //same as 0x7f
                            //log!("MEM {:04x}, {:02x}", port, value);
                            self.memory.switch_banks(value);
                        }
                        hi if (hi & 0xf0) == 0x10 => { //same as 0x1f
                            //log!("MEM+2 {:04x}, {:02x}", port, value);
                            self.memory.switch_banks_plus2(value);
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
    is128k: bool,
    z80: Z80,
    ula: ULA,
    image: Vec<Pixel>,
    speaker: Speaker,
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
    pub fn new(is128k: bool) -> Game {
        log!("Go!");
        let memory;
        let psg;
        if is128k {
            memory = Memory::new_from_bytes(ROM_128_0, Some(ROM_128_1));
            psg = Some(PSG::new());
        } else {
            memory = Memory::new_from_bytes(ROM_48, None);
            psg = None;
        };
        let z80 = Z80::new();
        Game {
            is128k,
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
                mic: false,
                psg,
            },
            image: vec![PIXELS[0][0]; (BX0 + 256 + BX1) * (BY0 + 192 + BY1)], //256x192 plus border
            speaker: Speaker::new(),
        }
    }
    pub fn is_128k(&self) -> bool {
        self.is128k
    }
    pub fn draw_frame(&mut self, turbo: bool) {
        //log!("Draw!");

        let n = if turbo { 100 } else { 1 };

        for _ in 0..n {
            self.ula.frame_counter = self.ula.frame_counter.wrapping_add(1);
            let inverted = self.ula.frame_counter % 32 < 16;
            let mut screen_time = 0;
            let mut screen_row = 0;
            while self.ula.time < TIME_TO_INT {
                let mut t = self.z80.exec(&mut self.ula);
                //self.z80._dump_regs();
                //contended memory and IO
                let delay_m = self.ula.memory.take_delay();
                let delay_io = self.ula.take_delay();
                if self.ula.time >= 224*64 && self.ula.time < 224*256 && self.ula.time % 224 < 128 {
                    //each row is 224 T, 128 are the real pixels where contention occurs
                    //we ignore the delay pattern (6,5,4,3,2,1,0,0) and instead do an
                    //estimation
                    match delay_m + delay_io {
                        0 => (),
                        1 => t += 4, //only 1 contention: these many Ts on average
                        x => t += 6*x - 2, //more than 1 contention: they use to chain so max up all but the first one
                    }
                }
                self.ula.add_time(t);

                if !turbo {
                    let sample = self.ula.audio_sample(t as i32);
                    self.speaker.push_sample(sample, t as i32);
                    screen_time += t as i32;
                    while screen_time >= 224 {
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
            self.z80.interrupt();
            //we drag the excess T to the next loop
            self.ula.time -= TIME_TO_INT;
        }
        if turbo {
            let screen = self.ula.memory.video_memory();
            write_screen(self.ula.border, false, screen, &mut self.image);
        } else {
            //adding samples should be rarely necessary, so use lazy generation
            let ula = &mut self.ula;
            let audio = self.speaker.complete_frame(TIME_TO_INT, || ula.audio_sample(0));
            js::putSoundData(audio);
            self.speaker.clear();
        }
        js::putImageData((BX0 + 256 + BX1) as i32, (BY0 + 192 + BY1) as i32, &self.image);
    }
    //Every byte in key is a key pressed:
    //  * low nibble: key number (0..5)
    //  * high nibble: row number (0..7, 8 = kempston)
    //A byte 0x00 means no key. To refer to key 0 in row 0 use 0x08, because that bit is ignored
    pub fn key_up(&mut self, mut keys: usize) {
        while keys != 0 {
            let k = keys & 0x07;
            let r = (keys >> 4) & 0x0f;
            self.ula.keys[r] &= !(1 << k);
            keys >>= 8;
        }
    }
    //Same as key_up
    pub fn key_down(&mut self, mut keys: usize) {
        while keys != 0 {
            let k = keys & 0x07;
            let r = (keys >> 4) & 0x0f;
            self.ula.keys[r] |= 1 << k;
            keys >>= 8;
        }
    }
    pub fn reset_input(&mut self) {
        self.ula.keys = Default::default();
    }
    pub fn tape_load(&mut self, data: Vec<u8>) -> usize {
        match Tape::new(Cursor::new(data), self.is128k) {
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
                self.ula.mic = false;
                Some((tape, None))
            }
            None => None
        }
    }
    pub fn snapshot(&self) -> Vec<u8> {
        //We will save V2 or V3 depending on the plus2 memory bank
        let banks_plus2 = self.ula.memory.last_banks_plus2();

        const HEADER: usize = 32;
        let header_extra = if banks_plus2 == 0 { 23 } else { 55 };
        let mut data = vec![0; HEADER + header_extra];
        self.z80.snapshot(&mut data);
        data[12] |= (PIXELS[0].iter().position(|c| *c == self.ula.border).unwrap_or(0) as u8) << 1;

        //extended header block
        //len of the block
        data[30] = header_extra as u8; data[31] = 0;
        //pc moved to signal v2
        data[32] = data[6]; data[33] = data[7];
        data[6] = 0; data[7] = 0;
        //hw mode
        data[34] = if self.is128k { 3 } else { 0 };
        //memory map
        data[35] = if self.is128k { self.ula.memory.last_banks() } else { 0 };
        //36
        data[37] = 3 | // R emulation | LDIR emulation 
                   (if !self.is128k && self.ula.psg.is_some() { 4 } else { 0 }); //PSG in 48k
        //psg
        if let Some(ref psg) = self.ula.psg {
            psg.snapshot(&mut data[38..55]);
        }
        if self.is128k && banks_plus2 != 0 {
            data[86] = banks_plus2;
        }

        //memory dump
        fn compress(data: &mut Vec<u8>, index: u8, bank: &[u8]) {
            //length, delayed
            let start = data.len();
            data.push(0); data.push(0);
            data.push(index);

            let mut seq : Option<(u8, u8)> = None;

            for &b in bank {
                seq = match seq {
                    None => {
                        Some((b, 1))
                    }
                    Some((seq_byte, seq_count)) if seq_byte == b && seq_count < 0xff => {
                        Some((b, seq_count + 1))
                    }
                    Some((seq_byte, seq_count)) if seq_count >= 5 || (seq_byte == 0xed && seq_count >= 2) => {
                        data.write_all(&[0xed, 0xed, seq_count, seq_byte]).unwrap();
                        Some((b, 1))
                    }
                    Some((0xed, 1)) => {
                        data.push(0xed);
                        data.push(b);
                        None
                    }
                    Some((seq_byte, seq_count)) => {
                        data.extend(std::iter::repeat(seq_byte).take(seq_count as usize));
                        Some((b, 1))
                    }
                };
            }
            match seq {
                None => {}
                Some((seq_byte, seq_count)) if seq_count >= 5 || (seq_byte == 0xed && seq_count >= 2) => {
                    data.write_all(&[0xed, 0xed, seq_count, seq_byte]).unwrap();
                }
                Some((seq_byte, seq_count)) => {
                    data.extend(std::iter::repeat(seq_byte).take(seq_count as usize));
                }
            }
            
            let len = (data.len() - start - 3) as u16;
            data[start] = len as u8;
            data[start + 1] = (len >> 8) as u8;
        }

        if self.is128k {
            for i in 0..8 {
                let bank = self.ula.memory.get_bank(i as usize);
                compress(&mut data, i + 3, &bank);
            }
        } else {
            for i in 1..4 {
                let bank = self.ula.memory.get_bank(i);
                compress(&mut data, [0, 8, 4, 5][i as usize], &bank);
            }
        }
        data
    }
    pub fn load_snapshot(data: &[u8]) -> io::Result<Game> {
        let data = match snapshot_from_zip(data) {
            Ok(v) => Cow::Owned(v),
            Err(_) => Cow::Borrowed(data),
        };
        let data_z80 = data.get(..34).ok_or(io::ErrorKind::InvalidData)?;
        let (z80, version) = Z80::load_snapshot(&data_z80);
        log!("z80 version {:?}", version);
        let border = PIXELS[0][((data_z80[12] >> 1) & 7) as usize];
        let (hdr, mem) = match version {
            Z80FileVersion::V1 => {
                (&[] as &[u8], data.get(30..).ok_or(io::ErrorKind::InvalidData)?)
            }
            Z80FileVersion::V2 => {
                (data.get(..55).ok_or(io::ErrorKind::InvalidData)?,
                 data.get(55..).ok_or(io::ErrorKind::InvalidData)?)
            }
            Z80FileVersion::V3(false) => {
                (data.get(..86).ok_or(io::ErrorKind::InvalidData)?,
                 data.get(86..).ok_or(io::ErrorKind::InvalidData)?)

            }
            Z80FileVersion::V3(true) => {
                (data.get(..87).ok_or(io::ErrorKind::InvalidData)?,
                 data.get(87..).ok_or(io::ErrorKind::InvalidData)?)
            }
        };
        let is128k = match version {
            Z80FileVersion::V1 => false,
            Z80FileVersion::V2 => {
                match hdr[34] {
                    0 => false,
                    3 => true,
                    _ => true, //if in doubt, assume 128k
                }
            }
            Z80FileVersion::V3(_) => {
                match hdr[34] {
                    0 => false,
                    4 => true,
                    _ => true, //if in doubt, assume 128k
                }
            }
        };
        let psg = if version != Z80FileVersion::V1 && (hdr[37] & 4) != 0 || is128k {
            Some(PSG::load_snapshot(&hdr[38 .. 55]))
        } else {
            None
        };
        match (is128k, &psg) {
            (true, _) => {
                log!("machine = 128k");
            }
            (false, Some(_)) => {
                log!("machine = 48k with PSG");
            }
            (false, None) => {
                log!("machine = 48k");
            }
        }
        /*
        let mut offset = match version {
            Z80FileVersion::V1 => 30,
            Z80FileVersion::V2 => 32 + 23,
            Z80FileVersion::V3(false) => 32 + 54,
            Z80FileVersion::V3(true) => 32 + 55,
        };*/
        let mut memory = if is128k {
            let mut m = Memory::new_from_bytes(ROM_128_0, Some(ROM_128_1));
            //port 0x7ffd
            m.switch_banks(hdr[35]);
            if version == Z80FileVersion::V3(true) {
                m.switch_banks_plus2(hdr[86]);
            }
            m
        } else {
            Memory::new_from_bytes(ROM_48, None)
        };

        fn uncompress(cdata: &[u8], bank: &mut [u8]) {
            let mut wbank = bank;
            let mut rdata = cdata.iter();
            let mut prev_ed = false;
            while let Some(&b) = rdata.next() {
                prev_ed = match (prev_ed, b) {
                    (true, 0xed) => {
                        let times = *rdata.next().unwrap();
                        let value = *rdata.next().unwrap();
                        wbank.write_all(&vec![value; times as usize]).unwrap();
                        false
                    }
                    (false, 0xed) => {
                        true
                    }
                    (true, b) => {
                        wbank.write_all(&[0xed, b]).unwrap();
                        false
                    }
                    (false, b) => {
                        wbank.write_all(&[b]).unwrap();
                        false
                    }
                }
            }
            if prev_ed {
                wbank.write_all(&[0xed]).unwrap();
            }
            if !wbank.is_empty() {
                log!("Warning: uncompressed page misses {} bytes", wbank.len());
            }
        }

        match version {
            Z80FileVersion::V1 => {
                let compressed = (data_z80[12] & 0x20) != 0;
                let ram = if compressed {
                    let mut fullmem = vec![0; 0xc000];
                    //the signature is 4 bytes long
                    let sig = mem.get(mem.len() - 4..).ok_or(io::ErrorKind::InvalidData)?;
                    if sig != [0, 0xed, 0xed, 0] {
                        return Err(io::ErrorKind::InvalidData.into());
                    }
                    let cdata = mem.get(.. mem.len() - 4).ok_or(io::ErrorKind::InvalidData)?;
                    uncompress(cdata, &mut fullmem);
                    Cow::Owned(fullmem)
                } else {
                    //is there a signature in uncompressed memory?
                    Cow::Borrowed(mem)
                };
                for (ibank, blockmem) in ram.chunks_exact(0x4000).enumerate() {
                    let bank = memory.get_bank_mut(ibank + 1);
                    bank.copy_from_slice(blockmem)
                }
            }
            _ => {
                let mut offset = 0;
                while offset < mem.len() - 3 {
                    let memlen = usize::from(u16::from(mem[offset]) | (u16::from(mem[offset + 1]) << 8));
                    let memlen = std::cmp::min(memlen, 0x4000);
                    let compressed = memlen < 0x4000;
                    let page = mem[offset + 2];
                    offset += 3;
                    let cdata = mem.get(offset .. offset + memlen).ok_or(io::ErrorKind::InvalidData)?;
                    offset += memlen;

                    let ibank = match (is128k, page) {
                        (false, 8) => 1,
                        (false, 4) => 2,
                        (false, 5) => 3,
                        (true, 3) => 0,
                        (true, 4) => 1,
                        (true, 5) => 2,
                        (true, 6) => 3,
                        (true, 7) => 4,
                        (true, 8) => 5,
                        (true, 9) => 6,
                        (true, 10) => 7,
                        _ => {
                            return Err(io::ErrorKind::InvalidData.into());
                        }
                    };
                    log!("MEM {:02x}: {:04x}", ibank, memlen);
                    let bank = memory.get_bank_mut(ibank);
                    if compressed {
                        uncompress(cdata, bank);
                    } else {
                        bank.copy_from_slice(cdata);
                    }
                }
            }
        }
        let game = Game {
            is128k,
            z80,
            ula: ULA {
                memory,
                keys: Default::default(),
                delay: 0,
                frame_counter: 0,
                time: 0,
                tape: None,
                border,
                ear: false,
                mic: false,
                psg,
            },
            image: vec![PIXELS[0][0]; (BX0 + 256 + BX1) * (BY0 + 192 + BY1)], //256x192 plus border
            speaker: Speaker::new(),
        };
        Ok(game)
    }
}

cfg_if! {
    if #[cfg(feature="zip")] {
        fn snapshot_from_zip(data: &[u8]) -> io::Result<Vec<u8>> {
            use std::io::Read;

            let rdr = Cursor::new(data);
            let mut zip = zip::ZipArchive::new(rdr)?;
            for i in 0 .. zip.len() {
                let mut ze = zip.by_index(i)?;
                let name = ze.sanitized_name();
                let ext = name.extension().
                    and_then(|e| e.to_str()).
                    map(|e| e.to_string()).
                    map(|e| e.to_ascii_lowercase());
                if let Some("z80") = ext.as_ref().map(|s| s.as_str()) {
                    log!("unzipping Z80 {}", name.to_string_lossy());
                    let mut res = Vec::new();
                    ze.read_to_end(&mut res)?;
                    return Ok(res);
                }
            }
            Err(io::ErrorKind::InvalidData.into())
        }
    } else {
        fn snapshot_from_zip(_data: &[u8]) -> io::Result<Vec<u8>> {
            Err(io::ErrorKind::NotFound.into())
        }
    }
}
