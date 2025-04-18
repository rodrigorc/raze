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

#[macro_use]
mod js;
mod game;
mod memory;
mod psg;
mod rzx;
mod speaker;
mod tape;
mod z80;

struct ConsoleGui;

static PALETTE: [[u8; 8]; 2] = [[0, 1, 2, 3, 4, 5, 6, 7], [8, 9, 10, 11, 12, 13, 14, 15]];

impl game::Gui for ConsoleGui {
    type Pixel = u8;

    fn palette(&self) -> &[[Self::Pixel; 8]; 2] {
        &PALETTE
    }
    fn on_rzx_running(&mut self, _running: bool, _percent: u32) {}

    fn on_tape_block(&mut self, _index: usize) {}

    fn put_sound_data(&mut self, _data: &[f32]) {}

    fn put_image_data(&mut self, _w: usize, _h: usize, _data: &[Self::Pixel]) {}
}
fn main() -> anyhow::Result<()> {
    let mut args = env::args_os();
    let _program = args
        .next()
        .ok_or_else(|| anyhow!("Missing command line argument"))?;

    let load = args.next().unwrap();

    let snap = std::fs::read(load)?;
    //dbg!(rzx::Rzx::new(&mut &snap[..])?);

    let mut game = game::Game::load_snapshot(&snap, ConsoleGui)?;

    //game.key_down(0x60);

    for _ in 0..1000 {
        game.draw_frame(true);
    }

    Ok(())
}
