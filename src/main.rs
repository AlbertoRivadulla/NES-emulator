pub mod cpu;
pub mod opcodes;
pub mod bus;
pub mod cartridge;
pub mod trace;
pub mod ppu;
pub mod render;

// use crate::cpu::CPU;
// use crate::cpu::Mem;
use cpu::Mem;
use cpu::CPU;
use bus::Bus;
use cartridge::Rom;
use trace::trace;
use render::palette;
use render::frame::Frame;

use rand::Rng;

use sdl2::event::Event;
use sdl2::EventPump;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::pixels::PixelFormatEnum;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate bitflags;

/*
    User input for the game Snake.
    The input is always stored in the memory address 0xFF.
    The number stored is the ASCII value of the corresponding key.
*/
fn handle_user_input(cpu: &mut CPU, event_pump: &mut EventPump) {
    for event in event_pump.poll_iter() {
        match event {
            Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                std::process::exit(0);
            },
            Event::KeyDown { keycode: Some(Keycode::W), .. } => {
                cpu.mem_write(0xFF, 0x77);
            },
            Event::KeyDown { keycode: Some(Keycode::S), .. } => {
                cpu.mem_write(0xFF, 0x73);
            },
            Event::KeyDown { keycode: Some(Keycode::A), .. } => {
                cpu.mem_write(0xFF, 0x61);
            },
            Event::KeyDown { keycode: Some(Keycode::D), .. } => {
                cpu.mem_write(0xFF, 0x64);
            },
            _ => { }
        }
    }
}

// /*
//     Map colors from the game (1 byte per pixel) to SDL colors.
// */
// fn color(byte: u8) -> Color {
//     match byte {
//         0 => sdl2::pixels::Color::BLACK,
//         1 => sdl2::pixels::Color::WHITE,
//         2 | 9 => sdl2::pixels::Color::GREY,
//         3 | 10 => sdl2::pixels::Color::RED,
//         4 | 11 => sdl2::pixels::Color::GREEN,
//         5 | 12 => sdl2::pixels::Color::BLUE,
//         6 | 13 => sdl2::pixels::Color::MAGENTA,
//         7 | 14 => sdl2::pixels::Color::YELLOW,
//         _ => sdl2::pixels::Color::CYAN
//     }
// }

// /*
//     Read the state of the frame, and return true if it has changed and needs to
//     be updated in the scren.
//     The screen is composed of 32x32 pixels, with 3 color bytes each.
// */
// fn read_screen_state(cpu: &mut CPU, frame: &mut [u8; 32 * 3 * 32]) -> bool {
//     let mut frame_idx = 0;
//     let mut update = false;
//     // The state of the screen is in the memory range [0x0200, 0x0600]
//     for i in 0x0200..0x0600 {
//         let color_idx = cpu.mem_read(i as u16);
//         let (b1, b2, b3) = color(color_idx).rgb();
//         if frame[frame_idx] != b1 || frame[frame_idx + 1] != b2 || frame[frame_idx + 2] != b3 {
//             frame[frame_idx] = b1;
//             frame[frame_idx + 1] = b2;
//             frame[frame_idx + 2] = b3;
//             update = true;
//         }
//         frame_idx += 3;
//     }
//     update
// }


// fn main() {
//     // // Initialize SDL2
//     // let sdl_context = sdl2::init().unwrap();
//     // let video_subsystem = sdl_context.video().unwrap();
//     // let window = video_subsystem
//     //     .window("Snake game", (32. * 10.) as u32, (32. * 10.) as u32)
//     //     .position_centered()
//     //     .build().unwrap();
//     //
//     // let mut canvas = window.into_canvas().present_vsync().build().unwrap();
//     // let mut event_pump = sdl_context.event_pump().unwrap();
//     // canvas.set_scale(10., 10.).unwrap();
//     //
//     // // Create a texture that will be used for rendering
//     // let texture_creator = canvas.texture_creator();
//     // let mut texture = texture_creator.create_texture_target(PixelFormatEnum::RGB24, 32, 32).unwrap();
//     //
//     // // Load the game from the dump rom
//     // let rom_bytes: Vec<u8> = std::fs::read("snake.nes").unwrap();
//     // let rom = Rom::new(&rom_bytes).unwrap();
//     //
//     // let bus = Bus::new(rom);
//     // let mut cpu = CPU::new(bus);
//     // cpu.reset();
//     //
//     // // Initialize the screen
//     // let mut screen_state = [0 as u8; 32 * 3 * 32];
//     // let mut rng = rand::thread_rng();
//     //
//     // cpu.run_with_callback(move |cpu| {
//     //     // The callback function that will be called before running each instruction
//     //     handle_user_input(cpu, &mut event_pump);
//     //     // Update mem[0xFE] with new Random Number
//     //     cpu.mem_write(0xfe, rng.gen_range(1, 16));
//     //
//     //     // Redraw the scene if it changed
//     //     if read_screen_state(cpu, &mut screen_state) {
//     //         texture.update(None, &screen_state, 32 * 3).unwrap();
//     //         canvas.copy(&texture, None, None).unwrap();
//     //         canvas.present();
//     //     }
//     //
//     //     // Wait 70000 nanoseconds (or 70 microseconds)
//     //     std::thread::sleep(std::time::Duration::new(0, 70000));
//     // });
//
//     // ----------------------------------------------------------------------------------------
//
//     // Run the test rom and print the trace
//     let rom_bytes: Vec<u8> = std::fs::read("../roms/nestest.nes").unwrap();
//     let rom = Rom::new(&rom_bytes).unwrap();
//
//     let bus = Bus::new(rom);
//     let mut cpu = CPU::new(bus);
//     cpu.reset();
//     cpu.program_counter = 0xC000;
//
//     cpu.run_with_callback(move |cpu| {
//         println!("{}", trace(cpu));
//     });
//
// }


// ----------------------------------------------------------------------------------------

fn show_tile(chr_rom: &Vec<u8>, bank: usize, tile_n: usize) -> Frame {
    // There is space for up to 512 tiles in the rom, divided in two banks (left and right).
    assert!(bank <= 1);

    let mut frame = Frame::new();
    let bank_offset = (bank * 0x1000) as usize;

    let tile = &chr_rom[(bank_offset + tile_n * 16)..=(bank_offset + tile_n * 16 + 15)];

    for y in 0..=7 {
        let mut upper = tile[y];
        let mut lower = tile[y + 8];

        for x in (0..=7).rev() {
            let value = (1 & upper) << 1 | (1 & lower);
            upper = upper >> 1;
            lower = lower >> 1;
            let rgb = match value {
                0 => palette::SYSTEM_PALETTE[0x01],
                1 => palette::SYSTEM_PALETTE[0x23],
                2 => palette::SYSTEM_PALETTE[0x27],
                3 => palette::SYSTEM_PALETTE[0x30],
                _ => panic!("Invalid color index.")
            };
            frame.set_pixel(x, y, rgb);
        }
    }

    frame
}

fn show_tile_bank(chr_rom: &Vec<u8>, bank: usize) -> Frame {
    // There is space for up to 512 tiles in the rom, divided in two banks (left and right).
    assert!(bank <= 1);

    let mut frame = Frame::new();
    let bank_offset = (bank * 0x1000) as usize;
    let mut tile_x = 0;
    let mut tile_y = 0;

    for tile_n in 0..255 {
        if tile_n != 0 && tile_n % 20 == 0 {
            tile_y += 10;
            tile_x = 0;
        }

        let tile = &chr_rom[(bank_offset + tile_n * 16)..=(bank_offset + tile_n * 16 + 15)];

        for y in 0..=7 {
            let mut upper = tile[y];
            let mut lower = tile[y + 8];

            for x in (0..=7).rev() {
                let value = (1 & upper) << 1 | (1 & lower);
                upper = upper >> 1;
                lower = lower >> 1;
                let rgb = match value {
                    0 => palette::SYSTEM_PALETTE[0x01],
                    1 => palette::SYSTEM_PALETTE[0x23],
                    2 => palette::SYSTEM_PALETTE[0x27],
                    3 => palette::SYSTEM_PALETTE[0x30],
                    _ => panic!("Invalid color index.")
                };
                frame.set_pixel(tile_x + x, tile_y + y, rgb);
            }
        }

        tile_x += 10;
    }

    frame
}

fn main() {
    // Initialize SDL2
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("Tile viewer", (256. * 3.) as u32, (240. * 3.) as u32)
        .position_centered()
        .build().unwrap();

    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();
    canvas.set_scale(3., 3.).unwrap();

    // Create a texture that will be used for rendering
    let texture_creator = canvas.texture_creator();
    let mut texture = texture_creator.create_texture_target(PixelFormatEnum::RGB24, 256, 240).unwrap();

    // Load the game from the dump rom
    // let rom_bytes: Vec<u8> = std::fs::read("test_roms/Alter_Ego.nes").unwrap();
    let rom_bytes: Vec<u8> = std::fs::read("test_roms/Pac-Man.nes").unwrap();
    let rom = Rom::new(&rom_bytes).unwrap();

    // // Show a single tile
    // let tile_frame = show_tile(&rom.chr_rom, 1, 0);
    // texture.update(None, &tile_frame.data, 256 * 3).unwrap();

    // Show an entire tile bank
    let right_bank = show_tile_bank(&rom.chr_rom, 1);
    texture.update(None, &right_bank.data, 256 * 3).unwrap();

    canvas.copy(&texture, None, None);
    canvas.present();

    loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    std::process::exit(0)
                },
                _ => {}
            }
        }
    }
}
