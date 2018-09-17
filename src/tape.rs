use std::io::{self, Read, Cursor};

pub struct Tape {
    data: Vec<Vec<u8>>
}

impl Tape {
    pub fn new(tap: Vec<u8>) -> io::Result<Tape> {
        let mut data = Vec::new();
        let mut tap = Cursor::new(tap);

        loop {
            let mut len = [0; 2];
            match tap.read_exact(&mut len) {
                Ok(_) => (),
                Err(_) => break,
            }
            let len = len[0] as usize | ((len[1] as usize) << 8);
            let mut block = vec![0; len];
            tap.read_exact(&mut block)?;
            data.push(block);
        }

        Ok(Tape{ data } )
    }
    pub fn play(&self, d: u32, pos: TapePos) -> Option<TapePos> {
        let TapePos { mut block, phase } = pos;
        if (block as usize) >= self.data.len() {
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
}

pub enum TapePhase {
    Pause, //500000 T
    Leader { pulse: u32 }, //8063 or 3223 pulses of 2168 T each
    FirstSync, //667 T
    SecondSync, //735 T
    Data { pos: u32, bit: u8, last_half: bool }, //2 * 855 T or 1710 T
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
    fn data(tape: &Tape, block: u32, pos: u32, bit: u8) -> TapePhaseT {
        TapePhaseT(TapePhaseT::tape_bit_len(tape, block, pos, bit), TapePhase::Data{ pos, bit, last_half: false })
    }
    fn tape_bit_len(tape: &Tape, block: u32, pos: u32, bit: u8) -> u32 {
        let byte = tape.data[block as usize][pos as usize];
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
        }
    }
    fn next(self, d: u32, tape: &Tape, block: u32) -> Option<TapePhaseT> {
        let TapePhaseT(t, phase) = self;

        if t > d {
            return Some(TapePhaseT(t - d, phase));
        }

        let TapePhaseT(rt, rphase) = match phase {
            TapePhase::Pause => {
                log!("leader");
                let header = *tape.data[block as usize].first().unwrap_or(&0) < 0x80;
                TapePhaseT::leader(header)
            }
            TapePhase::Leader { pulse } => {
                if pulse > 0 {
                    TapePhaseT(2168, TapePhase::Leader{ pulse: pulse - 1 })
                } else {
                    log!("firstsync");
                    TapePhaseT::first_sync()
                }
            }
            TapePhase::FirstSync => {
                log!("secondsync");
                TapePhaseT::second_sync()
            }
            TapePhase::SecondSync => {
                log!("data");
                TapePhaseT::data(tape, block, 0, 0)
            }
            TapePhase::Data { pos, bit, last_half } => {
                if !last_half {
                    TapePhaseT(TapePhaseT::tape_bit_len(tape, block, pos, bit), TapePhase::Data { pos, bit, last_half: true })
                } else {
                    if bit < 7 {
                        let bit = bit + 1;
                        TapePhaseT::data(tape, block, pos, bit)
                    } else if (pos as usize) < tape.data[block as usize].len() - 1 {
                        let pos = pos + 1;
                        TapePhaseT::data(tape, block, pos, 0)
                    } else {
                        return None;
                    }
                }
            }
        };
        Some(TapePhaseT(rt.saturating_sub(d - t), rphase))
    }
}

pub struct TapePos {
    block: u32,
    phase: TapePhaseT,
}

impl TapePos {
    pub fn new() -> TapePos {
        TapePos { block: 0, phase: TapePhaseT::pause() }
    }
    pub fn mic(&self) -> bool {
        self.phase.mic()
    }
}
