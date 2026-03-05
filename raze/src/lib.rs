mod disk;
mod floppy;
mod game;
mod memory;
mod psg;
mod rzx;
mod speaker;
mod tape;
mod z80;

pub use game::{Game, Gui, Model};
pub use z80::Z80;

use std::io::{self, Read};

impl<R: Read + ?Sized> ReadExt for R {}

trait ReadExt: Read {
    fn read_u8(&mut self) -> io::Result<u8> {
        let mut b = 0;
        self.read_exact(std::slice::from_mut(&mut b))?;
        Ok(b)
    }
    fn read_u16(&mut self) -> io::Result<u16> {
        let mut bs = [0; 2];
        self.read_exact(&mut bs)?;
        Ok(u16::from_le_bytes(bs))
    }
    fn read_u32(&mut self) -> io::Result<u32> {
        let mut bs = [0; 4];
        self.read_exact(&mut bs)?;
        Ok(u32::from_le_bytes(bs))
    }
    fn read_vec(&mut self, n: usize) -> io::Result<Vec<u8>> {
        let mut data = vec![0; n];
        self.read_exact(&mut data)?;
        Ok(data)
    }
    fn read_string(&mut self, n: usize) -> io::Result<String> {
        let bs = self.read_vec(n)?;
        Ok(latin1_to_string(&bs))
    }
}

fn latin1_to_string(s: &[u8]) -> String {
    s.iter().map(|&c| c as char).collect()
}
