use std::io::{prelude::*, self};
use std::borrow::Cow;

#[derive(Copy, Clone)]
struct Tone {
    num: u32,
    len1: u32,
    len2: u32,
}

#[derive(Copy, Clone, Debug)]
enum Duration {
    Infinite,
    T(u32),
}

impl Duration {
    fn zero() -> Duration {
        Duration::T(0)
    }
}

#[derive(Clone)]
struct Block {
    name: Option<String>,
    selectable: bool,
    tones: Vec<Tone>,
    len_zero: u32,
    len_one: u32,
    bits_last: u8,
    pause: Duration,
    data: Vec<u8>,
}

impl Block {
    fn standard_data_block(data: Vec<u8>) -> Block {
        let num_pilots = if *data.first().unwrap_or(&0) < 0x80 { 8063 } else { 3223 };
        Self::turbo_data_block(2168, num_pilots, 667, 735, 855, 1710, 8, 3_500_000, data)
    }
    fn turbo_data_block(len_pilot: u32, num_pilots: u32, len_sync1: u32, len_sync2: u32,
                        len_zero: u32, len_one: u32, bits_last: u8, pause: u32, data: Vec<u8>) -> Block {
        let mut tones = Vec::new();
        //num_pilots counts the half pulses, so divide by 2
        //If num_pilots is odd, add a pair. The proper thing to do would be to add a half tone
        //but then the levels should get inverted and that is not implemented yet.
        //pilot
        tones.push(Tone { num: (num_pilots + 1) / 2, len1: len_pilot, len2: len_pilot});
        //sync1
        tones.push(Tone { num: 1, len1: len_sync1, len2: 0});
        //sync2
        tones.push(Tone { num: 1, len1: 0, len2: len_sync2});
        Block {
            name: None,
            selectable: true,
            tones,
            len_zero,
            len_one,
            bits_last,
            pause: Duration::T(pause),
            data,
        }
    }
    fn pure_data_block(len_zero: u32, len_one: u32, bits_last: u8, pause: u32, data: Vec<u8>) -> Block {
        Self::turbo_data_block(0, 0, 0, 0, len_zero, len_one, bits_last, pause, data)
    }
    fn pure_tone_block(len_tone: u32, num_tones: u32) -> Block {
        let mut tones = Vec::new();
        if num_tones % 2 != 0 {
            tones.push(Tone { num: 1, len1: 0, len2: len_tone});
        }
        tones.push(Tone { num: num_tones / 2, len1: len_tone, len2: len_tone});
        Block {
            name: None,
            selectable: false,
            tones,
            len_zero: 0,
            len_one: 0,
            bits_last: 0,
            pause: Duration::zero(),
            data: Vec::new(),
        }
    }
    fn single_tone_block(len1: u32, len2: u32) -> Block {
        Block {
            name: None,
            selectable: false,
            tones: vec![Tone { num: 1, len1, len2 }],
            len_zero: 0,
            len_one: 0,
            bits_last: 0,
            pause: Duration::zero(),
            data: Vec::new(),
        }
    }
    fn pause_block(pause: u32) -> Block {
        Block {
            name: None,
            selectable: false,
            tones: Vec::new(),
            len_zero: 0,
            len_one: 0,
            bits_last: 0,
            pause: Duration::T(pause),
            data: Vec::new(),
        }
    }
    fn stop_block() -> Block {
        Block {
            name: Some("stop".to_string()),
            selectable: true,
            tones: Vec::new(),
            len_zero: 0,
            len_one: 0,
            bits_last: 0,
            pause: Duration::Infinite,
            data: Vec::new(),
        }
    }

    fn start() -> TapePhaseT {
        TapePhaseT(Duration::zero(), TapePhase::Start)
    }
    fn tones(&self, index: usize, pulse: u32, last_half: bool) -> TapePhaseT {
        if index >= self.tones.len() {
            self.data_bit(0, 0, false)
        } else {
            let tone = &self.tones[index];
            let len = if last_half { tone.len2 } else { tone.len1 };
            TapePhaseT(Duration::T(len), TapePhase::Tones { index, pulse, last_half })
        }
    }
    fn data_bit(&self, pos: usize, bit: u8, last_half: bool) -> TapePhaseT {
        if pos >= self.data.len() {
            self.pause()
        } else {
            let byte = self.data[pos];
            let v = byte & (0x80 >> bit) != 0;
            let len = if v { self.len_one } else { self.len_zero };
            TapePhaseT(Duration::T(len), TapePhase::Data { pos, bit, last_half })
        }
    }
    fn pause(&self) -> TapePhaseT {
        TapePhaseT(self.pause, TapePhase::Pause)
    }
}

pub struct Tape {
    blocks: Vec<Block>
}

fn read_u8(r: &mut impl Read) -> io::Result<u8> {
    let mut b = 0;
    r.read_exact(std::slice::from_mut(&mut b))?;
    Ok(b)
}
fn read_u16(r: &mut impl Read) -> io::Result<u16> {
    let mut bs = [0; 2];
    r.read_exact(&mut bs)?;
    Ok((u16::from(bs[0])) | (u16::from(bs[1]) << 8))
}
fn read_u32(r: &mut impl Read) -> io::Result<u32> {
    let l = u32::from(read_u16(r)?);
    let h = u32::from(read_u16(r)?);
    Ok(l | (h << 16))
}
fn read_vec(r: &mut impl Read, n: usize) -> io::Result<Vec<u8>> {
    let mut data = vec![0; n];
    r.read_exact(&mut data)?;
    Ok(data)
}
fn latin1_to_string(s: &[u8]) -> String {
    s.iter().map(|&c| c as char).collect()
}
fn read_string(r: &mut impl Read, n: usize) -> io::Result<String> {
    let bs = read_vec(r, n)?;
    Ok(latin1_to_string(&bs))
}

cfg_if! {
    if #[cfg(feature="zip")] {
        fn new_zip<R: Read + Seek>(r: &mut R, is128k: bool) -> io::Result<Vec<Block>> {
            let mut zip = zip::ZipArchive::new(r)?;

            for i in 0 .. zip.len() {
                let mut ze = zip.by_index(i)?;
                let name = ze.name();
                let name_l = name.to_ascii_lowercase();
                if name_l.ends_with(".tap") {
                    log!("unzipping TAP {}", name);
                    return new_tap(&mut ze);
                } else if name_l.ends_with(".tzx") {
                    log!("unzipping TZX {}", name);
                    return new_tzx(&mut ze, is128k);
                }
            }
            Err(io::ErrorKind::InvalidData.into())
        }
    } else {
        fn new_zip<R: Read + Seek>(_r: &mut R, _is128k: bool) -> io::Result<Vec<Block>> {
            Err(io::ErrorKind::NotFound.into())
        }
    }
}

fn new_tap(r: &mut impl Read) -> io::Result<Vec<Block>> {
    let mut blocks = Vec::new();
    loop {
        let len = match read_u16(r) {
            Ok(x) => x,
            Err(ref e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e),
        };
        let mut data = vec![0; usize::from(len)];
        r.read_exact(&mut data)?;
        blocks.push(Block::standard_data_block(data));
    }
    Ok(blocks)
}

fn new_tzx(r: &mut impl Read, is128k: bool) -> io::Result<Vec<Block>> {
    let mut sig = [0; 10];
    r.read_exact(&mut sig)?;
    if &sig[0..8] != b"ZXTape!\x1a" {
        return Err(io::ErrorKind::InvalidData.into());
    }
    let major = sig[8];
    let minor = sig[9];
    log!("tzx version: {}.{}", major, minor);

    #[derive(Clone)]
    enum GroupParse {
        First(String),
        Middle(String),
        SingleBlockName(String),
    };
    struct Parser {
        blocks: Vec<Block>,
        loop_start: Option<(usize, u16)>,
        group_name: Option<GroupParse>,
    }
    impl Parser {
        fn add_block(&mut self, mut block: Block) {
            self.group_name = match self.group_name.take() {
                Some(GroupParse::First(n)) => {
                    block.name = Some(n.clone());
                    block.selectable = true;
                    Some(GroupParse::Middle(n))
                }
                Some(GroupParse::Middle(n)) => {
                    block.name = Some(n.clone());
                    block.selectable = false;
                    Some(GroupParse::Middle(n))
                }
                Some(GroupParse::SingleBlockName(n)) => {
                    block.name = Some(n);
                    block.selectable = true;
                    None
                }
                None => None,
            };
            self.blocks.push(block);
        }
        fn group_start(&mut self, text: String) {
            if self.in_group() {
                log!("nested group not allowed");
            } else {
                self.group_name = Some(GroupParse::First(text));
            }
        }
        fn group_end(&mut self) {
            if self.in_group() {
                self.group_name = None;
            } else {
                log!("group end without start");
            }
        }
        fn in_group(&self) -> bool {
            match &self.group_name {
                None | Some(GroupParse::SingleBlockName(_)) => {
                    false
                }
                Some(GroupParse::First(_)) | Some(GroupParse::Middle(_)) => {
                    true
                }
            }
        }
        fn text_description(&mut self, text: String) {
            if self.in_group() {
                log!("text description inside a group");
            } else {
                self.group_name = Some(GroupParse::SingleBlockName(text));
            }
        }
        fn loop_start(&mut self, reps: u16) {
            if self.loop_start.is_some() {
                log!("nested loop");
            } else {
                self.loop_start = Some((self.blocks.len(), reps));
            }
        }
        fn loop_end(&mut self) {
            match self.loop_start.take() {
                Some((start, repetitions)) => {
                    let end = self.blocks.len();
                    for _ in 0..repetitions {
                        for i in start .. end {
                            let mut new_block = self.blocks[i].clone();
                            new_block.name = None;
                            new_block.selectable = false;
                            self.add_block(new_block);
                        }
                    }
                }
                None => {
                    log!("loop end without start");
                }
            }
        }
    }

    let mut parser = Parser {
        blocks: Vec::new(),
        loop_start: None,
        group_name: None,
    };

    loop {
        let kind = match read_u8(r) {
            Ok(b) => b,
            Err(ref e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e),
        };
        match kind {
            0x10 => { //standard speed data block
                let pause = u32::from(read_u16(r)?) * 3500; // ms -> T
                let block_len = read_u16(r)?;
                let data = read_vec(r, usize::from(block_len))?;
                log!("standard block P:{} D:{}", pause as f32 / 3_500_000.0, data.len());
                let mut block = Block::standard_data_block(data);
                block.pause = Duration::T(pause);
                parser.add_block(block);
            }
            0x11 => { //turbo speed data block
                let len_pilot = u32::from(read_u16(r)?);
                let len_sync1 = u32::from(read_u16(r)?);
                let len_sync2 = u32::from(read_u16(r)?);
                let len_zero = u32::from(read_u16(r)?);
                let len_one = u32::from(read_u16(r)?);
                let num_pilots = u32::from(read_u16(r)?);
                let bits_last = read_u8(r)?;
                let pause = u32::from(read_u16(r)?) * 3500; // ms -> T
                let num0 = usize::from(read_u16(r)?);
                let num1 = usize::from(read_u8(r)?);
                let num = num0 | (num1 << 16);
                let data = read_vec(r, num)?;
                log!("turbo speed data block P:{}*{} S1:{} S2:{} 0:{} 1:{} L:{} P:{} D:{}",
                         len_pilot, num_pilots,
                         len_sync1, len_sync2,
                         len_zero, len_one,
                         bits_last,
                         pause as f32 / 3_500_000.0, num);
                let block = Block::turbo_data_block(len_pilot, num_pilots, len_sync1, len_sync2,
                                                        len_zero, len_one, bits_last, pause, data);
                parser.add_block(block);
            }
            0x12 => { //pure tone
                let len_tone = u32::from(read_u16(r)?);
                let num_tones = u32::from(read_u16(r)?);
                log!("pure tone {} {}", len_tone, num_tones);
                let block = Block::pure_tone_block(len_tone, num_tones);
                parser.add_block(block);
            }
            0x13 => { //pulse sequence
                let num = read_u8(r)?;
                let mut pulses = Vec::with_capacity(usize::from(num));
                for _ in 0..num {
                    pulses.push(read_u16(r)?);
                }
                log!("pulse sequence {:?}", pulses);
                for p in pulses.chunks(2) {
                    if p.len() == 2 {
                        let block = Block::single_tone_block(u32::from(p[0]), u32::from(p[1]));
                        parser.add_block(block);
                    } else {
                        log!("odd pulse sequence unimplemented");
                    }
                }
            }
            0x14 => { //pure data block
                let len_zero = u32::from(read_u16(r)?);
                let len_one = u32::from(read_u16(r)?);
                let bits_last = read_u8(r)?;
                let pause = u32::from(read_u16(r)?) * 3500; // ms -> T;
                let num0 = usize::from(read_u16(r)?);
                let num1 = usize::from(read_u8(r)?);
                let num = num0 | (num1 << 16);
                let data = read_vec(r, num)?;
                log!("pure data block 0:{} 1:{} L:{} P:{} D:{}",
                     len_zero, len_one, bits_last,
                     pause as f32 / 3_500_000.0, num);
                let block = Block::pure_data_block(len_zero, len_one, bits_last, pause, data);
                parser.add_block(block);
            }
            //0x15 => {} //direct recording
            //0x16 | 0x17 => {} //C64?
            //0x18 => {} //CSW Recording
            //0x19 => {} //generalized data block
            0x20 => { //pause (stop)
                let pause = u32::from(read_u16(r)?) * 3500; // ms -> T;
                if pause == 0 {
                    log!("stop tape");
                    let block = Block::stop_block();
                    parser.add_block(block);
                } else {
                    log!("pause {}", pause);
                    let block = Block::pause_block(pause);
                    parser.add_block(block);
                }
            }
            0x21 => { //group start
                let len = read_u8(r)?;
                let text = read_string(r, usize::from(len))?;
                log!("group start: {}", text);
                parser.group_start(text);
            }
            0x22 => { //group end
                log!("group end");
                parser.group_end();
            }
            //0x23 => {} //jump to block
            0x24 => { //loop start
                let repetitions = read_u16(r)?;
                log!("loop start {}", repetitions);
                parser.loop_start(repetitions);
            }
            0x25 => { //loop end
                log!("loop end");
                parser.loop_end();
            }
            //0x26 => {} //call sequence
            //0x27 => {} //return from sequence
            //0x28 => {} //select block
            0x2a => { //stop the tape if in 48K mode
                let len = read_u32(r)?;
                if len > 0 {
                    return Err(io::ErrorKind::InvalidData.into());
                }
                log!("stop tape if 48k");
                if !is128k {
                    let block = Block::stop_block();
                    parser.add_block(block);
                }
            }
            //0x2b => {} //set signal level
            0x30 => { //text description
                let len = read_u8(r)?;
                let text = read_string(r, usize::from(len))?;
                log!("text description: {}", text);
                parser.text_description(text);
            }
            //0x31 => {} //message block
            0x32 => { //archive info
                let len = read_u16(r)?;
                let info = read_vec(r, usize::from(len))?;
                let ri = &mut info.as_slice();
                let num = read_u8(ri)?;
                for _i in 0..num {
                    let id = read_u8(ri)?;
                    let ilen = read_u8(ri)?;
                    let itext = read_string(ri, usize::from(ilen))?;
                    log!("archive info {:02x}: {}", id, itext);
                }
            }
            //0x33 => {} //hardware type
            //0x34 => {} //emulation info
            //0x35 => {} //custom info block
            //0x40 => {} //snapshot block
            //0x5a => {} //glue block
            x => {
                log!("*** unknown chunk type: 0x{:02x}", x);
            }
        }
    }

    Ok(parser.blocks)
}

static SPECTRUM_ENCODING : [&str; 0x100] = [
/* 0 */ "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "",
/* 1 */ "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "",
/* 2 */ " ", "!", "\"", "#", "$", "%", "&", "'", "(", ")", "*", "+", ",", "-", ".", "/",
/* 3 */ "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", ":", ";", "<", "=", ">", "?",
/* 4 */ "@", "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O",
/* 5 */ "P", "Q", "R", "S", "T", "U", "V", "W", "X", "Y", "Z", "[", "\\", "]", "↑", "_",
/* 6 */ "£", "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o",
/* 7 */ "p", "q", "r", "s", "t", "u", "v", "w", "x", "y", "z", "{", "|", "}", "~", "©",
/* 8 */ " ", "▝", "▘", "▀", "▗", "▐", "▚", "▜", "▖", "▞", "▌", "▛", "▄", "▟", "▙", "█",
/* 9 */ "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P",
/* A */ "Q", "R", "S", "T", "U", "RND", "INKEY$", "PI",
        "FN", "POINT", "SCREEN$", "ATTR", "AT", "TAB", "VAL$", "CODE",
/* B */ "VAL", "LEN", "SIN", "COS", "TAN", "ASN", "ACS", "ATN",
        "LN", "EXP", "INT", "SQR", "SGN", "ABS", "PEEK", "IN",
/* C */ "USR", "STR$", "CHR$", "NOT", "BIN", "OR", "AND", "<=",
        ">=", "<>", "LINE", "THEN", "TO", "STEP", "DEF FN", "CAT",
/* D */ "FORMAT", "MOVE", "ERASE", "OPEN #", "CLOSE #", "MERGE", "VERIFY", "BEEP",
        "CIRCLE", "INK", "PAPER", "FLASH", "BRIGHT", "INVERSE", "OVER", "OUT",
/* E */ "LPRINT", "LLIST", "STOP", "READ", "DATA", "RESTORE", "NEW", "BORDER",
        "CONTINUE", "DIM", "REM", "FOR", "GO TO", "GO SUB", "INPUT", "LOAD",
/* F */ "LIST", "LET", "PAUSE", "NEXT", "POKE", "PRINT", "PLOT", "RUN",
        "SAVE", "RANDOMIZE", "IF", "CLS", "DRAW", "CLEAR", "RETURN", "COPY",
];

fn string_from_zx(bs: &[u8]) -> String {
    let mut s = String::new();
    log!("{:?}", bs);
    for &b in bs {
        s += SPECTRUM_ENCODING[usize::from(b)];
    }
    s
}

impl Tape {
    pub fn new<R: Read + Seek>(mut tap: R, is128k: bool) -> io::Result<Tape> {
        let start_pos = tap.seek(io::SeekFrom::Current(0))?;

        let mut blocks = new_zip(tap.by_ref(), is128k)
        .or_else(|_| {
            tap.seek(io::SeekFrom::Start(start_pos))?;
            new_tzx(tap.by_ref(), is128k)
        }).or_else(|_| {
            tap.seek(io::SeekFrom::Start(start_pos))?;
            new_tap(tap.by_ref())
        })?;

        //try to guess the names of the unnamed blocks
        let mut prefixed = false;
        for block in blocks.iter_mut() {
            //header block
            let name = if block.data.len() == 0x13 && block.data[0] == 0 {
                let block_type = match block.data[1] {
                    0 => Cow::Borrowed("Program"),
                    1 => Cow::Borrowed("Array"),
                    3 => Cow::Borrowed("Bytes"),
                    x => Cow::Owned(format!("Type {}", x)),
                };
                let block_name = string_from_zx(&block.data[2..12]);
                prefixed = true;
                Some(format!("{}: {}", block_type, block_name))
            } else {
                if prefixed {
                    prefixed = false;
                    //let the user select the header, this one is not so useful
                    block.selectable = false;
                }
                Some(format!("{} bytes", block.data.len()))
            };
            if block.name.is_none() {
                block.name = name;
            }
        }
        Ok(Tape{ blocks } )
    }
    pub fn play(&self, mut d: u32, pos: TapePos) -> Option<TapePos> {
        let TapePos { mut block, mut phase } = pos;

        while d > 0 {
            if block >= self.blocks.len() {
                return None;
            }
            phase = match phase.next(&mut d, self, block) {
                Some(n) => n,
                None => {
                    block += 1;
                    Block::start()
                }
            };
        }
        Some(TapePos{ block, phase })
    }
    pub fn len(&self) -> usize {
        self.blocks.len()
    }
    pub fn block_name(&self, index: usize) -> &str {
        self.blocks[index].name.as_ref().map(|s| s.as_ref()).unwrap_or("")
    }
    pub fn block_selectable(&self, index: usize) -> bool {
        self.blocks[index].selectable
    }
}

#[derive(Debug)]
pub enum TapePhase {
    Start,
    Tones { index: usize, pulse: u32, last_half: bool },
    Data { pos: usize, bit: u8, last_half: bool }, //2 * 855 T or 1710 T
    Pause,
}

//The phase and remaining Tstates
#[derive(Debug)]
struct TapePhaseT(Duration, TapePhase);

impl TapePhaseT {
    fn mic(&self) -> bool {
        match self.1 {
            TapePhase::Start => false,
            TapePhase::Tones { last_half, .. } => last_half,
            TapePhase::Data { last_half, .. } => last_half,
            TapePhase::Pause => false,
        }
    }
    fn next(self, d: &mut u32, tape: &Tape, iblock: usize) -> Option<TapePhaseT> {
        let TapePhaseT(duration, phase) = self;

        match duration {
            Duration::Infinite => {
                *d = 0;
                return Some(TapePhaseT(Duration::Infinite, phase));
            }
            Duration::T(time) => {
                if time > *d {
                    let tnext = time - *d;
                    *d = 0;
                    return Some(TapePhaseT(Duration::T(tnext), phase));
                }
                *d -= time;
            }
        }

        let block = &tape.blocks[iblock];

        let TapePhaseT(mut dnext, rphase) = match phase {
            TapePhase::Start => {
                block.tones(0, 0, false)
            }
            TapePhase::Tones { index, pulse, last_half } => {
                if !last_half {
                    block.tones(index, pulse, true)
                } else {
                    let pulse = pulse + 1;
                    if pulse < block.tones[index].num {
                        block.tones(index, pulse, false)
                    } else {
                        block.tones(index + 1, 0, false)
                    }
                }
            }
            TapePhase::Data { pos, bit, last_half } => {
                if !last_half {
                    block.data_bit(pos, bit, true)
                } else {
                    let bit = bit + 1;
                    let bit_len = if pos == block.data.len() - 1 { block.bits_last } else { 8 };
                    if bit < bit_len {
                        block.data_bit(pos, bit, false)
                    } else {
                        block.data_bit(pos + 1, 0, false)
                    }
                }
            }
            TapePhase::Pause => {
                return None;
            }
        };
        match dnext {
            Duration::Infinite => {
                *d = 0;
            }
            Duration::T(ref mut time) => {
                if *time > *d {
                    *time -= *d;
                    *d = 0;
                } else {
                    *time = 0;
                    *d -= *time;
                }
            }
        }
        Some(TapePhaseT(dnext, rphase))
    }
}

pub struct TapePos {
    block: usize,
    phase: TapePhaseT,
}

impl TapePos {
    pub fn new_at_block(block: usize) -> TapePos {
        TapePos { block, phase: Block::start() }
    }
    pub fn mic(&self) -> bool {
        self.phase.mic()
    }
    pub fn block(&self, tape: &Tape) -> usize {
        let mut res = self.block;
        if res >= tape.blocks.len() {
            return 0xffff_ffff;
        }
        while !tape.blocks[res].selectable && res > 0 {
            res -= 1;
        }
        res
    }
}
