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
    last_banks: u8,
    last_banks_plus2: u8,
}

impl Memory {
    pub fn new_from_bytes(rom0: &[u8], rom1: Option<&[u8]>) -> Self {
        assert_eq!(rom0.len(), 0x4000);
        let rom0 = rom0.to_vec();
        match rom1 {
            None => {
                //48k
                let data = vec![
                    Bank::rom(rom0),
                    Bank::ram(true),
                    Bank::ram(false),
                    Bank::ram(false),
                ];
                Memory {
                    data,
                    banks: [0, 1, 2, 3],
                    vram: 1,
                    locked: true,
                    delay: 0,
                    last_banks: 0,
                    last_banks_plus2: 0,
                }
            }
            Some(rom1) => {
                //128k
                assert_eq!(rom1.len(), 0x4000);
                let rom1 = rom1.to_vec();
                let data = vec![
                    Bank::ram(false),
                    Bank::ram(true),
                    Bank::ram(false),
                    Bank::ram(true),
                    Bank::ram(false),
                    Bank::ram(true),
                    Bank::ram(false),
                    Bank::ram(true),
                    Bank::rom(rom0), //8: is the 128k rom
                    Bank::rom(rom1), //9: is the 128k-48k compatible rom
                ];
                Memory {
                    data,
                    banks: [8, 5, 2, 0],
                    vram: 5,
                    locked: false,
                    delay: 0,
                    last_banks: 0,
                    last_banks_plus2: 0,
                }
            }
        }
    }
    //All these functions use unchecked access to the arrays because the bit size of the arguments
    //make it impossible to overflow. Not checking bounds improves about 10% of CPU time.

    //returns (bankid, offset)
    #[inline]
    fn split_addr(&self, addr: impl Into<u16>) -> (usize, usize) {
        let addr: u16 = addr.into();
        let ibank = (addr >> 14) as usize;
        let offs = (addr & 0x3fff) as usize;
        // SAFETY: addr is 16 bits, split in 2 and 14 bits, ibank is between
        // 0x00 to 0x03, and self.banks is of length 4.
        (unsafe { *self.banks.get_unchecked(ibank) }, offs)
    }
    #[inline]
    pub fn peek(&mut self, addr: impl Into<u16>) -> u8 {
        let (bank, offs) = self.split_addr(addr);
        // SAFETY: bank is between 0..3 in 48k mode and between 0..9 in 128k mode.
        // The first is ensured by self.locked=true in 48k mode that prevents any change.
        // The second is so because Self::switch_banks{,2}() never assigns a value >9.
        let bank = unsafe { self.data.get_unchecked(bank) };
        if bank.contended {
            self.delay += 1;
        }
        // SAFETY: offs is 14 bits, and every bank is 0x4000 bytes long.
        // RAM banks are just created that way (see Bank::ram), while ROM
        // bank length is asserted in Memory::new_from_bytes()
        unsafe { *bank.data.get_unchecked(offs) }
    }
    #[inline]
    pub fn peek_no_delay(&self, addr: u16) -> u8 {
        let (bank, offs) = self.split_addr(addr);
        // SAFETY: same rules as Self::peek().
        let bank = unsafe { self.data.get_unchecked(bank) };
        unsafe { *bank.data.get_unchecked(offs) }
    }
    #[inline]
    pub fn poke(&mut self, addr: impl Into<u16>, data: u8) {
        let (bank, offs) = self.split_addr(addr);
        // SAFETY: same rules as Self::peek().
        let bank = unsafe { self.data.get_unchecked_mut(bank) };
        if bank.ro {
            //log!("writing to rom {:4x} <- {:2x}", offs, data);
            return;
        }
        if bank.contended {
            self.delay += 1;
        }
        // SAFETY: same rules as Self::peek().
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
    pub fn switch_banks(&mut self, v: u8) {
        if self.locked {
            log::warn!("mem locked");
            return;
        }
        self.banks[3] = (v & 0x07) as usize;
        self.vram = if v & 0x08 == 0 { 5 } else { 7 };
        self.banks[0] = if v & 0x10 == 0 { 8 } else { 9 };
        if v & 0x20 != 0 {
            self.locked = true;
        }
        self.last_banks = v;
    }
    pub fn switch_banks_plus2(&mut self, v: u8) {
        if self.locked {
            log::warn!("mem locked");
            return;
        }
        //special mode
        if v & 1 != 0 {
            let mode = (v >> 1) & 0x03;
            match mode {
                0 => {
                    self.banks = [0, 1, 2, 3];
                }
                1 => {
                    self.banks = [4, 5, 6, 7];
                }
                2 => {
                    self.banks = [4, 5, 6, 3];
                }
                3 => {
                    self.banks = [4, 7, 6, 3];
                }
                _ => unreachable!()
            }
        } else {
            self.banks = [8, 5, 2, 0];
            let v0 = self.last_banks;
            self.switch_banks(v0);
        }
        self.last_banks_plus2 = v;
    }
    pub fn last_banks(&self) -> u8 {
        self.last_banks
    }
    pub fn last_banks_plus2(&self) -> u8 {
        self.last_banks_plus2
    }
    pub fn get_bank(&self, i: usize) -> &[u8] {
        &self.data[i].data
    }
    pub fn get_bank_mut(&mut self, i: usize) -> &mut [u8] {
        &mut self.data[i].data
    }
}
