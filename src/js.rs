use game::Game;
use std::mem;

#[allow(non_snake_case)]
mod imports {
    extern "C" {
        pub fn alert(ptr: *const u8, len: usize);
        pub fn log(ptr: *const u8, len: usize);
        pub fn clearRect(ctx: i32, a: f32, b: f32, c: f32, d: f32);
        pub fn fillStyle(ctx: i32, ptr: *const u8, len: usize);
        pub fn fillRect(ctx: i32, a: f32, b: f32, c: f32, d: f32);
        pub fn strokeStyle(ctx: i32, ptr: *const u8, len: usize);
        pub fn strokeRect(ctx: i32, x: f32, y: f32, w: f32, h: f32);
        pub fn beginPath(ctx: i32);
        pub fn closePath(ctx: i32);
        pub fn stroke(ctx: i32);
        pub fn fill(ctx: i32);
        pub fn moveTo(ctx: i32, x: f32 , y: f32);
        pub fn lineTo(ctx: i32, x: f32, y: f32);
        pub fn arc(ctx: i32, x: f32, y: f32, r: f32, a1: f32, a2: f32, o: f32);
        pub fn arcTo(ctx: i32, x1: f32, y1: f32, x2: f32, y2: f32, r: f32);
        pub fn rect(ctx: i32, x: f32, y: f32, w: f32, h: f32);
        pub fn lineWidth(ctx: i32, w: f32);
        pub fn putImageData(ctx: i32, w: i32, h: i32, data: *const u8, len: usize);
    }
}

pub fn alert(s: impl AsRef<str>) {
    let s = s.as_ref();
    unsafe { imports::alert(s.as_ptr(), s.len()) };
}
pub fn log(s: impl AsRef<str>)
{
    let s = s.as_ref();
    unsafe { imports::log(s.as_ptr(), s.len()) };
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

pub enum Canvas {
    Bg,
    Fg,
}

#[allow(non_snake_case)]
impl Canvas {
    pub fn clearRect(self, a: f32, b: f32, c: f32, d: f32) {
        unsafe { imports::clearRect(self as i32, a, b, c, d) };
    }
    pub fn fillStyle(self, s: impl AsRef<str>) {
        let s = s.as_ref();
        unsafe { imports::fillStyle(self as i32, s.as_ptr(), s.len()) };
    }
    pub fn fillRect(self, a: f32, b: f32, c: f32, d: f32) {
        unsafe { imports::fillRect(self as i32, a, b, c, d) };
    }
    pub fn strokeStyle(self, s: impl AsRef<str>) {
        let s = s.as_ref();
        unsafe { imports::strokeStyle(self as i32, s.as_ptr(), s.len()) };
    }
    pub fn strokeRect(self, x: f32, y: f32, w: f32, h: f32) {
        unsafe { imports::strokeRect(self as i32, x, y, w, h) };
    }
    pub fn beginPath(self) {
        unsafe { imports::beginPath(self as i32) };
    }
    pub fn closePath(self) {
        unsafe { imports::closePath(self as i32) };
    }
    pub fn stroke(self) {
        unsafe { imports::stroke(self as i32) };
    }
    pub fn fill(self) {
        unsafe { imports::fill(self as i32) };
    }
    pub fn moveTo(self, x: f32 , y: f32) {
        unsafe { imports::moveTo(self as i32, x, y) };
    }
    pub fn lineTo(self, x: f32, y: f32) {
        unsafe { imports::lineTo(self as i32, x, y) };
    }
    pub fn arc(self, x: f32, y: f32, r: f32, a1: f32, a2: f32, o: f32) {
        unsafe { imports::arc(self as i32, x, y, r, a1, a2, o) };
    }
    pub fn arcTo(self, x1: f32, y1: f32, x2: f32, y2: f32, r: f32) {
        unsafe { imports::arcTo(self as i32, x1, y1, x2, y2, r) };
    }
    pub fn rect(self, x: f32, y: f32, w: f32, h: f32) {
        unsafe { imports::rect(self as i32, x, y, w, h) };
    }
    pub fn lineWidth(self, w: f32) {
        unsafe { imports::lineWidth(self as i32, w) };
    }
    pub fn putImageData<T>(self, w: i32, h: i32, data: &[T]) {
        unsafe { imports::putImageData(self as i32, w, h, data.as_ptr() as *const u8, data.len() * mem::size_of::<T>()) };
    }
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
pub extern "C" fn wasm_mouse_move(game: *mut Game, x: f32, y: f32) {
    let game = unsafe { &mut *game };
    game.mouse_move(x, y);
}
#[no_mangle]
pub extern "C" fn wasm_mouse_up(game: *mut Game, x: f32, y: f32) {
    let game = unsafe { &mut *game };
    game.mouse_up(x, y);
}
#[no_mangle]
pub extern "C" fn wasm_mouse_down(game: *mut Game, x: f32, y: f32) {
    let game = unsafe { &mut *game };
    game.mouse_down(x, y);
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

