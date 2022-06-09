#![allow(dead_code)]

mod images;

use images::*;
use std::fs::{File, self};
use std::io::Write;
use log::error;
use pixels::{Error, Pixels, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event::{Event, VirtualKeyCode};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

//Config
const WIDTH:         u32 = 960;
const HEIGHT:        u32 = 736;
const CHUNK_SIZE:    u32 = 32;
const SIM_WIDTH:     u32 = 1280;
const SIM_HEIGHT:    u32 = 960;

//Color
const WHITE:  &[u8; 4] = &[0xff, 0xff, 0xff, 0xff];
const BLACK:  &[u8; 4] = &[0x00, 0x00, 0x00, 0xff];
const SELECT: &[u8; 4] = &[0xff, 0xff, 0xff, 0x64];

//Paths
const EXPORT:  &str = "map.txt";
const EX_JSON: &str = "map.json";

#[derive(Clone, Copy)]
enum ChunkType {
    Air,
    Dirt,
    Grass,
    CheckPoint,
    Spikes,
    TallGrass,
    Stone,
    Planks,
    CrackedStone
}

impl From<u8> for ChunkType {
    fn from(n: u8) -> Self {
        match n {
            1 => ChunkType::Dirt,
            2 => ChunkType::Grass,
            3 => ChunkType::CheckPoint,
            4 => ChunkType::Spikes,
            5 => ChunkType::TallGrass,
            6 => ChunkType::Stone,
            7 => ChunkType::Planks,
            8 => ChunkType::CrackedStone,
            _ => ChunkType::Air
        }
    }
}

struct World {
    chunks: [[ChunkType; (SIM_WIDTH/CHUNK_SIZE) as usize]; (SIM_HEIGHT/CHUNK_SIZE) as usize],
    offset: (usize, usize),
    tmp_chunk: (usize, usize),
    spawn_chunk: (usize, usize),
    chunk_type: ChunkType
}

fn main() -> Result<(), Error> {
    env_logger::init();
    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();
    let window = {
        let size = LogicalSize::new(WIDTH as f64, HEIGHT as f64);
        WindowBuilder::new()
            .with_title("Hello Pixels")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        Pixels::new(WIDTH, HEIGHT, surface_texture)?
    };
    let mut world = World::new();

    event_loop.run(move |event, _, control_flow| {
        if let Event::RedrawRequested(_) = event {
            world.draw(pixels.get_frame());
            if pixels
                .render()
                .map_err(|e| error!("pixels.render() failed: {}", e))
                .is_err()
            {
                *control_flow = ControlFlow::Exit;
                return;
            }
        }

        if input.update(&event) {
            if input.key_pressed(VirtualKeyCode::Escape) || input.quit() {
                *control_flow = ControlFlow::Exit;
                return;
            }

            if input.key_pressed(VirtualKeyCode::Return) {
                world.save();
            }

            if input.mouse_held(0) {
                world.imprint();
            }

            if input.mouse_held(1) {
                world.set_spawn(input.clone());
            }

            if let Some(size) = input.window_resized() {
                pixels.resize_surface(size.width, size.height);
            }

            world.set_offset(input.clone());
            world.set_chunk_type(input.clone());
            world.update(input.clone());

            window.request_redraw();
        }
    });
}

impl World {
    fn new() -> Self {
        World {
            chunks: [[ChunkType::Air; (SIM_WIDTH/CHUNK_SIZE) as usize]; (SIM_HEIGHT/CHUNK_SIZE) as usize],
            offset: (0, 0),
            tmp_chunk: (0, 0),
            spawn_chunk: (0, 0),
            chunk_type: ChunkType::Dirt
        }
    }

    fn update(&mut self, input: WinitInputHelper) {
        if let Some((x, y)) = input.mouse() {
            self.tmp_chunk = (y as usize/CHUNK_SIZE as usize, x as usize/CHUNK_SIZE as usize);
        }
    }

    fn imprint(&mut self) {
        let (mut y,mut x) = self.tmp_chunk;
        x += self.offset.1 / CHUNK_SIZE as usize;
        y += self.offset.0 / CHUNK_SIZE as usize;
        x = x.clamp(0, (SIM_WIDTH/CHUNK_SIZE) as usize-1);
        y = y.clamp(0, (SIM_HEIGHT/CHUNK_SIZE) as usize-1);
        self.chunks[y][x] = self.chunk_type;
    }

    fn set_spawn(&mut self, input: WinitInputHelper) {
        if let Some((x, y)) = input.mouse() {
            self.spawn_chunk = (y as usize/CHUNK_SIZE as usize * 16, x as usize/CHUNK_SIZE as usize * 16);
        }
    }

    fn set_offset(&mut self, input: WinitInputHelper) {
        let mut offset = (self.offset.0 as isize, self.offset.1 as isize);
        if input.key_held(VirtualKeyCode::Right) && offset.1 < (SIM_WIDTH - CHUNK_SIZE) as isize {
            offset.1 += CHUNK_SIZE as isize;
        } else if input.key_held(VirtualKeyCode::Left) {
            offset.1 -= CHUNK_SIZE as isize;
        } else if input.key_held(VirtualKeyCode::Up) {
            offset.0 -= CHUNK_SIZE as isize;
        } else if input.key_held(VirtualKeyCode::Down) {
            offset.0 += CHUNK_SIZE as isize;
        }
        offset.0 = offset.0.clamp(0, (SIM_HEIGHT - HEIGHT) as isize);
        offset.1 = offset.1.clamp(0, (SIM_WIDTH  - WIDTH)  as isize);
        self.offset = (offset.0 as usize, offset.1 as usize);
    }

    fn set_chunk_type(&mut self, input: WinitInputHelper) {
        if input.key_pressed(VirtualKeyCode::Key0) {
            self.chunk_type = ChunkType::Air;
        } else if input.key_pressed(VirtualKeyCode::Key1) {
            self.chunk_type = ChunkType::Dirt;
        } else if input.key_pressed(VirtualKeyCode::Key2) {
            self.chunk_type = ChunkType::Grass;
        } else if input.key_pressed(VirtualKeyCode::Key3) {
            self.chunk_type = ChunkType::CheckPoint;
        } else if input.key_pressed(VirtualKeyCode::Key4) {
            self.chunk_type = ChunkType::Spikes;
        } else if input.key_pressed(VirtualKeyCode::Key5) {   
            self.chunk_type = ChunkType::TallGrass;
        } else if input.key_pressed(VirtualKeyCode::Key6) {   
            self.chunk_type = ChunkType::Stone;
        } else if input.key_pressed(VirtualKeyCode::Key7) {   
            self.chunk_type = ChunkType::Planks;
        } else if input.key_pressed(VirtualKeyCode::Key8) {   
            self.chunk_type = ChunkType::CrackedStone;
        } 
    }

    fn draw(&self, frame: &mut [u8]) {
        let chunk_choice = match self.chunk_type {
            ChunkType::Air          => [[*WHITE; 32]; 32],
            ChunkType::Dirt         => DIRT,
            ChunkType::Grass        => GRASS,
            ChunkType::CheckPoint   => CHECKPOINT,
            ChunkType::Spikes       => SPIKES,
            ChunkType::TallGrass    => TALL_GRASS,
            ChunkType::Stone        => STONE,
            ChunkType::Planks       => PLANKS,
            ChunkType::CrackedStone => CRACKED_STONE
        };

        for (chk, pix) in (0..WIDTH*HEIGHT).zip(frame.chunks_exact_mut(4)) {
            let mut x = (chk % WIDTH) as usize;
            let mut y = (chk / WIDTH) as usize;
            x += self.offset.1;
            y += self.offset.0;
            
            let chunk = self.chunks[y/CHUNK_SIZE as usize][x/CHUNK_SIZE as usize];

            let mut rgba = match chunk {
                ChunkType::Air          => *WHITE,
                ChunkType::Dirt         => image_pixels(DIRT, x, y),
                ChunkType::Grass        => image_pixels(GRASS, x, y),
                ChunkType::CheckPoint   => image_pixels(CHECKPOINT, x, y),
                ChunkType::Spikes       => image_pixels(SPIKES, x, y),
                ChunkType::TallGrass    => image_pixels(TALL_GRASS, x, y),
                ChunkType::Stone        => image_pixels(STONE, x, y),
                ChunkType::Planks       => image_pixels(PLANKS, x, y),
                ChunkType::CrackedStone => image_pixels(CRACKED_STONE, x, y)
            };
            
            if self.tmp_chunk == ((y - self.offset.0)/CHUNK_SIZE as usize, (x - self.offset.1)/CHUNK_SIZE as usize) {
                rgba = image_pixels(chunk_choice, x, y);
                rgba[3] = 0x64;
            }

            if self.spawn_chunk == (y/CHUNK_SIZE as usize*16, x/CHUNK_SIZE as usize*16) && image_pixels(SPAWN, x, y)[3] != 0 {
                rgba = image_pixels(SPAWN, x, y);
            }
            
            pix.copy_from_slice(&rgba);
        }
    }

    fn save(&self) {
        if File::open(EXPORT).is_ok() {
            fs::remove_file(EXPORT).unwrap();
        }
        let mut file = File::create(EXPORT).unwrap();
        for h in self.chunks.iter() {
            for w in h.iter() {
                write!(file, "{}", *w as u8).unwrap();
            }
            writeln!(file).unwrap();
        }

        if File::open(EX_JSON).is_ok() {
            fs::remove_file(EX_JSON).unwrap();
        }
        let mut file = File::create(EX_JSON).unwrap();
        writeln!(file, "{{").unwrap();
        writeln!(file, "\t\"x\": {},", self.spawn_chunk.1).unwrap();
        writeln!(file, "\t\"y\": {},", self.spawn_chunk.0).unwrap();
        writeln!(file, "\t\"depth\": 750").unwrap();
        writeln!(file, "}}").unwrap();
    }
}

fn image_pixels(image: [[[u8; 4]; 32]; 32], x: usize, y: usize) -> [u8; 4] {
    image[y%CHUNK_SIZE as usize][x%CHUNK_SIZE as usize]
}