#![allow(non_snake_case)]

use game::Game;
use std::mem;

mod imports {
    extern "C" {
        pub fn alert(ptr: *const u8, len: usize);
        pub fn consolelog(ptr: *const u8, len: usize);
        pub fn putImageData(border: u8, w: i32, h: i32, data: *const u8, len: usize);
        pub fn putSoundData(data: *const u8, len: usize);
    }
}

pub fn alert(s: impl AsRef<str>) {
    let s = s.as_ref();
    unsafe { imports::alert(s.as_ptr(), s.len()) };
}
pub fn log(s: impl AsRef<str>)
{
    let s = s.as_ref();
    unsafe { imports::consolelog(s.as_ptr(), s.len()) };
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

pub fn putImageData<T>(border: u8, w: i32, h: i32, data: &[T]) {
    unsafe { imports::putImageData(border, w, h, data.as_ptr() as *const u8, data.len() * mem::size_of::<T>()) };
}

pub fn putSoundData(data: &[u8]) {
    unsafe { imports::putSoundData(data.as_ptr() as *const u8, data.len()) };
}

#[no_mangle]
pub extern "C" fn wasm_main() -> *mut Game {
    let game = Game::new();
    Box::into_raw(game)
}
#[no_mangle]
pub extern "C" fn wasm_alloc(size: usize) -> *mut u8 {
    let mut v = Vec::with_capacity(size);
    let ptr = v.as_mut_ptr();
    mem::forget(v);
    ptr
}
#[no_mangle]
pub extern "C" fn wasm_draw_frame(game: *mut Game) {
    let game = unsafe { &mut *game };
    game.draw_frame();
}
#[no_mangle]
pub extern "C" fn wasm_load_file(game: *mut Game, ptr: *mut u8, size: usize) {
    let (game, data) = unsafe {
        (&mut *game, Vec::from_raw_parts(ptr, size, size))
    };
    game.load_file(data);
}
#[no_mangle]
pub extern "C" fn wasm_snapshot(game: *mut Game) -> *const u8 {
    let game = unsafe { &mut *game };
    let data = game.snapshot();
    let ptr = data.as_ptr();
    mem::forget(data);
    ptr
}
#[no_mangle]
pub extern "C" fn wasm_reset_input(game: *mut Game) {
    let game = unsafe { &mut *game };
    game.reset_input();
}
#[no_mangle]
pub extern "C" fn wasm_key_up(game: *mut Game, key: i32) {
    let game = unsafe { &mut *game };
    game.key_up(key as usize);
}
#[no_mangle]
pub extern "C" fn wasm_key_down(game: *mut Game, key: i32) {
    let game = unsafe { &mut *game };
    game.key_down(key as usize);
}

