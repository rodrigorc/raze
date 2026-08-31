//If not targetting wasm32 the wasm_bindgen are ignored and there are
//a lot of warnings about dead_code, for example with "cargo check".
//But for the real thing we want the warning there, so it is disabled conditionally.
#![cfg_attr(not(target_family = "wasm"), allow(dead_code))]

use color::Pixel;
use zxspectrum_raze as raze;

use raze::{Game, Gui, Model};
use std::mem;
use wasm_bindgen::prelude::*;

mod color {
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct Pixel {
        r: u8,
        g: u8,
        b: u8,
        a: u8,
    }

    const fn pixel(v: bool, c: u8) -> Pixel {
        let b = (c & 1) != 0;
        let r = (c & 2) != 0;
        let g = (c & 4) != 0;
        let x = if v { 0xff } else { 0xd7 };
        Pixel {
            r: if r { x } else { 0 },
            g: if g { x } else { 0 },
            b: if b { x } else { 0 },
            a: 0xff,
        }
    }
    const fn lo(c: u8) -> Pixel {
        pixel(false, c)
    }
    const fn hi(c: u8) -> Pixel {
        pixel(true, c)
    }

    pub const PALETTE: [[Pixel; 8]; 2] = [
        [lo(0), lo(1), lo(2), lo(3), lo(4), lo(5), lo(6), lo(7)],
        [hi(0), hi(1), hi(2), hi(3), hi(4), hi(5), hi(6), hi(7)],
    ];
}

pub struct JSGui;

impl Gui for JSGui {
    type Pixel = color::Pixel;
    const PALETTE: [[Pixel; 8]; 2] = color::PALETTE;
}

#[derive(Default)]
struct GameBuilder {
    model: ModelBuilder,
    border: Option<(i32, i32)>,
}

enum ModelBuilder {
    Model(Model),
    Snapshot(Vec<u8>),
}

impl Default for ModelBuilder {
    fn default() -> Self {
        ModelBuilder::Model(Model::Spec128k)
    }
}

mod exports {

    use super::*;

    #[wasm_bindgen]
    pub fn wasm_main() {
        let _ = console_log::init_with_level(log::Level::Debug);
    }

    #[wasm_bindgen]
    pub fn wasm_builder_new() -> *mut GameBuilder {
        let res = Box::new(GameBuilder::default());
        Box::into_raw(res)
    }

    #[wasm_bindgen]
    pub fn wasm_builder_set_model(bld: *mut GameBuilder, model: i32) {
        let bld = unsafe { &mut *bld };
        let model = match model {
            0 => Model::Spec48k,
            1 => Model::Spec128k,
            2 => Model::Plus3,
            _ => Model::Spec128k,
        };
        bld.model = ModelBuilder::Model(model);
    }

    #[wasm_bindgen]
    pub fn wasm_builder_set_snapshot(bld: *mut GameBuilder, data: &[u8]) {
        let bld = unsafe { &mut *bld };
        bld.model = ModelBuilder::Snapshot(data.to_vec());
    }

    #[wasm_bindgen]
    pub fn wasm_builder_set_border(bld: *mut GameBuilder, border_x: i32, border_y: i32) {
        let bld = unsafe { &mut *bld };
        bld.border = Some((border_x, border_y));
    }

    #[wasm_bindgen]
    pub fn wasm_builder_build(bld: *mut GameBuilder) -> Result<*mut Game<JSGui>, JsError> {
        let bld = unsafe { Box::from_raw(bld) };
        let bld = *bld;
        let mut game = match bld.model {
            ModelBuilder::Model(model) => Game::new(model),
            ModelBuilder::Snapshot(data) => {
                match Game::load_rom(&data).or_else(|_| Game::load_snapshot(&data)) {
                    Ok(g) => g,
                    Err(e) => {
                        return Err(JsError::new(&format!("Snapshot error: {e}")));
                    }
                }
            }
        };
        if let Some((bx, by)) = bld.border {
            game.set_border_size(bx as usize, by as usize);
        }
        let game = Box::new(game);
        Ok(Box::into_raw(game))
    }
    #[wasm_bindgen]
    pub fn wasm_game_model(game: *mut Game<JSGui>) -> i32 {
        let game = unsafe { &*game };
        match game.model() {
            Model::Spec48k => 0,
            Model::Spec128k => 1,
            Model::Plus3 => 2,
        }
    }
    #[wasm_bindgen]
    pub fn wasm_game_drop(game: *mut Game<JSGui>) {
        let _game = unsafe { Box::from_raw(game) };
    }

    #[wasm_bindgen]
    pub fn wasm_do_frame(
        game: *mut Game<JSGui>,
        turbo: bool,
        callback: js_sys::Function,
    ) -> Result<(), JsValue> {
        let game = unsafe { &mut *game };
        game.do_frame(turbo);

        let rzx = game.rzx_status();
        let tape_block = game.tape_block();

        callback.call2(&JsValue::NULL, &rzx.into(), &tape_block.into())?;
        Ok(())
    }

    #[wasm_bindgen]
    pub fn wasm_get_image(
        game: *mut Game<JSGui>,
        callback: js_sys::Function,
    ) -> Result<(), JsValue> {
        let game = unsafe { &mut *game };

        let (w, h, screen_data) = game.get_screen();

        let pixels = {
            let ptr = screen_data.as_ptr() as *const u8;
            let len = mem::size_of_val(screen_data);
            let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
            // some browsers need a Uint8ClampedArray in non-webgl mode
            js_sys::Uint8ClampedArray::new_from_slice(bytes)
        };

        callback.call3(&JsValue::NULL, &w.into(), &h.into(), &pixels)?;
        Ok(())
    }

    #[wasm_bindgen]
    pub fn wasm_get_audio(
        game: *mut Game<JSGui>,
        callback: js_sys::Function,
    ) -> Result<(), JsValue> {
        let game = unsafe { &mut *game };

        let audio_data = game.get_audio();
        let audio = js_sys::Float32Array::new_from_slice(audio_data);

        callback.call1(&JsValue::NULL, &audio.into())?;
        Ok(())
    }

    #[wasm_bindgen]
    pub fn wasm_tape_load(game: *mut Game<JSGui>, data: &[u8]) -> Result<usize, JsError> {
        let game = unsafe { &mut *game };
        game.tape_load(data)
            .map_err(|e| JsError::new(&format!("Tape error: {e}")))
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
    pub fn wasm_disk_load(game: *mut Game<JSGui>, data: &[u8]) -> Result<(), JsError> {
        let game = unsafe { &mut *game };
        game.load_disk(data)
            .map_err(|e| JsError::new(&format!("Disk error: {e}")))
    }
    #[wasm_bindgen]
    pub fn wasm_disk_eject(game: *mut Game<JSGui>) {
        let game = unsafe { &mut *game };
        let _ = game.eject_disk();
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
