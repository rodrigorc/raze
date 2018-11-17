use std::io::{self, Read};

struct Bank {
    data: Vec<u8>,
    ro: bool,
    contended: bool,
}
impl Bank {
    fn rom(data: Vec<u8>) -> Bank {
        Bank { data, ro: true, contended: false }
    }
    fn ram(contended: bool) -> Bank {
        Bank { data: vec![0; 0x4000], ro: false, contended }
    }
}

pub struct Memory {
    data: Vec<Bank>,
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
                let mut data = Vec::new();
                data.push(Bank::rom(brom0));
                for i in 1..4 {
                    data.push(Bank::ram(i == 1));
                }
                Memory {
                    data,
                    banks: [0, 1, 2, 3],
                    vram: 1,
                    locked: true,
                    delay: 0,
                }
            }
            Some(rom1) => {
                //128k
                let brom1 = rom1.to_vec();
                let mut data = vec![];
                for i in 0..8 {
                    data.push(Bank::ram(i & 1 == 1));
                }
                data.push(Bank::rom(brom0)); //8: is the 128k rom
                data.push(Bank::rom(brom1)); //9: is the 128k-48k compatible rom
                Memory {
                    data,
                    banks: [8, 5, 2, 0],
                    vram: 5,
                    locked: false,
                    delay: 0,
                }
            }
        }
    }
    //All these functions use unchecked access to the arrays because the bit size of the arguments
    //make it impossible to overflow. Not checking bounds improves about 10% of CPU time.

    //returns (bankid, offset)
    #[inline]
    fn split_addr(&self, addr: impl Into<u16>) -> (usize, usize) {
        let addr = addr.into();
        let ibank = (addr >> 14) as usize;
        let offs = (addr & 0x3fff) as usize;
        (unsafe { *self.banks.get_unchecked(ibank) } as usize, offs)
    }
    #[inline]
    pub fn peek(&mut self, addr: impl Into<u16>) -> u8 {
        let (bank, offs) = self.split_addr(addr);
        let bank = unsafe { self.data.get_unchecked(bank) };
        if bank.contended {
            self.delay += 1;
        }
        unsafe { *bank.data.get_unchecked(offs) }
    }
    #[inline]
    pub fn peek_no_delay(&self, addr: u16) -> u8 {
        let (bank, offs) = self.split_addr(addr);
        let bank = unsafe { self.data.get_unchecked(bank) };
        unsafe { *bank.data.get_unchecked(offs) }
    }
    #[inline]
    pub fn poke(&mut self, addr: impl Into<u16>, data: u8) {
        let (bank, offs) = self.split_addr(addr);
        let bank = unsafe { self.data.get_unchecked_mut(bank) };
        if bank.ro {
            //log!("writing to rom {:4x} <- {:2x}", offs, data);
            return;
        }
        if bank.contended {
            self.delay += 1;
        }
        unsafe { *bank.data.get_unchecked_mut(offs) = data };
    }
    pub fn take_delay(&mut self) -> u32 {
        let r = self.delay;
        self.delay = 0;
        r
    }
    pub fn video_memory(&self) -> &[u8] {
        &self.data[self.vram].data[0..32 * 192 + 32 * 24]
    }
    pub fn load(mut r: impl Read) -> io::Result<Self> {
        let mut data = vec![];
        for i in 0..4 {
            let mut bank = Bank::ram(i == 1);
            if i == 0 {
                bank.ro = true;
            }
            r.read_exact(&mut bank.data)?;
            data.push(bank);
        }
        Ok(Memory {
            data,
            banks: [0, 1, 2, 3],
            vram: 1,
            locked: true,
            delay: 0,
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
    pub fn last_reg(&self) -> u8 {
        let mut r = self.banks[3] as u8;
        if self.vram == 7 { r |= 0x08; }
        if self.banks[0] == 9 { r |= 0x10; }
        if self.locked { r |= 0x20; }
        r
    }
    pub fn get_bank(&self, i: usize) -> &[u8] {
        &self.data[i].data
    }
    pub fn get_bank_mut(&mut self, i: usize) -> &mut [u8] {
        &mut self.data[i].data
    }
}
