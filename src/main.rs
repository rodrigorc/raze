extern crate png;
#[cfg(feature="zip")]
extern crate zip;

use std::env;
use std::io::{self, BufWriter, Write, BufReader, Read, ErrorKind};
use std::fs::File;
use std::path::Path;

use png::HasParameters;

#[macro_use]
mod logger {
    macro_rules! log {
        ( $($e:tt)* ) => {
            println!($($e)*)
        };
    }
}

#[macro_use]
mod js;
mod game;
mod psg;
mod memory;
mod z80;
mod tape;

#[no_mangle]
pub extern "C" fn alert(_ptr: *const u8, _len: usize) {
}
#[no_mangle]
pub unsafe extern "C" fn consolelog(ptr: *const u8, len: usize) {
    let data = std::slice::from_raw_parts(ptr, len);
    let text = String::from_utf8_lossy(data);
    println!("{}", text);
}
#[no_mangle]
pub extern "C" fn putImageData(_w: i32, h: i32, _data: *const u8, _len: usize) {
}
#[no_mangle]
pub extern "C" fn putSoundData(_data: *const u8, _len: usize) {
}
#[no_mangle]
pub extern "C" fn onTapeBlock(_index: usize) {
}

use memory::Memory;
use z80::{Z80, Bus};

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

struct Spectrum {
    x: i32,
    memory: Memory,
}

impl Bus for Spectrum {
    fn do_in(&mut self, port: impl Into<u16>) -> u8 {
        let port = port.into();
        let lo = port as u8;
        let hi = (port >> 8) as u8;
        let r = match lo {
            0xfe => {
                match hi {
                    0xdf => {
                        if 5 < self.x && self.x < 600 {
                            0xfe //P
                        } else {
                            0xff
                        }
                    }
                    0xf7 => {
                        if 700 < self.x && self.x < 750 {
                            0xfd //2
                        } else {
                            0xff
                        }
                    }
                    0xef => {
                        if 800 < self.x && self.x < 850 {
                            0xfe //0
                        } else {
                            0xff
                        }
                    }
                    0xbf => {
                        if 900 < self.x && self.x < 1000 {
                            0xfe //enter
                        } else {
                            0xff
                        }
                    }
                    _ => 0xff,
                }
            }
            _ => 0xff,
        };
        //println!("IN {:04x}, {:02x}", port, r);
        self.x += 1;
        r
    }
    fn do_out(&mut self, _port: impl Into<u16>, _value: u8) {
        //println!("OUT {:04x}, {:02x}", port, value);
    }
    fn peek(&mut self, addr: impl Into<u16>) -> u8 {
        self.memory.peek(addr)
    }
    fn poke(&mut self, addr: impl Into<u16>, value: u8) {
        self.memory.poke(addr, value);
    }
}

fn main() -> io::Result<()> {

    let mut args = env::args_os();
    let _program = args.next().ok_or(ErrorKind::InvalidData)?;


    let load = args.next().unwrap();

    let snap = std::fs::read(load)?;
    let mut game = game::Game::load_snapshot(&snap)?;

    game.key_down(0x60);

    for _ in 0..100 {
        game.draw_frame(false);
    }
    game.dump_memory("fairlight.bin");

    Ok(())
}
