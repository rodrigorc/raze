use std::io;
use std::env;

#[macro_use]
mod logger {
    macro_rules! log {
        ( $($e:tt)* ) => {
            println!($($e)*)
        };
    }
}

mod z80;
mod memory;

use z80::Z80;

fn main() {
    let mut args = env::args();
    let program = args.next().unwrap();
    let mut z80 = Z80::new();

    let load = args.next();
    match load {
        Some(load) => {
            if load == "add" {
                z80.dump_add();
            } else if load == "adc" {
                z80.dump_adc();
            } else if load == "sub" {
                z80.dump_sub();
            } else if load == "sbc" {
                z80.dump_sbc();
            } else if load == "daa" {
                z80.dump_daa();
            } else {
                println!("unknown dump_op '{0}'", load);
            }
        }
        None => {
            println!("Usage: {0} <opcode>", program);
        }
    }
}
