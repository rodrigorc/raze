use crate::memory::Memory;
use crate::psg::Psg;
use crate::rzx;
use crate::speaker::Speaker;
use crate::tape::{Tape, TapePos};
use crate::z80::{self, Bus, Z80, Z80FileVersion};
use anyhow::{Result, anyhow};
use std::borrow::Cow;
use std::io::{Cursor, Read, Write};

const TIME_TO_INT: i32 = 69888;

static ROM_128_0: &[u8] = include_bytes!("128-0.rom");
static ROM_128_1: &[u8] = include_bytes!("128-1.rom");
static ROM_48: &[u8] = include_bytes!("48k.rom");

//margins
const BX0: usize = 5;
const BX1: usize = 5;
const BY0: usize = 4;
const BY1: usize = 4;

//256x192 plus border
const SCREEN_WIDTH: usize = BX0 + 256 + BX1;
const SCREEN_HEIGHT: usize = BY0 + 192 + BY1;
const SCREEN_SIZE: usize = SCREEN_WIDTH * SCREEN_HEIGHT;

fn black_screen<PIX: Copy>(palette: &[[PIX; 8]; 2]) -> [PIX; SCREEN_SIZE] {
    [palette[0][0]; SCREEN_SIZE]
}

struct RzxInfo {
    frames: Vec<rzx::InputFrame>,
    frame_idx: usize,
    frame_data_idx: usize,
    in_idx: usize,
}

struct Ula {
    memory: Memory,
    keys: [u8; 9], //8 semirows plus joystick
    delay: u32,
    frame_counter: u32,
    time: i32,
    tape: Option<(Tape, Option<TapePos>)>,
    border: u8,
    ear: bool,
    mic: bool,
    psg: Option<Psg>,
    fetch_count: u32,
    rzx_info: Option<RzxInfo>,
}

impl Ula {
    pub fn take_delay(&mut self) -> u32 {
        let r = self.delay;
        self.delay = 0;
        r
    }
    pub fn add_time(&mut self, t: u32, gui: &mut impl Gui) {
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
                    gui.on_tape_block(index_post);
                }
                Some((tape, next))
            }
            tape => tape,
        };
    }
    fn audio_sample(&mut self, t: u32) -> u32 {
        let v = if self.ear { 0x2000 } else { 0 } + if self.mic { 0x1000 } else { 0 };
        match &mut self.psg {
            None => v,
            Some(psg) => v + u32::from(psg.next_sample(t)),
        }
    }
    fn has_to_interrupt(&self) -> bool {
        if let Some(rzx) = &self.rzx_info {
            let frame = &rzx.frames[rzx.frame_idx];
            self.fetch_count >= frame.fetch_count as u32
        } else {
            self.time >= TIME_TO_INT
        }
    }
    fn update_time_after_exec(&mut self, t: &mut u32, gui: &mut impl Gui) {
        //contended memory and IO
        let delay_m = self.memory.take_delay();
        let delay_io = self.take_delay();
        if self.time >= 224 * 64 && self.time < 224 * 256 && self.time % 224 < 128 {
            //each row is 224 T, 128 are the real pixels where contention occurs
            //we ignore the delay pattern (6,5,4,3,2,1,0,0) and instead do an
            //estimation
            match delay_m + delay_io {
                0 => (),
                1 => *t += 4,         //only 1 contention: these many Ts on average
                x => *t += 6 * x - 2, //more than 1 contention: they use to chain so max up all but the first one
            }
        }
        self.add_time(*t, gui);
    }

    fn post_interrupt(&mut self, gui: &mut impl Gui) {
        if let Some(rzx) = &mut self.rzx_info {
            self.fetch_count = 0;
            self.time = 0;

            let total_frames = rzx.frames.len();
            rzx.frame_idx += 1;
            if rzx.frame_idx >= total_frames {
                self.rzx_info = None;
                gui.on_rzx_running(false, 0);
                return;
            }
            rzx.in_idx = 0;
            let frame = &rzx.frames[rzx.frame_idx];
            if matches!(frame.in_values, rzx::InValues::Data(_)) {
                rzx.frame_data_idx = rzx.frame_idx;
            }
            gui.on_rzx_running(
                true,
                (rzx.frame_idx * 100).checked_div(total_frames).unwrap_or(0) as u32,
            );
        } else {
            //we drag the excess T to the next loop
            self.time -= TIME_TO_INT;
        }
    }
}

impl Bus for Ula {
    fn peek(&mut self, addr: impl Into<u16>) -> u8 {
        self.memory.peek(addr)
    }
    fn poke(&mut self, addr: impl Into<u16>, value: u8) {
        self.memory.poke(addr, value);
    }
    fn do_in(&mut self, port: impl Into<u16>) -> u8 {
        if let Some(rzx) = &mut self.rzx_info {
            let frame = &rzx.frames[rzx.frame_data_idx];
            let b = match &frame.in_values {
                rzx::InValues::RepeatLast => {
                    log::error!("rzx repeatlast?");
                    0 //should not happen
                }
                rzx::InValues::Data(d) => {
                    if rzx.in_idx < d.len() {
                        let b = d[rzx.in_idx];
                        rzx.in_idx += 1;
                        b
                    } else {
                        log::error!(
                            "rzx in underflow {}/{}: {}<{}",
                            rzx.frame_idx,
                            rzx.frame_data_idx,
                            rzx.in_idx,
                            d.len()
                        );
                        rzx.in_idx += 1;
                        0 //should not happen
                    }
                }
            };
            return b;
        }
        let port = port.into();
        let lo = port as u8;
        let hi = (port >> 8) as u8;
        let mut r = 0xff;
        //ULA IO port
        if lo & 1 == 0 {
            self.delay += 1;
            if (0x4000..0x8000).contains(&port) {
                self.delay += 1;
            }
            for i in 0..8 {
                //half row keyboard
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
            if (0x4000..0x8000).contains(&port) {
                self.delay += 4;
            }
            match lo {
                0xfd => {
                    //Programmable Sound Generator
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
                0xff => {
                    //reads stale data from the floating bus (last attr byte?)
                    let row = self.time / 224;
                    let ofs = self.time % 224;
                    r = if (64..256).contains(&row) && ofs < 128 {
                        let row = row - 64;
                        let ofs = ofs / 8 * 2 + 1; //attrs are read in pairs each 8 T, more or less
                        let addr = 32 * 192 + 32 * (row / 8) + ofs;
                        self.memory.video_memory()[addr as usize]
                    } else {
                        //borders or retraces
                        0xff
                    }
                }
                x if x & 0x20 == 0 => {
                    //kempston joystick (0x1f | 0xdf ...)
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
            if (0x4000..0x8000).contains(&port) {
                self.delay += 1;
            }
            self.border = value & 7;
            self.ear = (value & 0x10) != 0;
            self.mic = (value & 0x08) != 0;
        } else {
            //log::info!("OUT {:04x}, {:02x}", port, value);
            if (0x4000..0x8000).contains(&port) {
                self.delay += 4;
            }
            #[allow(clippy::single_match)]
            match lo {
                0xfd => {
                    //128 stuff
                    match hi {
                        0x7f => {
                            //Memory banks
                            //log!("MEM {:04x}, {:02x}", port, value);
                            self.memory.switch_banks(value);
                        }
                        0x1f => {
                            //+2 Memory banks
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
                        hi if (hi & 0x80) == 0 => {
                            //same as 0x7f
                            //log!("MEM {:04x}, {:02x}", port, value);
                            self.memory.switch_banks(value);
                        }
                        hi if (hi & 0xf0) == 0x10 => {
                            //same as 0x1f
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
    fn inc_fetch_count(&mut self, reason: z80::FetchReason) {
        if reason != z80::FetchReason::Interrupt {
            self.fetch_count += 1;
        }
    }
}

pub trait Gui {
    type Pixel: Copy;

    //Palette of colors: 2 intensities, each with 8 basic colors
    fn palette(&self) -> &[[Self::Pixel; 8]; 2];
    fn on_rzx_running(&mut self, running: bool, percent: u32);
    fn on_tape_block(&mut self, index: usize);
    fn put_sound_data(&mut self, data: &[f32]);
    fn put_image_data(&mut self, w: usize, h: usize, data: &[Self::Pixel]);
}

pub struct Game<GUI: Gui> {
    is128k: bool,
    z80: Z80,
    ula: Ula,
    speaker: Speaker,
    image: [GUI::Pixel; SCREEN_SIZE],
    gui: GUI,
}

fn write_border_row<PIX: Copy>(y: usize, border: PIX, ps: &mut [PIX]) {
    let prow = &mut ps[SCREEN_WIDTH * y..SCREEN_WIDTH * (y + 1)];
    prow.fill(border);
}

fn write_screen_row<PIX: Copy>(
    y: usize,
    border: PIX,
    inv: bool,
    data: &[u8],
    palette: &[[PIX; 8]; 2],
    ps: &mut [PIX],
) {
    let orow = match y {
        0..=63 => (y % 8) * 256 + (y / 8) * 32,
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
        _ => unreachable!(),
    };
    let ym = y + BY0;
    let prow_full = &mut ps[SCREEN_WIDTH * ym..SCREEN_WIDTH * (ym + 1)];
    prow_full[..BX0].fill(border);
    prow_full[BX0 + 256..].fill(border);
    let prow = &mut prow_full[BX0..BX0 + 256];
    let arow = 192 * 32 + (y / 8) * 32;

    //Attributes are 8 bits:
    // b7: blink
    // b6: bright
    // b5-b3: bk color
    // b2-b0: fg color
    //Bitmap and attribute addresses are related in a funny way.
    //Binary values are grouped as octal, clippy doesn't seem to like that.
    #[allow(clippy::unusual_byte_groupings)]
    for ((&bits, &attr), pixels) in data[orow..orow + 32]
        .iter()
        .zip(&data[arow..arow + 32])
        .zip(prow.chunks_mut(8))
    {
        let bright = (attr & 0b01_000_000) != 0;
        let colors = &palette[bright as usize];
        let ink = colors[(attr & 0b00_000_111) as usize];
        let paper = colors[((attr & 0b00_111_000) >> 3) as usize];
        let inv = inv && (attr & 0b10_000_000) != 0;
        let mut bits = if inv { bits ^ 0xff } else { bits };
        pixels.fill_with(|| {
            let on = bits & 0x80 != 0;
            bits <<= 1;
            if on { ink } else { paper }
        });
    }
}

fn write_screen<PIX: Copy>(
    border: PIX,
    palette: &[[PIX; 8]; 2],
    inv: bool,
    data: &[u8],
    ps: &mut [PIX],
) {
    for y in 0..BY0 {
        write_border_row(y, border, ps);
    }
    for y in 0..192 {
        write_screen_row(y, border, inv, data, palette, ps);
    }
    for y in 0..BY1 {
        write_border_row(BY0 + 192 + y, border, ps);
    }
}

// General speed of the emulation is controlled by the audio output.
// The main audio channel should be configured at 22050 Hz, this function will return how many
// CPU ticks are required for every audio sample.
fn t_per_sample(is_128k: bool) -> u32 {
    const SAMPLER: u32 = 22050;
    // CPU freq is slightly different for different models
    let cpu_freq = if is_128k { 3_546_900 } else { 3_500_000 };
    // Round to nearest, that will give a maximum relative error in the emulation speed of:
    // (22.05k / 3.5M / 2) = 0.3%
    // That I think is acceptable. To get exact timings we would need to choose a sample output rate that is an exact division of the
    // CPU freq. But that would require resampling somewhere in the audio pipeline, and that could decrease performance.
    (cpu_freq + SAMPLER / 2) / SAMPLER
}

impl<GUI: Gui> Game<GUI> {
    pub fn new(is128k: bool, mut gui: GUI) -> Game<GUI> {
        log::info!("Go!");
        let memory;
        let psg;
        if is128k {
            memory = Memory::new_from_bytes(ROM_128_0, Some(ROM_128_1));
            psg = Some(Psg::new());
        } else {
            memory = Memory::new_from_bytes(ROM_48, None);
            psg = None;
        };
        let z80 = Z80::new();
        gui.on_rzx_running(false, 0);
        Game {
            is128k,
            z80,
            ula: Ula {
                memory,
                keys: Default::default(),
                delay: 0,
                frame_counter: 0,
                time: 0,
                tape: None,
                border: 0,
                ear: false,
                mic: false,
                psg,
                fetch_count: 0,
                rzx_info: None,
            },
            speaker: Speaker::new(t_per_sample(is128k)),
            image: black_screen(gui.palette()),
            gui,
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
            while !self.ula.has_to_interrupt() {
                let mut t = self.z80.exec(&mut self.ula);
                //self.z80._dump_regs();
                self.ula.update_time_after_exec(&mut t, &mut self.gui);

                if !turbo {
                    let sample = self.ula.audio_sample(t);
                    self.speaker.push_sample(sample, t);
                    //Border is never bright
                    let palette = self.gui.palette();
                    let border = palette[0][self.ula.border as usize];
                    screen_time += t as i32;
                    while screen_time >= 224 {
                        screen_time -= 224;
                        match screen_row {
                            60..=63 | 256..=259 => {
                                write_border_row(screen_row - 60, border, &mut self.image);
                            }
                            64..=255 => {
                                let screen = self.ula.memory.video_memory();
                                write_screen_row(
                                    screen_row - 64,
                                    border,
                                    inverted,
                                    screen,
                                    palette,
                                    &mut self.image,
                                );
                            }
                            _ => {}
                        }
                        screen_row += 1;
                    }
                }
            }
            self.z80.interrupt();
            self.ula.post_interrupt(&mut self.gui);
        }
        if turbo {
            let screen = self.ula.memory.video_memory();
            //Border is never bright
            let palette = self.gui.palette();
            let border = palette[0][self.ula.border as usize];
            write_screen(border, palette, false, screen, &mut self.image);
        } else {
            //adding samples should be rarely necessary, so use lazy generation
            let ula = &mut self.ula;
            let audio = self
                .speaker
                .complete_frame(TIME_TO_INT as u32, || ula.audio_sample(0));
            self.gui.put_sound_data(audio);
            self.speaker.clear();
        }
        self.gui
            .put_image_data(SCREEN_WIDTH, SCREEN_HEIGHT, &self.image);
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
    pub fn peek(&mut self, addr: u16) -> u8 {
        self.ula.memory.peek_no_delay(addr)
    }
    pub fn poke(&mut self, addr: u16, value: u8) {
        self.ula.memory.poke(addr, value);
    }
    pub fn stop_rzx_replay(&mut self) {
        self.ula.rzx_info = None;
        self.gui.on_rzx_running(false, 0);
    }
    pub fn reset_input(&mut self) {
        self.ula.keys = Default::default();
    }
    pub fn tape_load(&mut self, data: Vec<u8>) -> Result<usize> {
        let tape = Tape::new(Cursor::new(data), self.is128k)?;
        let res = tape.len();
        if res > 0 {
            self.ula.tape = Some((tape, Some(TapePos::new_at_block(0))));
        } else {
            self.ula.tape = None;
        }
        Ok(res)
    }
    pub fn tape_name(&self, index: usize) -> &str {
        match &self.ula.tape {
            Some((tape, _)) => tape.block_name(index),
            None => "",
        }
    }
    pub fn tape_selectable(&self, index: usize) -> bool {
        match &self.ula.tape {
            Some((tape, _)) => tape.block_selectable(index),
            None => false,
        }
    }
    pub fn tape_seek(&mut self, index: usize) {
        self.ula.tape = match self.ula.tape.take() {
            Some((tape, _)) => {
                self.gui.on_tape_block(index);
                Some((tape, Some(TapePos::new_at_block(index))))
            }
            None => None,
        }
    }
    pub fn tape_stop(&mut self) {
        self.ula.tape = match self.ula.tape.take() {
            Some((tape, _)) => {
                self.ula.mic = false;
                Some((tape, None))
            }
            None => None,
        }
    }
    pub fn snapshot(&self) -> Vec<u8> {
        //We will save V2 or V3 depending on the plus2 memory bank
        let banks_plus2 = self.ula.memory.last_banks_plus2();

        const HEADER: usize = 32;
        let header_extra = if banks_plus2 == 0 { 23 } else { 55 };
        let mut data = vec![0; HEADER + header_extra];
        self.z80.snapshot(&mut data);
        data[12] |= self.ula.border << 1;

        //extended header block
        //len of the block
        data[30] = header_extra as u8;
        data[31] = 0;
        //pc moved to signal v2
        data[32] = data[6];
        data[33] = data[7];
        data[6] = 0;
        data[7] = 0;
        //hw mode
        data[34] = if self.is128k { 3 } else { 0 };
        //memory map
        data[35] = if self.is128k {
            self.ula.memory.last_banks()
        } else {
            0
        };
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
            data.push(0);
            data.push(0);
            data.push(index);

            let mut seq: Option<(u8, u8)> = None;

            for &b in bank {
                seq = match seq {
                    None => Some((b, 1)),
                    Some((seq_byte, seq_count)) if seq_byte == b && seq_count < 0xff => {
                        Some((b, seq_count + 1))
                    }
                    Some((seq_byte, seq_count))
                        if seq_count >= 5 || (seq_byte == 0xed && seq_count >= 2) =>
                    {
                        data.extend(&[0xed, 0xed, seq_count, seq_byte]);
                        Some((b, 1))
                    }
                    Some((0xed, 1)) => {
                        data.push(0xed);
                        data.push(b);
                        None
                    }
                    Some((seq_byte, seq_count)) => {
                        data.extend(std::iter::repeat_n(seq_byte, seq_count as usize));
                        Some((b, 1))
                    }
                };
            }
            match seq {
                None => {}
                Some((seq_byte, seq_count))
                    if seq_count >= 5 || (seq_byte == 0xed && seq_count >= 2) =>
                {
                    data.extend(&[0xed, 0xed, seq_count, seq_byte]);
                }
                Some((seq_byte, seq_count)) => {
                    data.extend(std::iter::repeat_n(seq_byte, seq_count as usize));
                }
            }

            let len = (data.len() - start - 3) as u16;
            data[start] = len as u8;
            data[start + 1] = (len >> 8) as u8;
        }

        if self.is128k {
            for i in 0..8 {
                let bank = self.ula.memory.get_bank(i);
                // 3 first banks are ROM, do not save those
                compress(&mut data, i as u8 + 3, bank);
            }
        } else {
            for i in 1..4 {
                let bank = self.ula.memory.get_bank(i);
                compress(&mut data, [0, 8, 4, 5][i], bank);
            }
        }
        data
    }
    pub fn load_snapshot(data: &[u8], gui: GUI) -> Result<Game<GUI>> {
        let mut data = match snapshot_from_zip(data) {
            Ok(v) => Cow::Owned(v),
            Err(_) => Cow::Borrowed(data),
        };

        //Check if it is a RZX first
        let rzx = rzx::Rzx::new(&mut data.as_ref()).ok();
        let mut rzx_input = None;

        if let Some(rzx) = rzx {
            for b in rzx.blocks {
                match b {
                    rzx::Block::Snapshot(ss) => {
                        data = Cow::Owned(ss.data);
                    }
                    rzx::Block::Input(input) => {
                        if !input.frames.is_empty() {
                            rzx_input = Some(input.frames);
                        }
                    }
                    _ => {}
                }
            }
        }

        let file_too_short_error = || anyhow!("invalid z80 format: file too short");
        let data_z80 = data.get(..34).ok_or_else(file_too_short_error)?;
        let (z80, version) = Z80::load_snapshot(data_z80)?;
        log::debug!("z80 version {:?}", version);
        let border = (data_z80[12] >> 1) & 7;
        let (hdr, mem) = match version {
            Z80FileVersion::V1 => (
                &[] as &[u8],
                data.get(30..).ok_or_else(file_too_short_error)?,
            ),
            Z80FileVersion::V2 => (
                data.get(..55).ok_or_else(file_too_short_error)?,
                data.get(55..).ok_or_else(file_too_short_error)?,
            ),
            Z80FileVersion::V3(false) => (
                data.get(..86).ok_or_else(file_too_short_error)?,
                data.get(86..).ok_or_else(file_too_short_error)?,
            ),
            Z80FileVersion::V3(true) => (
                data.get(..87).ok_or_else(file_too_short_error)?,
                data.get(87..).ok_or_else(file_too_short_error)?,
            ),
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
        let psg = if (version != Z80FileVersion::V1 && (hdr[37] & 4) != 0) || is128k {
            Some(Psg::load_snapshot(&hdr[38..55]))
        } else {
            None
        };
        match (is128k, &psg) {
            (true, _) => {
                log::debug!("machine = 128k");
            }
            (false, Some(_)) => {
                log::debug!("machine = 48k with PSG");
            }
            (false, None) => {
                log::debug!("machine = 48k");
            }
        }
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

        fn uncompress(cdata: &[u8], bank: &mut [u8]) -> Result<()> {
            let mut wbank = bank;
            let mut rdata = cdata.iter();
            let mut prev_ed = false;
            while let Some(&b) = rdata.next() {
                prev_ed = match (prev_ed, b) {
                    (true, 0xed) => {
                        let times = *rdata
                            .next()
                            .ok_or_else(|| anyhow!("invalid compressed data"))?;
                        let value = *rdata
                            .next()
                            .ok_or_else(|| anyhow!("invalid compressed data"))?;
                        wbank.write_all(&vec![value; times as usize])?;
                        false
                    }
                    (false, 0xed) => true,
                    (true, b) => {
                        wbank.write_all(&[0xed, b])?;
                        false
                    }
                    (false, b) => {
                        wbank.write_all(&[b])?;
                        false
                    }
                }
            }
            if prev_ed {
                wbank.write_all(&[0xed])?;
            }
            if !wbank.is_empty() {
                log::warn!("Warning: uncompressed page misses {} bytes", wbank.len());
            }
            Ok(())
        }

        match version {
            Z80FileVersion::V1 => {
                let compressed = (data_z80[12] & 0x20) != 0;
                let ram = if compressed {
                    let mut fullmem = vec![0; 0xc000];
                    //the signature is 4 bytes long, at the end
                    let sig = mem.get(mem.len() - 4..);
                    if sig != Some(&[0, 0xed, 0xed, 0]) {
                        return Err(anyhow!("invalid Z80v1 signature"));
                    }
                    let cdata = &mem[..mem.len() - 4];
                    uncompress(cdata, &mut fullmem)?;
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
                    let memlen =
                        usize::from(u16::from(mem[offset]) | (u16::from(mem[offset + 1]) << 8));
                    let memlen = std::cmp::min(memlen, 0x4000);
                    let compressed = memlen < 0x4000;
                    let page = mem[offset + 2];
                    offset += 3;
                    let cdata = mem
                        .get(offset..offset + memlen)
                        .ok_or_else(|| anyhow!("invalid compressed memory block"))?;
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
                            return Err(anyhow!("unknown memory page: {} (128k={})", page, is128k));
                        }
                    };
                    log::debug!("MEM {:02x}: {:04x}", ibank, memlen);
                    let bank = memory.get_bank_mut(ibank);
                    if compressed {
                        uncompress(cdata, bank)?;
                    } else {
                        bank.copy_from_slice(cdata);
                    }
                }
            }
        }
        let mut game = Game {
            is128k,
            z80,
            ula: Ula {
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
                fetch_count: 0,
                rzx_info: rzx_input.map(|frames| RzxInfo {
                    frames,
                    frame_idx: 0,
                    frame_data_idx: 0,
                    in_idx: 0,
                }),
            },
            speaker: Speaker::new(t_per_sample(is128k)),
            image: black_screen(gui.palette()),
            gui,
        };
        game.gui.on_rzx_running(game.ula.rzx_info.is_some(), 0);
        Ok(game)
    }
}

#[cfg(feature = "zip")]
fn snapshot_from_zip(data: &[u8]) -> Result<Vec<u8>> {
    let rdr = Cursor::new(data);
    let mut zip = zip::ZipArchive::new(rdr)?;
    for i in 0..zip.len() {
        let mut ze = zip.by_index(i)?;
        let name = ze.name();
        let lowname = name.to_ascii_lowercase();
        if lowname.ends_with(".z80") || lowname.ends_with(".rzx") {
            log::debug!("unzipping Z80 {}", name);
            let mut res = Vec::new();
            ze.read_to_end(&mut res)?;
            return Ok(res);
        }
    }
    Err(anyhow!("ZIP file does not contain any *.z80 or *.rzx file"))
}

#[cfg(not(feature = "zip"))]
fn snapshot_from_zip(_data: &[u8]) -> Result<Vec<u8>> {
    Err(anyhow!("ZIP format not supported"))
}
