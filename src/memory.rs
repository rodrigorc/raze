use std::io::{self, Read, Write};
use std::fs::File;
use std::path::Path;

pub struct Memory {
    data: Vec<u8>,
    delay: u32,
}

impl Memory {
    #[allow(unused)]
    pub fn new()-> Self {
        let data = vec![0; 0x10000];
        Memory { data, delay: 0 }
    }
    #[allow(unused)]
    pub fn new_rom(rom: impl AsRef<Path>) -> io::Result<Self> {
        let mut data = vec![0; 0x10000];
        let mut f_rom = File::open(&rom)?;
        f_rom.read_exact(&mut data[0..0x4000])?;
        Ok(Memory { data, delay: 0 })
    }
    pub fn new_from_bytes(rom: &[u8]) -> Self {
        let mut data = vec![0; 0x10000];
        data[0..rom.len()].copy_from_slice(rom);
        Memory { data, delay: 0 }
    }
    pub fn peek(&mut self, addr: impl Into<u16>) -> u8 {
        let addr = addr.into();
        if addr >= 0x4000 && addr  < 0x8000 {
            self.delay = self.delay.wrapping_add(1);
        }
        self.data[addr as usize]
    }
    pub fn poke(&mut self, addr: impl Into<u16>, data: u8) {
        let addr = addr.into();
        if addr < 0x4000 {
            //writing to rom
            //println!("writing to rom {:4x} <- {:2x}", addr, data);
            return;
        }
        if addr >= 0x4000 && addr  < 0x8000 {
            self.delay = self.delay.wrapping_add(1);
        }
        self.data[addr as usize] = data;
    }
    pub fn peek_u16(&mut self, addr: impl Into<u16>) -> u16 {
        let addr = addr.into();
        let lo = self.peek(addr) as u16;
        let addr = addr.wrapping_add(1);
        let hi = self.peek(addr) as u16;
        (hi << 8) | lo
    }
    pub fn take_delay(&mut self) -> u32 {
        let r = self.delay;
        self.delay = 0;
        r
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
    #[allow(unused)]
    pub fn load(&mut self, mut r: impl Read) -> io::Result<()> {
        r.read_exact(&mut self.data)?;
        Ok(())
    }
}
