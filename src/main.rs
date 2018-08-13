#![allow(unused)]
#![warn(unreachable_patterns)]

use std::env;
use std::io::{self, ErrorKind};

mod memory;
mod z80;

use memory::Memory;
use z80::Z80;

fn main() -> io::Result<()> {
    let mut args = env::args_os();
    let _program = args.next().ok_or(ErrorKind::InvalidData)?;
    let rom = args.next().unwrap();
    let mut memory = Memory::new(&rom)?;
    let mut z80 = Z80::new();

    loop {
        z80.exec(&mut memory);
    }
    //Ok(())
}
