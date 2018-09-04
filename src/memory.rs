use std::io::{self, Read, Write};
use std::fs::File;
use std::path::Path;

pub struct Memory {
    data: Vec<u8>
}

impl Memory {
    pub fn new()-> Self {
        let data = vec![0; 0x10000];
        Memory{ data }
    }
    pub fn new_rom(rom: impl AsRef<Path>) -> io::Result<Self> {
        let mut data = vec![0; 0x10000];
        let mut f_rom = File::open(&rom)?;
        f_rom.read_exact(&mut data[0..0x4000])?;
        Ok(Memory { data })
    }
    pub fn new_from_bytes(rom: &[u8]) -> Self {
        let mut data = vec![0; 0x10000];
        data[0..rom.len()].copy_from_slice(rom);
        Memory { data }
    }
    pub fn peek(&self, addr: impl Into<u16>) -> u8 {
        self.data[addr.into() as usize]
    }
    pub fn poke(&mut self, addr: impl Into<u16>, data: u8) {
        let addr = addr.into();
        if addr < 0x4000 {
            //writing to rom
            //println!("writing to rom {:4x} <- {:2x}", addr, data);
            return;
        }
        self.data[addr as usize] = data;
    }
    pub fn peek_u16(&self, addr: impl Into<u16>) -> u16 {
        let addr = addr.into();
        let lo = self.peek(addr) as u16;
        let addr = addr.wrapping_add(1);
        let hi = self.peek(addr) as u16;
        (hi << 8) | lo
    }
    pub fn poke_u16(&mut self, addr: impl Into<u16>, data: u16) {
        let addr = addr.into();
        self.poke(addr, data as u8);
        let addr = addr.wrapping_add(1);
        self.poke(addr, (data >> 8) as u8);
    }
    pub fn slice(&self, addr: u16, end: u16) -> &[u8] {
        &self.data[addr as usize..end as usize]
    }
    pub fn save(&self, mut w: impl Write) -> io::Result<()> {
        w.write_all(&self.data)?;
        Ok(())
    }
    pub fn load(&mut self, mut r: impl Read) -> io::Result<()> {
        r.read_exact(&mut self.data)?;
        Ok(())
    }
}
