use anyhow::Result;
use std::io::BufRead;

#[derive(Debug, Default)]
pub struct PokFile {
    cheats: Vec<Cheat>,
}

#[derive(Debug)]
pub struct Cheat {
    pub name: String,
    pub entries: Vec<CheatEntry>,
}

#[derive(Debug)]
pub struct CheatEntry {
    pub bank: u8,
    pub addr: u16,
    pub value: u16,
    pub prev: u8,
}

impl PokFile {
    pub fn new() -> PokFile {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.cheats.is_empty()
    }

    pub fn cheats(&self) -> impl Iterator<Item = &Cheat> {
        self.cheats.iter()
    }

    pub fn parse<R: BufRead>(&mut self, mut rdr: R) -> Result<()> {
        let mut buf = Vec::new();
        loop {
            buf.clear();
            let len = rdr.read_until(b'\n', &mut buf)?;
            if len == 0 {
                break;
            }
            if buf.is_empty() {
                continue;
            }
            match buf[0] {
                b'N' => {
                    let name = String::from_utf8_lossy(&buf[1..]);
                    self.cheats.push(Cheat {
                        name: name.trim().to_owned(),
                        entries: Vec::new(),
                    });
                }
                b'M' | b'Z' => {
                    let Some(cheat) = self.cheats.last_mut() else {
                        continue;
                    };
                    let Ok(line) = str::from_utf8(&buf[1..]) else {
                        continue;
                    };
                    let mut words = line.split_ascii_whitespace();
                    let Some(bank) = words.next() else { continue };
                    let Some(addr) = words.next() else { continue };
                    let Some(value) = words.next() else { continue };
                    let Some(prev) = words.next() else { continue };

                    let Ok(bank) = bank.parse() else { continue };
                    let Ok(addr) = addr.parse() else { continue };
                    let Ok(value) = value.parse() else { continue };
                    let Ok(prev) = prev.parse() else { continue };

                    cheat.entries.push(CheatEntry {
                        bank,
                        addr,
                        value,
                        prev,
                    });
                }
                _ => {}
            }
        }

        Ok(())
    }
}
