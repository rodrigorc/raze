use std::env;
use std::io::{self, ErrorKind};

#[allow(unused)]
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
mod speaker;
mod memory;
mod z80;
mod tape;

fn main() -> io::Result<()> {

    let mut args = env::args_os();
    let _program = args.next().ok_or(ErrorKind::InvalidData)?;


    let load = args.next().unwrap();

    let snap = std::fs::read(load)?;
    let mut game = game::Game::load_snapshot(&snap)?;

    //game.key_down(0x60);

    for _ in 0..1000 {
        game.draw_frame(true);
    }

    Ok(())
}
