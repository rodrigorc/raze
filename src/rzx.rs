// Many struct fields are unused
#![allow(dead_code)]

use anyhow::anyhow;
use std::io::{self, prelude::*};

#[derive(Debug)]
pub struct Rzx {
    pub major: u8,
    pub minor: u8,
    pub flags: u32,
    pub blocks: Vec<Block>,
}

#[derive(Debug)]
pub enum Block {
    Creator(CreatorBlock),
    Snapshot(SnapshotBlock),
    Input(InputBlock),
    Unknown(UnknownBlock),
}

#[derive(Debug)]
pub struct CreatorBlock {
    pub creator: String,
    pub major: u16,
    pub minor: u16,
    pub custom: Vec<u8>,
}

#[derive(Debug)]
pub struct SnapshotBlock {
    pub format: String,
    pub data: Vec<u8>,
}

#[derive(Debug)]
pub struct InputBlock {
    pub frames: Vec<InputFrame>,
}

#[derive(Debug)]
pub struct UnknownBlock {
    pub id: u8,
    pub data: Vec<u8>,
}

#[derive(Debug)]
pub struct InputFrame {
    pub fetch_count: u16,
    pub in_values: InValues,
}

#[derive(Debug)]
pub enum InValues {
    RepeatLast,
    Data(Vec<u8>),
}

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
    fn drain(&mut self) -> io::Result<usize> {
        let mut bs = [0; 4096];
        let mut total = 0;
        loop {
            let len = self.read(&mut bs)?;
            total += len;
            if len < bs.len() {
                break;
            }
        }
        Ok(total)
    }
    fn read_string(&mut self, n: usize) -> io::Result<String> {
        let mut bs = self.read_vec(n)?;
        if let Some(eos) = bs.iter().position(|&c| c == 0) {
            bs.truncate(eos);
        }
        while bs.last() == Some(&b' ') {
            bs.pop();
        }
        let s = match String::from_utf8(bs) {
            Ok(s) => s,
            Err(e) => String::from_utf8_lossy(e.as_bytes()).into_owned(),
        };
        Ok(s)
    }
}

#[cfg(feature = "flate2")]
fn inflate(r: impl Read) -> anyhow::Result<impl Read> {
    let z = flate2::read::ZlibDecoder::new(r);
    Ok(z)
}

#[cfg(not(feature = "flate2"))]
fn inflate(_: impl Read) -> anyhow::Result<std::io::Empty> {
    Err(anyhow!("compressed RZX is not supported"))
}

impl Rzx {
    pub fn new<R: Read>(mut f: R) -> anyhow::Result<Rzx> {
        if f.read_u32()? != 0x21585a52 {
            return Err(anyhow!("invalid RZX signature"));
        }
        let major = f.read_u8()?;
        let minor = f.read_u8()?;
        let flags = f.read_u32()?;

        let mut blocks = Vec::new();

        loop {
            let mut id = 0;
            if f.read(std::slice::from_mut(&mut id))? == 0 {
                break;
            }
            let len = f.read_u32()?;
            let len = len
                .checked_sub(5)
                .ok_or_else(|| anyhow!("invalid RZX block length"))?;
            let mut f = f.by_ref().take(len as u64);
            let block = match id {
                //Creator
                0x10 => {
                    let creator = f.read_string(20)?;
                    let major = f.read_u16()?;
                    let minor = f.read_u16()?;
                    let mut custom = Vec::new();
                    f.read_to_end(&mut custom)?;
                    Block::Creator(CreatorBlock {
                        creator,
                        major,
                        minor,
                        custom,
                    })
                }
                /*
                //Security information
                0x20 => {
                }
                //Security signature
                0x21 => {
                }*/
                //Snapshot
                0x30 => {
                    let flags = f.read_u32()?;
                    let format = f.read_string(4)?;
                    let full_len = f.read_u32()?;
                    if (flags & 1) != 0 {
                        return Err(anyhow!("RZX external snapshot not supported"));
                    }
                    let mut data = Vec::new();
                    if (flags & 2) != 0 {
                        //compressed
                        let mut z = inflate(&mut f)?;
                        z.read_to_end(&mut data)?
                    } else {
                        //uncompressed
                        f.read_to_end(&mut data)?
                    };
                    if data.len() != full_len as usize {
                        log::warn!(
                            "RZX snapshot: compressed length does not match {} != {}",
                            data.len(),
                            full_len
                        );
                    }
                    Block::Snapshot(SnapshotBlock { format, data })
                }
                //Input recording
                0x80 => {
                    let num_frames = f.read_u32()?;
                    let _ = f.read_u8()?; //reserved
                    let _start_t = f.read_u32()?;
                    let flags = f.read_u32()?;
                    if (flags & 1) != 0 {
                        return Err(anyhow!("RZX encrypted file not supported"));
                    }
                    let mut z;
                    let r: &mut dyn Read = if (flags & 2) != 0 {
                        //compressed
                        z = inflate(&mut f)?;
                        &mut z
                    } else {
                        //uncompressed
                        &mut f
                    };
                    let mut frames = Vec::with_capacity(num_frames as usize);
                    for _ in 0..num_frames {
                        let fetch_count = r.read_u16()?;
                        let n = r.read_u16()?;
                        let in_values = if n == 0xffff {
                            InValues::RepeatLast
                        } else {
                            InValues::Data(r.read_vec(n as usize)?)
                        };
                        frames.push(InputFrame {
                            fetch_count,
                            in_values,
                        })
                    }
                    Block::Input(InputBlock { frames })
                }
                //Unknown
                id => {
                    log::warn!("rzx block id {:2x} is unknown", id);
                    let mut data = Vec::new();
                    f.read_to_end(&mut data)?;
                    Block::Unknown(UnknownBlock { id, data })
                }
            };
            blocks.push(block);
            //in case there are remaining data in this block
            let unk = f.drain()?;
            if unk != 0 {
                log::warn!("rzx block 0x{:2x} with spare bytes {}", id, unk);
            }
        }
        Ok(Rzx {
            major,
            minor,
            flags,
            blocks,
        })
    }
}
