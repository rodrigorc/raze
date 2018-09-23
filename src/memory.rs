use std::io::{self, Read, Write};

pub struct Memory {
    data: Vec<Vec<u8>>,
    banks: [usize; 4],
    vram: usize,
    locked: bool,
    delay: u32,
}

impl Memory {
    pub fn new_from_bytes(rom0: &[u8], rom1: Option<&[u8]>) -> Self {
        let brom0 = rom0.to_vec();
        match rom1 {
            None => {
                //48k
                let mut data = vec![brom0];
                for _ in 0..3 {
                    data.push(vec![0; 0x4000]);
                }
                Memory {
                    data,
                    banks: [0, 1, 2, 3],
                    vram: 1,
                    locked: true,
                    delay: 0
                }
            }
            Some(rom1) => {
                //128k
                let brom1 = rom1.to_vec();
                let mut data = vec![];
                for _ in 0..8 {
                    data.push(vec![0; 0x4000]);
                }
                data.push(brom0); //8: is the 128k rom
                data.push(brom1); //9: is the 128k-48k compatible rom
                Memory {
                    data,
                    banks: [8, 5, 2, 0],
                    vram: 5,
                    locked: false,
                    delay: 0
                }
            }
        }
    }
    //returns (readonly, bank, offset)
    #[inline]
    fn split_addr(&self, addr: impl Into<u16>) -> (bool, usize, usize) {
        let addr = addr.into();
        let ibank = (addr >> 14) as usize;
        let offs = (addr & 0x3fff) as usize;
        (ibank == 0, self.banks[ibank] as usize, offs)
    }
    pub fn peek(&mut self, addr: impl Into<u16>) -> u8 {
        let (_ro, bank, offs) = self.split_addr(addr);
        if bank == 1 {
            self.delay = self.delay.wrapping_add(1);
        }
        self.data[bank][offs]
    }
    pub fn peek_no_delay(&self, addr: u16) -> u8 {
        let (_ro, bank, offs) = self.split_addr(addr);
        self.data[bank][offs]
    }
    pub fn poke(&mut self, addr: impl Into<u16>, data: u8) {
        let (ro, bank, offs) = self.split_addr(addr);
        if ro {
            //log!("writing to rom {:4x} <- {:2x}", offs, data);
            return;
        }
        if bank == 1 {
            self.delay = self.delay.wrapping_add(1);
        }
        self.data[bank][offs] = data;
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
    pub fn video_memory(&self) -> &[u8] {
        &self.data[self.vram][0..32 * 192 + 32 * 24]
    }
    //TODO load/save banks
    pub fn save(&self, mut w: impl Write) -> io::Result<()> {
        for i in &self.banks {
            let ref bs = self.data[*i];
            w.write_all(bs)?;
        }
        Ok(())
    }
    pub fn load(mut r: impl Read) -> io::Result<Self> {
        let mut data = vec![];
        for _ in 0..4 {
            let mut bs = vec![0; 0x4000];
            r.read_exact(&mut bs)?;
            data.push(bs);
        }
        Ok(Memory {
            data,
            banks: [0, 1, 2, 3],
            vram: 1,
            locked: true,
            delay: 0
        })
    }
    pub fn switch_banks(&mut self, v: u8) {
        if self.locked {
            log!("mem locked");
            return;
        }
        self.banks[3] = (v & 0x07) as usize;
        self.vram = if v & 0x08 == 0 { 5 } else { 7 };
        self.banks[0] = if v & 0x10 == 0 { 8 } else { 9 };
        if v & 0x20 != 0 {
            self.locked = true;
        }
    }
}
