//If not targetting wasm32 the wasm_bindgen are ignored and there are
//a lot of warnings about dead_code, for example with "cargo check".
//But for the real thing we want the warning there, so it is disabled conditionally.
#![cfg_attr(not(target_family="wasm"), allow(dead_code))]

use crate::game::{Game, Gui};
use std::mem;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = alert)]
    fn alert_slice(s: &str);
}


#[wasm_bindgen(raw_module = "../raze.js")]
extern "C" {
    pub fn putImageData(w: i32, h: i32, data: &[u8]);
    pub fn putSoundData(data: &[f32]);
    pub fn onTapeBlock(index: usize);
    pub fn onRZXRunning(is_running: bool);
}

pub fn alert(s: impl AsRef<str>) {
    let s = s.as_ref();
    log::error!("{}", s);
    alert_slice(s);
}

macro_rules! alert {
    ( $($e:tt)* ) => {
        $crate::js::alert(format!($($e)*))
    };
}

mod color {
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct Pixel(pub u8, pub u8, pub u8, pub u8);

    const fn pixel(v: bool, c: u8) -> Pixel {
        let b = (c & 1) != 0;
        let r = (c & 2) != 0;
        let g = (c & 4) != 0;
        let x = if v { 0xff } else { 0xd7 };
        Pixel(
            if r { x } else { 0 },
            if g { x } else { 0 },
            if b { x } else { 0 },
            0xff
        )
    }
    const fn lo(c: u8) -> Pixel { pixel(false, c) }
    const fn hi(c: u8) -> Pixel { pixel(true, c) }

    pub static PALETTE : [[Pixel; 8]; 2] = [
        [lo(0), lo(1), lo(2), lo(3), lo(4), lo(5), lo(6), lo(7)],
        [hi(0), hi(1), hi(2), hi(3), hi(4), hi(5), hi(6), hi(7)],
    ];
}

pub struct JSGui;

impl Gui for JSGui {
    type Pixel = color::Pixel;

    fn palette(&self) -> &[[Self::Pixel; 8]; 2] {
        &color::PALETTE
    }
    fn put_image_data(&mut self, w: usize, h: usize, data: &[Self::Pixel]) {
        //Pixel is repr(C) just like [u8;4]
        let ptr = data.as_ptr() as *const u8;
        let len = data.len() * mem::size_of::<Self::Pixel>();
        let bytes = unsafe {
            std::slice::from_raw_parts(ptr, len)
        };
        putImageData(w as i32, h as i32, bytes);
    }
    fn put_sound_data(&mut self, data: &[f32]) {
        putSoundData(data);
    }
    fn on_tape_block(&mut self, index: usize) {
        onTapeBlock(index);
    }
    fn on_rzx_running(&mut self, running: bool) {
        onRZXRunning(running);
    }
}

mod exports {
    use super::*;

    #[wasm_bindgen]
    pub fn wasm_main(is128k: bool) -> *mut Game<JSGui> {
        let _ = console_log::init_with_level(log::Level::Debug);
        let game = Box::new(Game::new(is128k, JSGui));
        Box::into_raw(game)
    }
    #[wasm_bindgen]
    pub fn wasm_drop(game: *mut Game<JSGui>) {
        let _game = unsafe { Box::from_raw(game) };
    }
    #[wasm_bindgen]
    pub fn wasm_alloc(size: usize) -> *mut u8 {
        let mut v = Vec::with_capacity(size);
        let ptr = v.as_mut_ptr();
        mem::forget(v);
        ptr
    }
    #[wasm_bindgen]
    pub fn wasm_draw_frame(game: *mut Game<JSGui>, turbo: bool) {
        let game = unsafe { &mut *game };
        game.draw_frame(turbo);
    }
    #[wasm_bindgen]
    pub fn wasm_load_tape(game: *mut Game<JSGui>, data: Vec<u8>) -> usize {
        let game = unsafe { &mut *game };
        game.tape_load(data)
    }
    #[wasm_bindgen]
    pub fn wasm_tape_name(game: *mut Game<JSGui>, index: usize) -> String {
        let game = unsafe { &mut *game };
        game.tape_name(index).to_owned()
    }
    #[wasm_bindgen]
    pub fn wasm_tape_selectable(game: *mut Game<JSGui>, index: usize) -> bool {
        let game = unsafe { &mut *game };
        game.tape_selectable(index)
    }
    #[wasm_bindgen]
    pub fn wasm_tape_seek(game: *mut Game<JSGui>, index: usize) {
        let game = unsafe { &mut *game };
        game.tape_seek(index);
    }
    #[wasm_bindgen]
    pub fn wasm_tape_stop(game: *mut Game<JSGui>) {
        let game = unsafe { &mut *game };
        game.tape_stop();
    }
    #[wasm_bindgen]
    pub fn wasm_load_snapshot(game: *mut Game<JSGui>, data: &[u8]) -> bool{
        let old_game = unsafe { &mut *game };
        log::debug!("snap len {}", data.len());
        match Game::load_snapshot(data, JSGui) {
            Ok(new_game) => {
                *old_game = new_game;
            }
            Err(e) => {
                alert!("Snapshot error: {}", e);
            }
        }
        old_game.is_128k()
    }
    #[wasm_bindgen]
    pub fn wasm_snapshot(game: *mut Game<JSGui>) -> Vec<u8> {
        let game = unsafe { &mut *game };
        game.snapshot()
    }
    #[wasm_bindgen]
    pub fn wasm_reset_input(game: *mut Game<JSGui>) {
        let game = unsafe { &mut *game };
        game.reset_input();
    }
    #[wasm_bindgen]
    pub fn wasm_key_up(game: *mut Game<JSGui>, key: i32) {
        let game = unsafe { &mut *game };
        game.key_up(key as usize);
    }
    #[wasm_bindgen]
    pub fn wasm_key_down(game: *mut Game<JSGui>, key: i32) {
        let game = unsafe { &mut *game };
        game.key_down(key as usize);
    }
    #[wasm_bindgen]
    pub fn wasm_peek(game: *mut Game<JSGui>, addr: u16) -> u8 {
        let game = unsafe { &mut *game };
        game.peek(addr)
    }
    #[wasm_bindgen]
    pub fn wasm_poke(game: *mut Game<JSGui>, addr: u16, value: u8) {
        let game = unsafe { &mut *game };
        game.poke(addr, value);
    }
    #[wasm_bindgen]
    pub fn wasm_stop_rzx_replay(game: *mut Game<JSGui>) {
        let game = unsafe { &mut *game };
        game.stop_rzx_replay();
    }
}

