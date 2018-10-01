use std::io::{self, Read, Cursor};
use std::borrow::Cow;

struct Block {
    name: String,
    selectable: bool,
    data: Vec<u8>,
}

pub struct Tape {
    blocks: Vec<Block>
}

impl Tape {
    pub fn new(tap: Vec<u8>) -> io::Result<Tape> {
        let mut blocks = Vec::new();
        let mut tap = Cursor::new(tap);

        loop {
            let mut len = [0; 2];
            match tap.read_exact(&mut len) {
                Ok(_) => (),
                Err(_) => break,
            }
            let len = len[0] as usize | ((len[1] as usize) << 8);
            let mut data = vec![0; len];
            tap.read_exact(&mut data)?;
            blocks.push(Block {
                name: String::new(),
                selectable: true,
                data
            });
        }

        //We'll guess the names of the blocks
        let mut prefixed = false;
        for block in blocks.iter_mut() {
            //header block
            block.name = if block.data.len() == 0x13 && block.data[0] == 0 {
                let block_type = match block.data[1] {
                    0 => Cow::from("Program"),
                    1 => Cow::from("Array"),
                    3 => Cow::from("Bytes"),
                    x => Cow::from(format!("Type {}", x)),
                };
                let block_name = String::from_utf8_lossy(&block.data[2..12]);
                prefixed = true;
                format!("{}: {}", block_type, block_name)
            } else {
                if prefixed {
                    prefixed = false;
                    //let the user select the header, this one is not so useful
                    block.selectable = false;
                }
                format!("{} bytes", block.data.len())
            }
        }
        Ok(Tape{ blocks } )
    }
    pub fn play(&self, d: u32, pos: TapePos) -> Option<TapePos> {
        let TapePos { mut block, phase } = pos;
        if block >= self.blocks.len() {
            return None;
        }
        let next = match phase.next(d, self, block) {
            Some(n) => n,
            None => {
                block += 1;
                TapePhaseT::pause()
            }
        };
        Some(TapePos{ block, phase: next })
    }
    pub fn len(&self) -> usize {
        self.blocks.len()
    }
    pub fn block_name(&self, index: usize) -> &str {
        &self.blocks[index].name
    }
    pub fn block_selectable(&self, index: usize) -> bool {
        self.blocks[index].selectable
    }
}

pub enum TapePhase {
    Pause, //500000 T
    Leader { pulse: u32 }, //8063 or 3223 pulses of 2168 T each
    FirstSync, //667 T
    SecondSync, //735 T
    Data { pos: usize, bit: u8, last_half: bool }, //2 * 855 T or 1710 T
    End,
}

//The phase and remaining Tstates
struct TapePhaseT(u32, TapePhase);

impl TapePhaseT {
    fn pause() -> TapePhaseT {
        TapePhaseT(500000, TapePhase::Pause)
    }
    fn leader(header: bool) -> TapePhaseT {
        TapePhaseT(if header { 8063 } else { 2168 }, TapePhase::Leader{ pulse: 3223 })
    }
    fn first_sync() -> TapePhaseT {
        TapePhaseT(667, TapePhase::FirstSync)
    }
    fn second_sync() -> TapePhaseT {
        TapePhaseT(735, TapePhase::SecondSync)
    }
    fn data(tape: &Tape, block: usize, pos: usize, bit: u8) -> TapePhaseT {
        TapePhaseT(TapePhaseT::tape_bit_len(tape, block, pos, bit), TapePhase::Data{ pos, bit, last_half: false })
    }
    fn end() -> TapePhaseT {
        TapePhaseT(2500000, TapePhase::End)
    }
    fn tape_bit_len(tape: &Tape, block: usize, pos: usize, bit: u8) -> u32 {
        let byte = tape.blocks[block].data[pos];
        let v = byte & (0x80 >> bit) != 0;
        if v { 1710 } else { 855 }
    }
    fn mic(&self) -> bool {
        match self.1 {
            TapePhase::Pause => false,
            TapePhase::Leader { pulse } => pulse % 2 == 0,
            TapePhase::FirstSync => false,
            TapePhase::SecondSync => true,
            TapePhase::Data { last_half, .. } => last_half,
            TapePhase::End => false,
        }
    }
    fn next(self, d: u32, tape: &Tape, block: usize) -> Option<TapePhaseT> {
        let TapePhaseT(t, phase) = self;

        if t > d {
            return Some(TapePhaseT(t - d, phase));
        }

        let TapePhaseT(rt, rphase) = match phase {
            TapePhase::Pause => {
                let header = *tape.blocks[block].data.first().unwrap_or(&0) < 0x80;
                TapePhaseT::leader(header)
            }
            TapePhase::Leader { pulse } => {
                if pulse > 0 {
                    TapePhaseT(2168, TapePhase::Leader{ pulse: pulse - 1 })
                } else {
                    TapePhaseT::first_sync()
                }
            }
            TapePhase::FirstSync => {
                TapePhaseT::second_sync()
            }
            TapePhase::SecondSync => {
                TapePhaseT::data(tape, block, 0, 0)
            }
            TapePhase::Data { pos, bit, last_half } => {
                if !last_half {
                    TapePhaseT(TapePhaseT::tape_bit_len(tape, block, pos, bit), TapePhase::Data { pos, bit, last_half: true })
                } else {
                    if bit < 7 {
                        let bit = bit + 1;
                        TapePhaseT::data(tape, block, pos, bit)
                    } else if (pos as usize) < tape.blocks[block].data.len() - 1 {
                        let pos = pos + 1;
                        TapePhaseT::data(tape, block, pos, 0)
                    } else {
                        TapePhaseT::end()
                    }
                }
            }
            TapePhase::End => {
                return None;
            }
        };
        Some(TapePhaseT(rt.saturating_sub(d - t), rphase))
    }
}

pub struct TapePos {
    block: usize,
    phase: TapePhaseT,
}

impl TapePos {
    pub fn new_at_block(block: usize) -> TapePos {
        TapePos { block, phase: TapePhaseT::pause() }
    }
    pub fn mic(&self) -> bool {
        self.phase.mic()
    }
    pub fn block(&self, tape: &Tape) -> usize {
        let mut res = self.block;
        if res >= tape.blocks.len() {
            return 0xffffffff;
        }
        while !tape.blocks[res].selectable && res > 0 {
            res -= 1;
        }
        res
    }
}
