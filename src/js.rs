#![allow(non_snake_case)]

use crate::game::Game;
use std::mem;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn log_slice(s: &str);
    #[wasm_bindgen(js_name = alert)]
    fn alert_slice(s: &str);
}

#[wasm_bindgen(raw_module = "../raze.js")]
extern "C" {
    #[wasm_bindgen(js_name = putImageData)]
    fn putImageDataU8(w: i32, h: i32, data: &[u8]);
    pub fn putSoundData(data: &[f32]);
    pub fn onTapeBlock(index: usize);
}

//Wrapper for console.log that takes a `impl AsRef<str>`
pub fn log(s: impl AsRef<str>) {
    log_slice(s.as_ref());
}

pub fn alert(s: impl AsRef<str>) {
    alert_slice(s.as_ref());
}

macro_rules! log {
    ( $($e:tt)* ) => {
        $crate::js::log(format!($($e)*))
    };
}
macro_rules! alert {
    ( $($e:tt)* ) => {
        $crate::js::alert(format!($($e)*))
    };
}

//Wrapper for extern.putImageData, because in Rust we will use &[Pixel], but in JS we prefer &[u8].
pub fn putImageData<T>(w: i32, h: i32, data: &[T]) {
    let ptr = data.as_ptr() as *const u8;
    let len = data.len() * mem::size_of::<T>();
    unsafe {
        let bytes = std::slice::from_raw_parts(ptr, len);
        putImageDataU8(w, h, bytes);
    }
}

mod exports {
    use super::*;

    #[wasm_bindgen]
    pub fn wasm_main(is128k: bool) -> *mut Game {
        let game = Box::new(Game::new(is128k));
        Box::into_raw(game)
    }
    #[wasm_bindgen]
    pub fn wasm_drop(game: *mut Game) {
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
    pub fn wasm_draw_frame(game: *mut Game, turbo: bool) {
        let game = unsafe { &mut *game };
        game.draw_frame(turbo);
    }
    #[wasm_bindgen]
    pub fn wasm_load_tape(game: *mut Game, data: Vec<u8>) -> usize {
        let game = unsafe { &mut *game };
        game.tape_load(data)
    }
    #[wasm_bindgen]
    pub fn wasm_tape_name(game: *mut Game, index: usize) -> String {
        let game = unsafe { &mut *game };
        game.tape_name(index).to_owned()
    }
    #[wasm_bindgen]
    pub fn wasm_tape_selectable(game: *mut Game, index: usize) -> bool {
        let game = unsafe { &mut *game };
        game.tape_selectable(index)
    }
    #[wasm_bindgen]
    pub fn wasm_tape_seek(game: *mut Game, index: usize) {
        let game = unsafe { &mut *game };
        game.tape_seek(index);
    }
    #[wasm_bindgen]
    pub fn wasm_tape_stop(game: *mut Game) {
        let game = unsafe { &mut *game };
        game.tape_stop();
    }
    #[wasm_bindgen]
    pub fn wasm_load_snapshot(game: *mut Game, data: &[u8]) -> bool{
        let old_game = unsafe { &mut *game };
        log!("snap len {}", data.len());
        match Game::load_snapshot(data) {
            Ok(new_game) => {
                *old_game = new_game;
            }
            Err(e) => {
                alert!("{}", e);
            }
        }
        old_game.is_128k()
    }
    #[wasm_bindgen]
    pub fn wasm_snapshot(game: *mut Game) -> Vec<u8> {
        let game = unsafe { &mut *game };
        game.snapshot()
    }
    #[wasm_bindgen]
    pub fn wasm_reset_input(game: *mut Game) {
        let game = unsafe { &mut *game };
        game.reset_input();
    }
    #[wasm_bindgen]
    pub fn wasm_key_up(game: *mut Game, key: i32) {
        let game = unsafe { &mut *game };
        game.key_up(key as usize);
    }
    #[wasm_bindgen]
    pub fn wasm_key_down(game: *mut Game, key: i32) {
        let game = unsafe { &mut *game };
        game.key_down(key as usize);
    }
    #[wasm_bindgen]
    pub fn wasm_peek(game: *mut Game, addr: u16) -> u8 {
        let game = unsafe { &mut *game };
        game.peek(addr)
    }
    #[wasm_bindgen]
    pub fn wasm_poke(game: *mut Game, addr: u16, value: u8) {
        let game = unsafe { &mut *game };
        game.poke(addr, value);
    }
}

