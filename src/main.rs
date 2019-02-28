#[macro_use]
extern crate cfg_if;

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
pub extern "C" fn putImageData(_w: i32, _h: i32, _data: *const u8, _len: usize) {
}
#[no_mangle]
pub extern "C" fn putSoundData(_data: *const u8, _len: usize) {
}
#[no_mangle]
pub extern "C" fn onTapeBlock(_index: usize) {
}

fn main() -> io::Result<()> {

    let mut args = env::args_os();
    let _program = args.next().ok_or(ErrorKind::InvalidData)?;


    let load = args.next().unwrap();

    let snap = std::fs::read(load)?;
    let mut game = game::Game::load_snapshot(&snap)?;

    //game.key_down(0x60);

    for _ in 0..100 {
        game.draw_frame(false);
    }

    Ok(())
}
