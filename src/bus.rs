use crate::cpu::Mem;

//  _______________ $10000  _______________
// | PRG-ROM       |       |               |
// | Upper Bank    |       |               |
// |_ _ _ _ _ _ _ _| $C000 | PRG-ROM       |
// | PRG-ROM       |       |               |
// | Lower Bank    |       |               |
// |_______________| $8000 |_______________|
// | SRAM          |       | SRAM          |
// |_______________| $6000 |_______________|
// | Expansion ROM |       | Expansion ROM |
// |_______________| $4020 |_______________|
// | I/O Registers |       |               |
// |_ _ _ _ _ _ _ _| $4000 |               |
// | Mirrors       |       | I/O Registers |
// | $2000-$2007   |       |               |
// |_ _ _ _ _ _ _ _| $2008 |               |
// | I/O Registers |       |               |
// |_______________| $2000 |_______________|
// | Mirrors       |       |               | These addresses are mapped to the range [0000-07FF], three times.
// | $0000-$07FF   |       |               | This is due to the fact the actual RAM of the system is not enough to 
// |_ _ _ _ _ _ _ _| $0800 |               | fill the whole range of 16-bit addresses.
// | RAM           |       | RAM           |
// |_ _ _ _ _ _ _ _| $0200 |               |
// | Stack         |       |               |
// |_ _ _ _ _ _ _ _| $0100 |               |
// | Zero Page     |       |               |
// |_______________| $0000 |_______________|

const RAM: u16 = 0x0000;
const RAM_MIRRORS_END: u16 = 0x1FFF;
const PPU_REGISTERS: u16 = 0x2000;
const PPU_REGISTERS_MIRRORS_END: u16 = 0x3FFF;



pub struct Bus {
    cpu_vram: [u8; 2048]
}

impl Bus {
    pub fn new() -> Self {
        Bus {
            cpu_vram: [0; 2048]
        }
    }
}

impl Mem for Bus {
    fn mem_read(&self, address: u16) -> u8 {
        match address {
            RAM ..= RAM_MIRRORS_END => {
                let mirror_down_addr = address & 0b00000111_11111111;
                self.cpu_vram[mirror_down_addr as usize]
            }
            PPU_REGISTERS ..= PPU_REGISTERS_MIRRORS_END => {
                let _mirror_down_addr = address & 0b00100000_00000111;
                todo!("PPU is not supported yet")
            }
            _ => {
                println!("Ignoring memory read access at {}", address);
                0
            }
        }
    }

    fn mem_write(&mut self, address: u16, data: u8) {
        match address {
            RAM ..= RAM_MIRRORS_END => {
                let mirror_down_addr = address & 0b00000111_11111111;
                self.cpu_vram[mirror_down_addr as usize] = data;
            }
            PPU_REGISTERS ..= PPU_REGISTERS_MIRRORS_END => {
                let _mirror_down_addr = address & 0b00100000_00000111;
                todo!("PPU is not supported yet")
            }
            _ => {
                println!("Ignoring memory write access at {}", address);
            }
        }
    }
}
