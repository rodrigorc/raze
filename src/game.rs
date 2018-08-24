use js::{*, Canvas::*};
use z80::{Z80, InOut};
use memory::Memory;

#[repr(C)]
#[derive(Copy, Clone)]
struct Pixel(u8,u8,u8,u8);

struct IO {
    keys: [[bool; 5]; 8],
    frame_counter: u32,
}

pub struct Game {
    memory: Memory,
    z80: Z80,
    io: IO,
    image: Vec<Pixel>,
}

impl InOut for IO {
    fn do_in(&mut self, port: u16) -> u8 {
        let lo = port as u8;
        let hi = (port >> 8) as u8;
        let mut r = 0xff;
        //ULA IO port
        if lo & 1 == 0 {
            for i in 0..8 { //half row keyboard
                if hi & (1 << i) == 0 {
                    for j in 0..5 { //keys
                        if self.keys[i][j] {
                            r &= !(1 << j);
                        }
                    }
                }
            }
            if self.frame_counter % 200 < 100 {
                r &= 0b1011_1111; //cassette
            }
        }
        //log!("IN {:04x}, {:02x}", port, r);
        r
    }
    fn do_out(&mut self, port: u16, value: u8) {
        log!("OUT {:04x}, {:02x}", port, value);
        let lo = port as u8;
        let hi = (port >> 8) as u8;
        //ULA IO port
        if lo & 1 == 0 {
            let c = value & 7;
            Bg.fillStyle(["#000", "#00a", "#a00", "#a0a", "#0a0", "#0aa", "#aa0", "#aaa"][c as usize]);
            Bg.fillRect(0.0, 0.0, 800.0, 600.0);
        }
    }
}

fn write_screen(inv: bool, data: &[u8], ps: &mut [Pixel]) {
    assert!(ps.len() == 256 * 192);
    for y in 0..192 {
        let orow = match y {
            0..=63 => {
                let y = (y % 8) * 256 + (y / 8) * 32;
                y
            }
            64..=127 => {
                let y = y - 64;
                let y = (y % 8) * 256 + (y / 8) * 32;
                y + 64 * 32
            }
            128..=191 => {
                let y = y - 128;
                let y = (y % 8) * 256 + (y / 8) * 32;
                y + 128 * 32
            }
            _ => unreachable!()
        };
        for x in 0..32 {
            let attr = data[192 * 32 + (y / 8) * 32 + x];
            let d = data[orow + x];
            for b in 0..8 {
                let pix = ((d >> (7-b)) & 1) != 0;
                let v = if (attr & 0b0100_0000) != 0 { 0xff } else { 0xaa };
                let blink = inv && (attr & 0b1000_0000) != 0;

                let c = if pix ^ blink {
                    (attr & 0b0000_0111)
                } else {
                    (attr & 0b0011_1000) >> 3
                };
                let offs = 256 * y + 8 * x + b;
                ps[offs] = Pixel(
                    if (c & 2) != 0 { v } else { 0 },
                    if (c & 4) != 0 { v } else { 0 },
                    if (c & 1) != 0 { v } else { 0 },
                    0xff,
                    );
            }
        }
    }
}

impl Game {
    pub fn new() -> Box<Game> {
        log!("Go!");
        Bg.clearRect(0.0, 0.0, 800.0, 600.0);
        let mut memory = Memory::new_from_bytes(include_bytes!("48k.rom"));
        let mut z80 = Z80::new();
        let game = Game{
            memory, z80,
            io: IO { keys: Default::default(), frame_counter: 0 },
            image: vec![Pixel(0,0,0,0xff); 256 * 192],
        };
        game.into()
    }
    pub fn draw_frame(&mut self) {
        //log!("Draw!");
        self.io.frame_counter = self.io.frame_counter.wrapping_add(1);
        const NUM_OPS : i32 = 10_000;
        for _ in 0..NUM_OPS {
            /*if self.io.keys[0][0] {
                self.z80.dump_regs();
            }*/
            self.z80.exec(&mut self.memory, &mut self.io);
        }
        self.z80.interrupt(&mut self.memory);
        let screen = self.memory.slice(0x4000, 0x4000 + 32 * 192 + 32 * 24);
        write_screen(self.io.frame_counter % 32 < 16, screen, &mut self.image);
        Fg.putImageData(256, 192, &self.image);
    }
    pub fn mouse_move(&mut self, _x: f32, _y: f32) {
    }
    pub fn mouse_up(&mut self, _x: f32, _y: f32) {
    }
    pub fn mouse_down(&mut self, _x: f32, _y: f32) {
    }
    pub fn key_up(&mut self, mut key: usize) {
        while key != 0 {
            let k = key & 0x07;
            let r = (key >> 4) & 0x07;
            self.io.keys[r][k] = false;
            key >>= 8;
        }
    }
    pub fn key_down(&mut self, mut key: usize) {
        while key != 0 {
            let k = key & 0x07;
            let r = (key >> 4) & 0x07;
            self.io.keys[r][k] = true;
            key >>= 8;
        }
    }
}

