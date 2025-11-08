use anyhow::Result;
use cpal::{SampleFormat, SampleRate, Stream, traits::DeviceTrait};
use easy_imgui::{
    ChildFlags, Color, ColorId, Cond, Dir, DockNodeFlags, DrawFlags, InputFlags, Key, MouseCursor,
    SelectableFlags, TextWrapPos, TextureRef, UiBuilder, VEC2_ZERO, Vector2, WindowClass,
    WindowFlags,
    easy_imgui_sys::{self, ImVec2},
    id, lbl_id, vec2,
};
use easy_imgui_filechooser::{FileChooser, glob};

use easy_imgui_opengl::{
    Texture,
    glow::{self, HasContext},
};
use easy_imgui_sdl3::Application;
use fancy_duration::FancyDuration;
use fftw::plan::R2CPlan;
use sdl3::video::{GLProfile, SwapInterval, WindowPos};
use std::{
    cell::Cell,
    collections::VecDeque,
    path::PathBuf,
    slice,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use zxspectrum_raze::{self as raze, Game};

struct App {
    redock: bool,
    game: raze::Game<GameUi>,
    gui: GameUi,
    cursor_mode: CursorMode,
    turbo: bool,
    pause: bool,
    fullscreen: bool,
    snapshots: Vec<Snapshot>,
    snapshot_sel: Option<usize>,
    last_snapshot_id: usize,
    fd_atlas: easy_imgui_filechooser::CustomAtlas,
    file_dialog: Option<AppFileDialog>,
    fd_tape_path: PathBuf,
    fd_snapshot_path: PathBuf,

    modal_message: Option<ModalMessage>,
}

struct ModalMessage {
    title: String,
    message: String,
    on_yes: Option<Box<dyn FnOnce() -> UiAction>>,
}

impl ModalMessage {
    fn error(message: impl Into<String>) -> ModalMessage {
        ModalMessage {
            title: String::from("Error"),
            message: message.into(),
            on_yes: None,
        }
    }
    fn confirm(
        message: impl Into<String>,
        action: impl FnOnce() -> UiAction + 'static,
    ) -> ModalMessage {
        ModalMessage {
            title: String::from("Confirm"),
            message: message.into(),
            on_yes: Some(Box::new(action)),
        }
    }
}

struct Snapshot {
    name: String,
    data: Vec<u8>,
    ts: std::time::Instant,
}

struct AppFileDialog {
    fd: FileChooser,
    title: String,
    default_extension: Option<&'static str>,
    on_ok: Box<dyn Fn(PathBuf) -> UiAction>,
}

#[derive(thiserror::Error, Debug)]
enum SaveError {
    #[error("Overwrite")]
    ConfirmOverwrite,
    #[error("{0}")]
    Other(#[from] std::io::Error),
}

struct GameUi {
    texture: Texture,
    size: Cell<Vector2>,
    audio_buffer: Arc<Mutex<AudioBuffer>>,
    do_sound_ft: bool,
    plan_ft: fftw::plan::R2CPlan32,
    audio_ft: Vec<f32>,
    #[allow(dead_code)]
    audio: Stream,
}

struct AudioBuffer {
    // skip these bytes from the beginning of the first block
    offset: usize,
    data: VecDeque<AudioBlock>,
}

struct AudioBlock {
    // (22050 samples/s) / 50 (frames / s) = 440 samples / frame, approx.
    // round up to 500
    data: [f32; 500],
    length: usize,
}

const AUDIO_FT_BLOCK: usize = 430;

impl Default for AudioBuffer {
    fn default() -> Self {
        AudioBuffer {
            offset: 0,
            data: VecDeque::with_capacity(3),
        }
    }
}

impl Default for AudioBlock {
    fn default() -> Self {
        Self::from([].as_slice())
    }
}

impl From<&[f32]> for AudioBlock {
    fn from(src: &[f32]) -> Self {
        let mut res = AudioBlock {
            data: [0.0; 500],
            length: src.len(),
        };
        res.data[..res.length].copy_from_slice(src);
        res
    }
}

impl AudioBuffer {
    fn fill_buffer(&mut self, mut data: &mut [f32]) {
        while !data.is_empty() {
            // get the first block
            let Some(block) = self.data.front() else {
                // if the audio underflows, write all 0.0
                //println!("underflow {}", data.len());
                data.fill(0.0);
                return;
            };
            // skip the offset if any
            let buf = &block[self.offset..];
            // how many samples
            let len = data.len().min(buf.len());
            // copy!
            data[..len].copy_from_slice(&buf[..len]);
            // consume the output buffer
            data = &mut data[len..];
            // consume the input buffer
            if len < buf.len() {
                // the block is not complete, advance the offset
                self.offset += len;
            } else {
                // the block is consumed, remove it
                self.data.pop_front();
                self.offset = 0;
            }
        }
    }
    #[allow(dead_code)]
    fn len(&self) -> usize {
        self.data.iter().map(|d| d.length).sum::<usize>() - self.offset
    }
}

impl<I: std::slice::SliceIndex<[f32]>> std::ops::Index<I> for AudioBlock {
    type Output = I::Output;
    fn index(&self, index: I) -> &I::Output {
        &self.data[..self.length][index]
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum CursorMode {
    CursorKeys = 0,
    Kempston = 1,
    Sinclair1 = 2,
    Sinclair2 = 3,
    Protek = 4,
}

static CURSOR_KEYS: [[usize; 5]; 5] = [
    //cursorkeys
    [0x0834, 0x0842, 0x0844, 0x0843, 0x71], //Shift+{5,8,6,7}, SymbolShift
    //kempston
    [0x81, 0x80, 0x82, 0x83, 0x84],
    //sinclair1
    [0x44, 0x43, 0x42, 0x41, 0x40], //6, 7, 8, 9, 0
    //sinclair2
    [0x30, 0x31, 0x32, 0x33, 0x34], //1, 2, 3, 4, 5
    //protek
    [0x34, 0x42, 0x44, 0x43, 0x40], //5, 8, 6, 7, 0
];

impl raze::Gui for GameUi {
    type Pixel = [u8; 3];

    const PALETTE: [[[u8; 3]; 8]; 2] = [
        [
            [0x00, 0x00, 0x00],
            [0x00, 0x00, 0xd7],
            [0xd7, 0x00, 0x00],
            [0xd7, 0x00, 0xd7],
            [0x00, 0xd7, 0x00],
            [0x00, 0xd7, 0xd7],
            [0xd7, 0xd7, 0x00],
            [0xd7, 0xd7, 0xd7],
        ],
        [
            [0x00, 0x00, 0x00],
            [0x00, 0x00, 0xff],
            [0xff, 0x00, 0x00],
            [0xff, 0x00, 0xff],
            [0x00, 0xff, 0x00],
            [0x00, 0xff, 0xff],
            [0xff, 0xff, 0x00],
            [0xff, 0xff, 0xff],
        ],
    ];

    fn on_rzx_running(&mut self, _running: bool, _percent: u32) {}

    fn on_tape_block(&mut self, _index: usize) {}

    fn put_sound_data(&mut self, data: &[f32]) {
        let mut ab = self.audio_buffer.lock().unwrap();
        ab.data.push_back(AudioBlock::from(data));
        if self.do_sound_ft
            && let Some(data) = data.get(0..AUDIO_FT_BLOCK)
        {
            let mut fdata = fftw::array::AlignedVec::new(data.len());
            fdata.copy_from_slice(data);
            let mut ft = fftw::array::AlignedVec::new(data.len() / 2 + 1);
            self.plan_ft.r2c(&mut fdata, &mut ft).unwrap();
            self.audio_ft = ft[1..]
                .iter()
                .map(|c| c.norm()) // / 8.879 * (-0.2358 * (i + 1) as f32).exp() + 0.1954
                .collect();
        } else {
            self.audio_ft = Vec::new();
        }
    }

    fn put_image_data(&mut self, w: usize, h: usize, data: &[Self::Pixel]) {
        let gl = self.texture.gl();
        unsafe {
            gl.bind_texture(glow::TEXTURE_2D, Some(self.texture.id()));
            let ptr = data.as_ptr();
            let len = std::mem::size_of_val(data);
            let bytes = slice::from_raw_parts(ptr as *const u8, len);
            gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGB as i32,
                w as i32,
                h as i32,
                0,
                glow::RGB,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(bytes)),
            );
            gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 4);
            gl.bind_texture(glow::TEXTURE_2D, None);
        }
        self.size.set(vec2(w as f32, h as f32));
    }
}

impl Application for App {
    type UserEvent = ();

    type Data = ();

    fn new(args: easy_imgui_sdl3::Args<'_, Self>) -> Self {
        let gl = args.gl;
        let texture = Texture::generate(gl).unwrap();

        unsafe {
            gl.bind_texture(glow::TEXTURE_2D, Some(texture.id()));
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAX_LEVEL, 0);
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::LINEAR as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::LINEAR as i32,
            );
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::REPEAT as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::REPEAT as i32);
            gl.bind_texture(glow::TEXTURE_2D, None);
        }

        use cpal::traits::HostTrait;
        let host = cpal::default_host();
        let snd_dev = host
            .default_output_device()
            .expect("no audio output available");
        let mut cfs = snd_dev.supported_output_configs().unwrap();
        let output_config = cfs
            .find(|c| c.channels() == 1 && c.sample_format() == SampleFormat::F32)
            .expect("no compatible audio configuration");
        let output_config = output_config.with_sample_rate(SampleRate(22050));
        let mut snd_cfg = output_config.config();
        snd_cfg.buffer_size = cpal::BufferSize::Fixed(512);

        let audio_buffer = Arc::new(Mutex::new(AudioBuffer::default()));
        let audio = snd_dev
            .build_output_stream(
                dbg!(&snd_cfg),
                {
                    let audio_buffer = Arc::clone(&audio_buffer);
                    move |data: &mut [f32], _info| {
                        let mut ab = audio_buffer.lock().unwrap();
                        ab.fill_buffer(data);
                    }
                },
                |error| {
                    eprintln!("{error}");
                },
                None,
            )
            .unwrap();
        let mut gui = GameUi {
            texture,
            size: Cell::new(vec2(4.0, 3.0)),
            audio_buffer,
            do_sound_ft: false,
            plan_ft: fftw::plan::R2CPlan32::aligned(
                std::slice::from_ref(&AUDIO_FT_BLOCK),
                fftw::types::Flag::MEASURE,
            )
            .unwrap(),
            audio_ft: Vec::new(),
            audio,
        };

        let atlas = args.imgui.io_mut().font_atlas_mut();
        let fd_atlas = easy_imgui_filechooser::build_custom_atlas(atlas);
        let game = raze::Game::new(/*is128k*/ true, &mut gui);

        App {
            redock: true,
            game,
            gui,
            cursor_mode: CursorMode::CursorKeys,
            turbo: false,
            pause: false,
            fullscreen: false,
            snapshots: Vec::new(),
            snapshot_sel: None,
            last_snapshot_id: 0,
            fd_atlas,
            file_dialog: None,
            fd_tape_path: PathBuf::from("."),
            fd_snapshot_path: PathBuf::from("."),
            modal_message: None,
        }
    }

    fn post_frame(&mut self, mut args: easy_imgui_sdl3::Args<'_, Self>) {
        args.ping_user_input();
        let _ = args.window.set_fullscreen(self.fullscreen);
    }

    fn sdl3_event(
        &mut self,
        mut args: easy_imgui_sdl3::Args<'_, Self>,
        event: sdl3::event::Event,
        res: &mut easy_imgui::EventResult,
    ) {
        if res.window_closed {
            //args.event_loop.exit();
        }
        args.ping_user_input();
        if !res.want_capture_keyboard {
            use sdl3::event::Event;
            match event {
                Event::KeyDown {
                    scancode: Some(scancode),
                    ..
                } => {
                    if let Some(key) = self.map_key(scancode) {
                        self.game.key_down(key);
                    }
                }
                Event::KeyUp {
                    scancode: Some(scancode),
                    ..
                } => {
                    if let Some(key) = self.map_key(scancode) {
                        self.game.key_up(key);
                    }
                }
                Event::JoyButtonDown { button_idx, .. } => {
                    if button_idx == 0 {
                        self.game.key_down(CURSOR_KEYS[1][4]);
                    }
                }
                Event::ControllerButtonDown { button, .. } => {
                    if let Some(btn) = self.map_button(button) {
                        self.game.key_down(btn);
                    }
                }
                Event::ControllerButtonUp { button, .. } => {
                    if let Some(btn) = self.map_button(button) {
                        self.game.key_up(btn);
                    }
                }
                _ => {}
            }
        }
    }
}

#[derive(Debug)]
enum UiAction {
    None,
    Reset { is128k: bool },
    TapeLoadDlg,
    TapeLoad(PathBuf),
    TapeStop,
    SnapshotLoadDlg,
    SnapshotLoad(PathBuf),
    SnapshotSaveDlg(usize),
    SnapshotSave(PathBuf, usize, bool), // (file, index, force_overwrite)
    SnapshotDo,
    SnapshotRestore(usize),
    SnapshotDelete(usize),
}

impl UiBuilder for App {
    fn pre_render(&mut self, _ctx: &mut easy_imgui::CurrentContext<'_>) {
        if !self.pause {
            while self.gui.audio_buffer.lock().unwrap().data.len() < 3 {
                self.game.draw_frame(self.turbo, &mut self.gui);
                if self.turbo {
                    break;
                }
            }
        }
    }
    fn do_ui(&mut self, ui: &easy_imgui::Ui<Self>) {
        // The main user actil will be recorded here and done later, to avoid mutating the game in the middle
        // of the frame. And also, it makes the UI code easier to understand.
        let mut ui_action = UiAction::None;

        let vp = ui.get_main_viewport();
        let tex_image = TextureRef::Id(easy_imgui_sdl3::map_tex(self.gui.texture.id()));
        let gui_size = self.gui.size.get();

        if ui.shortcut_ex(Key::F11, InputFlags::RouteGlobal) {
            self.fullscreen = !self.fullscreen;
        }
        if ui.shortcut_ex(Key::F2, InputFlags::RouteGlobal) {
            self.pause = !self.pause;
        }
        if ui.is_key_down(Key::F10) {
            self.turbo = true;
        } else if ui.is_key_released(Key::F10) {
            self.turbo = false;
        }
        if ui.shortcut_ex(Key::F5, InputFlags::RouteGlobal) {
            ui_action = UiAction::SnapshotDo;
        }
        if ui.shortcut_ex(Key::F9, InputFlags::RouteGlobal)
            && let Some(sel) = self.snapshot_sel
        {
            ui_action = UiAction::SnapshotRestore(sel)
        }

        if self.fullscreen {
            if ui.shortcut_ex(Key::Escape, InputFlags::RouteGlobal) {
                self.fullscreen = false;
            }
            ui.set_mouse_cursor(MouseCursor::None);
            let mut pos = vp.pos();
            let mut size = vp.size();
            if size.x * gui_size.y > size.y * gui_size.x {
                let x = size.y * gui_size.x / gui_size.y;
                pos.x += (size.x - x) / 2.0;
                size.x = x;
            } else {
                let y = size.x * gui_size.y / gui_size.x;
                pos.y += (size.y - y) / 2.0;
                size.y = y;
            }
            ui.viewport_background_draw_list(vp).add_image(
                tex_image,
                pos,
                pos + size,
                VEC2_ZERO,
                vec2(1.0, 1.0),
                Color::WHITE,
            );
            self.run_ui_action(ui_action);
            return;
        }

        let id_root = ui.dock_space_over_viewport(0, vp, easy_imgui::DockNodeFlags::None, None);
        if self.redock {
            self.redock = false;
            ui.dock_builder(Some(id_root), DockNodeFlags::None, |dock_id, builder| {
                builder.set_node_size(dock_id, vp.size());
                let (dock1, d_tape) = builder.split_node(dock_id, Dir::Right, 0.8);
                let (d_display, d_control) = builder.split_node(dock1, Dir::Up, 0.8);

                builder.dock_window(id("display"), d_display);
                builder.dock_window(id("tape"), d_tape);
                builder.dock_window(id("snapshots"), d_tape);
                builder.dock_window(id("control"), d_control);
                builder.dock_window(id("sound"), d_control);
            });
        }

        //ui.show_demo_window(None);

        ui.window_config(lbl_id("Tape", "tape")).with(|| {
            if ui.button(lbl_id("Load...", "load")) {
                ui_action = UiAction::TapeLoadDlg;
            }

            if ui.button(lbl_id("Stop", "stop")) {
                ui_action = UiAction::TapeStop;
            }

            ui.child_config(lbl_id("Blocks", "blocks"))
                .child_flags(ChildFlags::FrameStyle)
                .with(|| {
                    if let Some((len, pos)) = self.game.tape_len_and_pos() {
                        for i in 0..len {
                            let name = lbl_id(self.game.tape_name(i), format!("block_{i}"));
                            let (current, percent) = match pos {
                                Some((ii, pp)) if ii == i => (true, pp),
                                _ => (false, 0.0),
                            };
                            let selectable = self.game.tape_selectable(i);
                            if !selectable {
                                ui.set_cursor_pos_x(ui.get_cursor_pos_x() + 16.0);
                            }
                            if ui
                                .selectable_config(name)
                                .flags(
                                    if current {
                                        SelectableFlags::Highlight
                                    } else {
                                        SelectableFlags::empty()
                                    } | if selectable {
                                        SelectableFlags::empty()
                                    } else {
                                        SelectableFlags::Disabled
                                    },
                                )
                                .build()
                            {
                                self.game.tape_seek(i, &mut self.gui);
                            }
                            if current {
                                let r0 = ui.get_item_rect_min();
                                let sz = ui.get_item_rect_size();
                                let r1 = vec2(r0.x + sz.x * percent, r0.y + sz.y);
                                ui.window_draw_list().add_rect_filled(
                                    r0,
                                    r1,
                                    Color::new(0.0, 1.0, 0.0, 0.3),
                                    0.0,
                                    DrawFlags::None,
                                );
                            }
                        }
                    }
                });
        });

        ui.window_config(lbl_id("Snapshots", "snapshots")).with(|| {
            if ui.button(lbl_id("Load...", "load")) {
                ui_action = UiAction::SnapshotLoadDlg;
            }

            if ui.button(lbl_id("Snapshot!", "snapshot")) {
                ui_action = UiAction::SnapshotDo;
            }

            ui.same_line();

            ui.align_text_to_frame_padding();
            ui.text_disabled("(?)");
            ui.with_item_tooltip(|| {
                ui.with_push(TextWrapPos(ui.get_font_size() * 50.0), || {
                    ui.text("Right-click an item to save and other options.");
                });
            });

            if ui.shortcut(Key::Delete)
                && let Some(sel) = self.snapshot_sel
            {
                ui_action = UiAction::SnapshotDelete(sel);
            }

            ui.child_config(lbl_id("Snapshots", "snapshots"))
                .child_flags(ChildFlags::FrameStyle)
                .with(|| {
                    let now = Instant::now();
                    for (i, snapshot) in self.snapshots.iter().enumerate().rev() {
                        let ago = Duration::from_secs((now - snapshot.ts).as_secs());
                        let name = lbl_id(
                            format!("{} ({})", snapshot.name, FancyDuration(ago).truncate(2)),
                            format!("snapshot_{i}"),
                        );
                        let selected = Some(i) == self.snapshot_sel;
                        if ui
                            .selectable_config(name)
                            .flags(
                                if selected {
                                    SelectableFlags::Highlight
                                } else {
                                    SelectableFlags::empty()
                                } | SelectableFlags::AllowDoubleClick,
                            )
                            .build()
                        {
                            self.snapshot_sel = Some(i);
                            if ui.is_mouse_double_clicked(easy_imgui::MouseButton::Left) {
                                ui_action = UiAction::SnapshotRestore(i);
                            }
                        }
                        ui.popup_context_item_config().with(|| {
                            self.snapshot_sel = Some(i);
                            if ui.menu_item_config(lbl_id("Save...", "save")).build() {
                                ui_action = UiAction::SnapshotSaveDlg(i);
                            }
                            if ui.menu_item_config(lbl_id("Restore", "restore")).build() {
                                ui_action = UiAction::SnapshotRestore(i);
                            }
                            if ui.menu_item_config(lbl_id("Delete", "delete")).build() {
                                ui_action = UiAction::SnapshotDelete(i);
                            }
                        });
                    }
                });
        });

        ui.window_config(lbl_id("Control", "control"))
            .flags(WindowFlags::HorizontalScrollbar)
            .with(|| {
                if ui.button(lbl_id("Reset 128K", "reset_128")) {
                    ui_action = UiAction::Reset { is128k: true };
                }
                ui.same_line();
                if ui.button(lbl_id("Reset 48K", "reset_48")) {
                    ui_action = UiAction::Reset { is128k: false };
                }
                ui.with_push(
                    if self.pause {
                        Some([
                            (ColorId::Button, Color::RED),
                            (ColorId::ButtonHovered, Color::new(0.5, 0.0, 0.0, 1.0)),
                            (ColorId::ButtonActive, Color::new(0.75, 0.0, 0.0, 1.0)),
                        ])
                    } else {
                        None
                    },
                    || {
                        if ui.button(lbl_id("Pause (F2)", "pause")) {
                            self.pause = !self.pause;
                        }
                    },
                );
                ui.same_line();
                ui.with_push(
                    if self.turbo {
                        Some([
                            (ColorId::Button, Color::RED),
                            (ColorId::ButtonHovered, Color::new(0.5, 0.0, 0.0, 1.0)),
                            (ColorId::ButtonActive, Color::new(0.75, 0.0, 0.0, 1.0)),
                        ])
                    } else {
                        None
                    },
                    || {
                        if ui.button(lbl_id("Turbo (F10)", "turbo")) {
                            self.turbo = !self.turbo;
                        }
                    },
                );
                ui.same_line();
                if ui.button(lbl_id("Fullscreen (F11)", "fullcreen")) {
                    self.fullscreen = true;
                }

                ui.align_text_to_frame_padding();
                ui.text("Cursor keys");
                ui.same_line();
                ui.set_next_item_width(200.0);
                ui.combo(
                    lbl_id("", "cursor_keys"),
                    [
                        CursorMode::CursorKeys,
                        CursorMode::Kempston,
                        CursorMode::Sinclair1,
                        CursorMode::Sinclair2,
                        CursorMode::Protek,
                    ],
                    |cm| match cm {
                        CursorMode::CursorKeys => "Cursors",
                        CursorMode::Kempston => "Kempston",
                        CursorMode::Sinclair1 => "Sinclair #1",
                        CursorMode::Sinclair2 => "Sinclair #2",
                        CursorMode::Protek => "Protek",
                    },
                    &mut self.cursor_mode,
                );
            });

        let maybe_sound = ui.window_config(lbl_id("Sound", "sound")).with(|| {
            if !self.gui.audio_ft.is_empty() {
                unsafe {
                    easy_imgui_sys::ImGui_PlotLines(
                        id("sound").into().as_ptr(),
                        self.gui.audio_ft.as_ptr(),
                        self.gui.audio_ft.len() as i32,
                        0,
                        std::ptr::null(),
                        0.0,
                        2.0,
                        ImVec2 { x: -1.0, y: -1.0 },
                        std::mem::size_of::<f32>() as i32,
                    );
                }
            }
        });
        self.gui.do_sound_ft = maybe_sound.is_some();

        let display_class = WindowClass::default()
            .dock_node_flags(DockNodeFlags::NoDockingOverMe | DockNodeFlags::NoTabBar);
        ui.set_next_window_class(&display_class);
        ui.window_config(lbl_id("display", "display")).with(|| {
            let mut pos = vec2(0.0, 0.0); //vp.pos();
            let mut size = ui.get_content_region_avail();
            if size.x * gui_size.y > size.y * gui_size.x {
                let x = size.y * gui_size.x / gui_size.y;
                pos.x += (size.x - x) / 2.0;
                size.x = x;
            } else {
                let y = size.x * gui_size.y / gui_size.x;
                pos.y += (size.y - y) / 2.0;
                size.y = y;
            }
            ui.set_cursor_screen_pos(ui.get_cursor_screen_pos() + pos);
            ui.image_config(tex_image, size).build();
        });

        if let Some(file_dialog) = &mut self.file_dialog {
            let mut open = true;
            let mut closed = false;
            ui.set_next_window_size(vec2(600.0, 400.0), Cond::Appearing);
            ui.window_config(lbl_id(&file_dialog.title, "file_dialog"))
                .open(&mut open)
                .flags(WindowFlags::NoDocking)
                .with(|| {
                    let output = file_dialog.fd.do_ui(ui, &self.fd_atlas);
                    match output {
                        easy_imgui_filechooser::Output::Continue => {}
                        easy_imgui_filechooser::Output::Cancel => {
                            closed = true;
                        }
                        easy_imgui_filechooser::Output::Ok => {
                            ui_action = (file_dialog.on_ok)(
                                file_dialog.fd.full_path(file_dialog.default_extension),
                            );
                            // Do not close the file dialog until the action is completed, in case
                            // something goes wrong.
                        }
                    }
                });
            if !open || closed {
                self.file_dialog = None;
            }
        }

        if let Some(mut message) = self.modal_message.take() {
            let mut opened = true;
            let mut closed = false;
            ui.open_popup(id("error"));
            ui.popup_modal_config(lbl_id(&message.title, "error"))
                .opened(Some(&mut opened))
                .flags(WindowFlags::AlwaysAutoResize)
                .with(|| {
                    let font_sz = ui.get_font_size();
                    ui.text(&message.message);
                    ui.separator();
                    let btn_size = vec2(font_sz * 5.5, 0.0);
                    // f is FnOnce, so it is consumed when called:
                    // Take it, and then either call it or put it back.
                    if let Some(f) = message.on_yes.take() {
                        // Yes/No buttons
                        if ui
                            .button_config(lbl_id("Yes", "yes"))
                            .size(btn_size)
                            .build()
                        {
                            ui_action = f();
                            closed = true;
                        } else {
                            message.on_yes = Some(f);
                        }
                        ui.same_line();
                        if ui.button_config(lbl_id("No", "No")).size(btn_size).build() {
                            closed = true;
                        }
                    } else {
                        // Ok button
                        if ui.button_config(lbl_id("Ok", "ok")).size(btn_size).build() {
                            closed = true;
                        }
                    }
                    if closed {
                        ui.close_current_popup();
                    }
                });
            if opened && !closed {
                self.modal_message = Some(message);
            }
        }

        self.run_ui_action(ui_action);
    }
}

impl App {
    fn run_ui_action(&mut self, ui_action: UiAction) {
        // Do the action recorded above
        match ui_action {
            UiAction::None => {}
            UiAction::Reset { is128k } => {
                self.game = raze::Game::new(is128k, &mut self.gui);
            }
            UiAction::TapeLoadDlg => {
                let mut fd = FileChooser::new();
                fd.add_filter(easy_imgui_filechooser::Filter {
                    id: easy_imgui_filechooser::FilterId(0),
                    text: String::from("Tape files"),
                    globs: vec![
                        glob::Pattern::new("*.tap").unwrap(),
                        glob::Pattern::new("*.tzx").unwrap(),
                        glob::Pattern::new("*.zip").unwrap(),
                    ],
                });
                let _ = fd.set_path(&self.fd_tape_path);
                self.file_dialog = Some(AppFileDialog {
                    fd,
                    title: String::from("Open tape..."),
                    default_extension: None,
                    on_ok: Box::new(UiAction::TapeLoad),
                });
            }
            UiAction::TapeStop => {
                self.game.tape_stop();
            }
            UiAction::SnapshotLoadDlg => {
                let mut fd = FileChooser::new();
                fd.add_filter(easy_imgui_filechooser::Filter {
                    id: easy_imgui_filechooser::FilterId(0),
                    text: String::from("Snapshot files"),
                    globs: vec![
                        glob::Pattern::new("*.z80").unwrap(),
                        glob::Pattern::new("*.rzx").unwrap(),
                        glob::Pattern::new("*.zip").unwrap(),
                    ],
                });
                let _ = fd.set_path(&self.fd_snapshot_path);
                self.file_dialog = Some(AppFileDialog {
                    fd,
                    title: String::from("Open snapshot..."),
                    default_extension: Some("z80"),
                    on_ok: Box::new(UiAction::SnapshotLoad),
                });
            }
            UiAction::SnapshotSaveDlg(idx) => {
                let mut fd = FileChooser::new();
                fd.add_filter(easy_imgui_filechooser::Filter {
                    id: easy_imgui_filechooser::FilterId(0),
                    text: String::from("Snapshot files"),
                    globs: vec![glob::Pattern::new("*.z80").unwrap()],
                });
                let _ = fd.set_path(&self.fd_snapshot_path);
                self.file_dialog = Some(AppFileDialog {
                    fd,
                    title: String::from("Save snapshot..."),
                    default_extension: Some("z80"),
                    on_ok: Box::new(move |p| UiAction::SnapshotSave(p, idx, false)),
                });
            }
            UiAction::TapeLoad(path_buf) => {
                let mut load_file = || -> Result<()> {
                    let data = std::fs::read(&path_buf)?;
                    self.game.tape_load(data)?;
                    if let Some(path) = path_buf.parent() {
                        self.fd_tape_path = path.to_owned();
                    }
                    Ok(())
                };
                match load_file() {
                    // Close the file dialog
                    Ok(()) => self.file_dialog = None,
                    Err(e) => self.modal_message = Some(ModalMessage::error(format!("{e:#}"))),
                }
            }
            UiAction::SnapshotLoad(path_buf) => {
                let mut load_file = || -> Result<()> {
                    let data = std::fs::read(&path_buf)?;
                    let game = Game::load_snapshot(&data, &mut self.gui)?;
                    self.game = game;
                    self.add_snapshot(
                        path_buf
                            .file_name()
                            .map(|f| f.to_string_lossy().into_owned()),
                        data,
                    );
                    if let Some(path) = path_buf.parent() {
                        self.fd_snapshot_path = path.to_owned();
                    }
                    Ok(())
                };
                match load_file() {
                    // Close the file dialog
                    Ok(()) => self.file_dialog = None,
                    Err(e) => self.modal_message = Some(ModalMessage::error(format!("{e:#}"))),
                }
            }
            UiAction::SnapshotSave(path_buf, idx, overwrite) => {
                let mut save_file = || -> std::result::Result<(), SaveError> {
                    let snapshot = self
                        .snapshots
                        .get(idx)
                        .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))?;
                    if !overwrite && path_buf.exists() {
                        return Err(SaveError::ConfirmOverwrite);
                    }
                    std::fs::write(&path_buf, &snapshot.data)?;
                    if let Some(path) = path_buf.parent() {
                        self.fd_snapshot_path = path.to_owned();
                    }
                    Ok(())
                };
                match save_file() {
                    // Close the file dialog
                    Ok(()) => self.file_dialog = None,
                    Err(SaveError::ConfirmOverwrite) => {
                        let msg = format!(
                            "The file {} already exists.\n\nOverwrite?",
                            path_buf.file_name().unwrap_or_default().display()
                        );
                        self.modal_message = Some(ModalMessage::confirm(msg, move || {
                            UiAction::SnapshotSave(path_buf, idx, true)
                        }));
                    }
                    Err(e) => self.modal_message = Some(ModalMessage::error(format!("{e:#}"))),
                }
            }
            UiAction::SnapshotDo => {
                let data = self.game.snapshot();
                self.add_snapshot(None, data);
            }
            UiAction::SnapshotRestore(i) => {
                let game = self
                    .snapshots
                    .get(i)
                    .and_then(|s| Game::load_snapshot(&s.data, &mut self.gui).ok());
                if let Some(game) = game {
                    self.game = game;
                };
            }
            UiAction::SnapshotDelete(idx) => {
                self.snapshots.remove(idx);
                // usually the deleted item is the selected one
                let sel = self.snapshot_sel.unwrap_or(idx);
                // recompute the selected index, if needed
                if sel >= idx {
                    if sel > 0 {
                        self.snapshot_sel = Some(sel - 1);
                    } else if !self.snapshots.is_empty() {
                        self.snapshot_sel = Some(0);
                    } else {
                        self.snapshot_sel = None;
                    }
                }
            }
        }
    }
    fn map_button(&self, button: sdl3::gamepad::Button) -> Option<usize> {
        use sdl3::gamepad::Button;
        match button {
            Button::DPadLeft => Some(CURSOR_KEYS[CursorMode::Kempston as usize][0]),
            Button::DPadRight => Some(CURSOR_KEYS[CursorMode::Kempston as usize][1]),
            Button::DPadDown => Some(CURSOR_KEYS[CursorMode::Kempston as usize][2]),
            Button::DPadUp => Some(CURSOR_KEYS[CursorMode::Kempston as usize][3]),
            Button::South | Button::East | Button::North | Button::West => {
                Some(CURSOR_KEYS[CursorMode::Kempston as usize][4])
            }
            _ => None,
        }
    }
    fn map_key(&self, scancode: sdl3::keyboard::Scancode) -> Option<usize> {
        use sdl3::keyboard::Scancode;
        match scancode {
            Scancode::LShift | Scancode::RShift => Some(0x08), // should be 0x00
            Scancode::Z => Some(0x01),
            Scancode::X => Some(0x02),
            Scancode::C => Some(0x03),
            Scancode::V => Some(0x04),
            Scancode::A => Some(0x10),
            Scancode::S => Some(0x11),
            Scancode::D => Some(0x12),
            Scancode::F => Some(0x13),
            Scancode::G => Some(0x14),
            Scancode::Q => Some(0x20),
            Scancode::W => Some(0x21),
            Scancode::E => Some(0x22),
            Scancode::R => Some(0x23),
            Scancode::T => Some(0x24),
            Scancode::_1 => Some(0x30),
            Scancode::_2 => Some(0x31),
            Scancode::_3 => Some(0x32),
            Scancode::_4 => Some(0x33),
            Scancode::_5 => Some(0x34),
            Scancode::_0 => Some(0x40),
            Scancode::_9 => Some(0x41),
            Scancode::_8 => Some(0x42),
            Scancode::_7 => Some(0x43),
            Scancode::_6 => Some(0x44),
            Scancode::P => Some(0x50),
            Scancode::O => Some(0x51),
            Scancode::I => Some(0x52),
            Scancode::U => Some(0x53),
            Scancode::Y => Some(0x54),
            Scancode::Return => Some(0x60),
            Scancode::L => Some(0x61),
            Scancode::K => Some(0x62),
            Scancode::J => Some(0x63),
            Scancode::H => Some(0x64),
            Scancode::Space => Some(0x70),
            Scancode::RCtrl | Scancode::LAlt | Scancode::RAlt => Some(0x71),
            Scancode::M => Some(0x72),
            Scancode::N => Some(0x73),
            Scancode::B => Some(0x74),

            Scancode::Backspace => Some(0x0840),
            Scancode::Left => Some(CURSOR_KEYS[self.cursor_mode as usize][0]),
            Scancode::Right => Some(CURSOR_KEYS[self.cursor_mode as usize][1]),
            Scancode::Down => Some(CURSOR_KEYS[self.cursor_mode as usize][2]),
            Scancode::Up => Some(CURSOR_KEYS[self.cursor_mode as usize][3]),
            Scancode::LCtrl => Some(CURSOR_KEYS[self.cursor_mode as usize][4]),
            _ => None,
        }
    }

    fn add_snapshot(&mut self, name: Option<String>, data: Vec<u8>) {
        let name = name.unwrap_or_else(|| {
            self.last_snapshot_id += 1;
            format!("Snapshot #{}", self.last_snapshot_id)
        });
        self.snapshots.push(Snapshot {
            name,
            data,
            ts: Instant::now(),
        });
        self.snapshot_sel = Some(self.snapshots.len() - 1);
    }
}

fn main() {
    let sdl = sdl3::init().unwrap();
    sdl3::hint::set(sdl3::hint::names::QUIT_ON_LAST_WINDOW_CLOSE, "0");

    let sdl_video = sdl.video().unwrap();
    let sdl_event = sdl.event().unwrap();
    let _sdl_gamepad = sdl.gamepad().unwrap();

    let gla = sdl_video.gl_attr();
    gla.set_context_version(3, 2);
    gla.set_context_profile(GLProfile::Core);
    gla.set_depth_size(0);
    let main_scale = sdl_video
        .get_primary_display()
        .unwrap()
        .get_content_scale()
        .unwrap();

    let mut window = sdl_video
        .window(
            "R.A.Z.E.",
            (256.0 * 4.0 * main_scale) as u32,
            (192.0 * 4.0 * main_scale) as u32,
        )
        .opengl()
        .resizable()
        .hidden()
        .high_pixel_density()
        .build()
        .unwrap();
    let sdl_gl = window.gl_create_context().unwrap();
    window.gl_make_current(&sdl_gl).unwrap();
    let _ = sdl_video.gl_set_swap_interval(SwapInterval::VSync);
    window.set_position(WindowPos::Centered, WindowPos::Centered);
    window.show();

    let mut event_pump = sdl.event_pump().unwrap();

    let mut app_handler = easy_imgui_sdl3::AppHandler::<App>::new(
        &easy_imgui::ContextBuilder::new(),
        &sdl_event,
        window,
        sdl_gl,
        (),
    );
    //app_handler.imgui_mut().set_ini_file_name(Some("raze.ini"));

    let io = app_handler.imgui_mut().io_mut();

    io.enable_docking(true);
    io.enable_viewports(true);

    unsafe {
        io.inner().ConfigViewportsNoDefaultParent = false;
        io.inner().ConfigViewportsNoTaskBarIcon = true;
        //io.inner().ConfigViewportsNoAutoMerge = true;
    }

    app_handler.run(&mut event_pump);
}
