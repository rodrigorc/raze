use raze::Z80;
use zxspectrum_raze as raze;

use anyhow::anyhow;
use std::env;

#[allow(unused)]
#[macro_use]
mod logger {
    macro_rules! log {
        ( $($e:tt)* ) => {
            println!($($e)*)
        };
    }
}

struct ConsoleGui;

impl raze::Gui for ConsoleGui {
    type Pixel = u8;

    const PALETTE: [[u8; 8]; 2] = [[0, 1, 2, 3, 4, 5, 6, 7], [8, 9, 10, 11, 12, 13, 14, 15]];
}
fn main() -> anyhow::Result<()> {
    let mut args = env::args();
    let _program = args
        .next()
        .ok_or_else(|| anyhow!("Missing command line argument"))?;

    let load = args.next().unwrap();

    match load.as_str() {
        "add" => {
            let mut z80 = Z80::new();
            z80.dump_add();
        }
        "adc" => {
            let mut z80 = Z80::new();
            z80.dump_adc();
        }
        "sub" => {
            let mut z80 = Z80::new();
            z80.dump_sub();
        }
        "sbc" => {
            let mut z80 = Z80::new();
            z80.dump_sbc();
        }
        "daa" => {
            let mut z80 = Z80::new();
            z80.dump_daa();
        }
        file => {
            let snap = std::fs::read(file)?;
            //dbg!(rzx::Rzx::new(&mut &snap[..])?);

            let mut game = raze::Game::<ConsoleGui>::load_snapshot(&snap)?;

            //game.key_down(0x60);

            for _ in 0..1000 {
                game.do_frame(true);
            }
        }
    }

    Ok(())
}
