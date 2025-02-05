use crate::cartridge::Rom;
use crate::cpu::Mem;
use crate::ppu::NesPPU;
use crate::ppu::PPU;

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
    cpu_vram: [u8; 2048],
    prg_rom: Vec<u8>,
    ppu: NesPPU,

    cycles: usize
}

impl Bus {
    pub fn new(rom: Rom) -> Self {
        let ppu = NesPPU::new(rom.chr_rom, rom.screen_mirroring);

        Bus {
            cpu_vram: [0; 2048],
            prg_rom: rom.prg_rom,
            ppu: ppu,
            cycles: 0
        }
    }

    /*
        Read the space [0x8000, 0x10000], which corresponds to the ROM.
        This maps a region of 32 KiB, but some roms only use 16 KiB.
    */
    fn read_prg_rom(&self, mut addr: u16) -> u8 {
        addr -= 0x8000;
        if self.prg_rom.len() == 0x4000 && addr >= 0x4000 {
            addr = addr % 0x4000;
        }
        self.prg_rom[addr as usize]
    }

    /*
        This is called after running an instruction in the CPU, passing the number of cycles that the instruction took.
        The number cycles passed to the PPU is multiplied by 3, since its clock speed is three times that of the CPU.
    */
    pub fn tick(&mut self, cycles: u8) {
        self.cycles += cycles as usize;
        self.ppu.tick(cycles * 3);
    }

    pub fn poll_nmi_status(&mut self) -> Option<u8> {
        self.ppu.nmi_interrupt.take()
    }
}

impl Mem for Bus {
    fn mem_read(&mut self, address: u16) -> u8 {
        match address {
            RAM ..= RAM_MIRRORS_END => {
                let mirror_down_addr = address & 0b00000111_11111111;
                self.cpu_vram[mirror_down_addr as usize]
            }
            0x2000 | 0x2001 | 0x2003 | 0x2005 | 0x2006 | 0x4014 => {
                panic!("Bus: Attempting to read from write-only PPU address {:04x}", address);
            },
            // Read PPU registers
            0x2002 => self.ppu.read_status(),
            0x2004 => self.ppu.read_oam_data(),
            0x2007 => self.ppu.read_data(),
            // Read PPU VRAM or Palettes
            0x2008 ..= PPU_REGISTERS_MIRRORS_END => {
                // Mirror down below 0x2008 and read the address
                let mirror_down_addr = address & 0b00100000_00000111;
                self.mem_read(mirror_down_addr)
            }
            0x8000..=0xFFFF => self.read_prg_rom(address),
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
            0x2000 => {
                self.ppu.write_to_ctrl(data);
            },
            0x2001 => {
                self.ppu.write_to_mask(data);
            },
            0x2002 => panic!("Bus: Attempt to write to PPU status register."),
            0x2003 => {
                self.ppu.write_to_oam_addr(data);
            },
            0x2004 => {
                self.ppu.write_to_oam_data(data);
            },
            0x2005 => {
                self.ppu.write_to_scroll(data);
            },
            0x2006 => {
                self.ppu.write_to_ppu_addr(data);
            },
            0x2007 => {
                self.ppu.write_to_data(data);
            }
            0x2008 ..= PPU_REGISTERS_MIRRORS_END => {
                let mirror_down_addr = address & 0b00100000_00000111;
                self.mem_write(mirror_down_addr, data);
            }
            0x8000..=0xFFFF => {
                panic!("Attempt to write on cartridge ROM space.")
            }
            _ => {
                println!("Ignoring memory write access at {}", address);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::cartridge::test;

    #[test]
    fn test_mem_read_write_to_ram() {
        let mut bus = Bus::new(test::test_rom());
        bus.mem_write(0x01, 0x55);
        assert_eq!(bus.mem_read(0x01), 0x55);
    }
}
