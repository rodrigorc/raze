#![allow(unused)]
#![warn(unreachable_patterns)]

extern crate png;

use std::env;
use std::io::{self, BufWriter, Write, ErrorKind};
use std::fs::File;
use std::path::Path;

use png::HasParameters;

mod memory;
mod z80;

use memory::Memory;
use z80::Z80;

fn write_screen(path: impl AsRef<Path>, data: &[u8]) -> io::Result<()> {
    let file = File::create(path)?;
    let mut w = BufWriter::new(file);
    //w.write_all(data)?;

    let mut encoder = png::Encoder::new(w, 256, 192);
    encoder.set(png::ColorType::Grayscale).set(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;

    let mut ps = vec![0; 256 * 192 * 1];
    for y in 0..192 {
        let orow = match y {
            0..=63 => {
                let y = (y % 8) * 256 + (y / 8) * 32;
                y
            }
            64..=127 => {
                let y = y - 64;
                let y = (y % 8) * 256 + (y / 8) * 32;
                y + 64 * 32
            }
            128..=191 => {
                let y = y - 128;
                let y = (y % 8) * 256 + (y / 8) * 32;
                y + 128 * 32
            }
            _ => unreachable!()
        };
        for x in 0..32 {
            let attr = data[192 * 32 + (y / 8) * 32 + x];
            let d = data[orow + x];
            for b in 0..8 {
                let pix = ((d >> (7-b)) & 1) != 0;
                let pixo = (256 * y + 8*x + b) * 1;
                if pix {
                    ps[pixo + 0] = (attr & 0b0000_0111) << 5;
                    //ps[pixo + 1] = 0x00;
                    //ps[pixo + 2] = 0x00;
                } else {
                    ps[pixo + 0] = (attr & 0b0011_1000) << 2;
                    //ps[pixo + 1] = 0xff;
                    //ps[pixo + 2] = 0xff;
                }
            }
        }
    }
    writer.write_image_data(&ps)?;
    Ok(())
}

fn main() -> io::Result<()> {
    let mut args = env::args_os();
    let _program = args.next().ok_or(ErrorKind::InvalidData)?;
    let rom = args.next().unwrap();
    let mut memory = Memory::new(&rom)?;
    let mut z80 = Z80::new();

    let mut count = 0;
    const SCROPS : i32 = 10_000;
    loop {
        z80.dump_regs();
        z80.exec(&mut memory);
        count += 1;
        if count % SCROPS == 0 {
            {
                let screen = memory.slice(0x4000, 0x4000 + 32 * 192 + 32 * 24);
                write_screen(format!("scr{:06}.png", count / SCROPS), screen)?;
            }
            z80.interrupt(&mut memory);
        }
        if count == 1_000_000 { break }
    }
    Ok(())
}
