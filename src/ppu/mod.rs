use crate::cartridge::Mirroring;
use registers::addr::AddrRegister;
use registers::control::ControlRegister;

pub mod registers;

pub struct NesPPU {
    pub chr_rom: Vec<u8>,
    pub palette_table: [u8; 32],
    pub vram: [u8; 2048],

    pub mirroring: Mirroring,

    // Registers
    /*
        0x2000: Controller
        0x2001: Mask
        0x2002: Status
        0x2003: OAM Address
        0x2004: OAM Data
        0x2005: Scroll
        0x2006: Address
        0x2007: Data
        0x2014: OAM DMA
    */
    pub ctrl: ControlRegister,
    pub addr: AddrRegister,
    pub oam_data: [u8; 256],

    internal_data_buf: u8
}

impl NesPPU {
    pub fn new(chr_rom: Vec<u8>, mirroring: Mirroring) -> Self {
        NesPPU {
            chr_rom: chr_rom,
            palette_table: [0; 32],
            vram: [0; 2048],

            mirroring: mirroring,

            ctrl: ControlRegister::new(),
            addr: AddrRegister::new(),
            oam_data: [0; 64 * 4],

            internal_data_buf: 0
        }
    }

    /*
        Horizontal mirroring:
            [A] [a]
            [B] [b]

        Vertical mirroring:
            [A] [B]
            [a] [b]
    */
    pub fn mirror_vram_addr(&self, address: u16) -> u16 {
        // Mirror down [0x3000, 0x3EFF] to [0x2000, 0x2EFF]
        let mirrored_vram = address & 0b1011111111111111;
        // Index in the VRAM vector
        let vram_index = mirrored_vram - 0x2000;
        // Index of the name table
        let name_table = vram_index / 0x400;

        match (&self.mirroring, name_table) {
            (Mirroring::Vertical, 2) | (Mirroring::Vertical, 3) => vram_index - 0x800,
            (Mirroring::Horizontal, 2) | (Mirroring::Horizontal, 1) => vram_index - 0x400,
            (Mirroring::Horizontal, 3) => vram_index - 0x800,
            _ => vram_index
        }
    }

    fn increment_vram_addr(&mut self) {
        self.addr.increment(self.ctrl.vram_addr_increment());
    }
}

pub trait PPU {
    // TODO
    fn write_to_ctrl(&mut self, value: u8);
    fn write_to_ppu_addr(&mut self, value: u8);
    fn read_data(&mut self) -> u8;
    // TODO
}

impl PPU for NesPPU {
    fn write_to_ctrl(&mut self, value: u8) {
        self.ctrl.update(value);
    }

    fn write_to_ppu_addr(&mut self, value: u8) {
        self.addr.update(value);
    }

    fn read_data(&mut self) -> u8 {
        let address = self.addr.get();
        self.increment_vram_addr();

        match address {
            0..=0x1FFF => {
                // Access CHR_ROM
                let result = self.internal_data_buf;
                self.internal_data_buf = self.chr_rom[address as usize];
                result
            },
            0x2000..=0x2FFF => {
                // Access VRAM
                let result = self.internal_data_buf;
                self.internal_data_buf = self.vram[self.mirror_vram_addr(address) as usize];
                result
            },
            // Addresses $3F10/$3F14/$3F18/$3F1C are mirrors of $3F00/$3F04/$3F08/$3F0C
            0x3f10 | 0x3f14 | 0x3f18 | 0x3f1c => {
                let add_mirror = addr - 0x10;
                self.palette_table[(add_mirror - 0x3f00) as usize]
            }
            0x3000..=0x3EFF => { 
                unimplemented!("NesPPU: Address space [0x3000, 0x3EFF] is not expected to be used, requested = {:04x}", address)
            },
            0x3F00..=0x3FFF => {
                self.palette_table[(address - 0x3F00) as usize]
            },
            _ => panic!("NesPPU: Unexpected access to mirrored space, requested = {:04x}", address)
        }
    }
}






#[cfg(test)]
pub mod test {
    use super::*;

    // #[test]
    // fn test_ppu_vram_writes() {
    //     let mut ppu = NesPPU::new_empty_rom();
    //     ppu.write_to_ppu_addr(0x23);
    //     ppu.write_to_ppu_addr(0x05);
    //     ppu.write_to_data(0x66);
    //
    //     assert_eq!(ppu.vram[0x0305], 0x66);
    // }

    // #[test]
    // fn test_ppu_vram_reads() {
    //     let mut ppu = NesPPU::new_empty_rom();
    //     ppu.write_to_ctrl(0);
    //     ppu.vram[0x0305] = 0x66;
    //
    //     ppu.write_to_ppu_addr(0x23);
    //     ppu.write_to_ppu_addr(0x05);
    //
    //     ppu.read_data(); //load_into_buffer
    //     assert_eq!(ppu.addr.get(), 0x2306);
    //     assert_eq!(ppu.read_data(), 0x66);
    // }
    //
    // #[test]
    // fn test_ppu_vram_reads_cross_page() {
    //     let mut ppu = NesPPU::new_empty_rom();
    //     ppu.write_to_ctrl(0);
    //     ppu.vram[0x01ff] = 0x66;
    //     ppu.vram[0x0200] = 0x77;
    //
    //     ppu.write_to_ppu_addr(0x21);
    //     ppu.write_to_ppu_addr(0xff);
    //
    //     ppu.read_data(); //load_into_buffer
    //     assert_eq!(ppu.read_data(), 0x66);
    //     assert_eq!(ppu.read_data(), 0x77);
    // }
    //
    // #[test]
    // fn test_ppu_vram_reads_step_32() {
    //     let mut ppu = NesPPU::new_empty_rom();
    //     ppu.write_to_ctrl(0b100);
    //     ppu.vram[0x01ff] = 0x66;
    //     ppu.vram[0x01ff + 32] = 0x77;
    //     ppu.vram[0x01ff + 64] = 0x88;
    //
    //     ppu.write_to_ppu_addr(0x21);
    //     ppu.write_to_ppu_addr(0xff);
    //
    //     ppu.read_data(); //load_into_buffer
    //     assert_eq!(ppu.read_data(), 0x66);
    //     assert_eq!(ppu.read_data(), 0x77);
    //     assert_eq!(ppu.read_data(), 0x88);
    // }
    //
    // // Horizontal: https://wiki.nesdev.com/w/index.php/Mirroring
    // //   [0x2000 A ] [0x2400 a ]
    // //   [0x2800 B ] [0x2C00 b ]
    // #[test]
    // fn test_vram_horizontal_mirror() {
    //     let mut ppu = NesPPU::new_empty_rom();
    //     ppu.write_to_ppu_addr(0x24);
    //     ppu.write_to_ppu_addr(0x05);
    //
    //     ppu.write_to_data(0x66); //write to a
    //
    //     ppu.write_to_ppu_addr(0x28);
    //     ppu.write_to_ppu_addr(0x05);
    //
    //     ppu.write_to_data(0x77); //write to B
    //
    //     ppu.write_to_ppu_addr(0x20);
    //     ppu.write_to_ppu_addr(0x05);
    //
    //     ppu.read_data(); //load into buffer
    //     assert_eq!(ppu.read_data(), 0x66); //read from A
    //
    //     ppu.write_to_ppu_addr(0x2C);
    //     ppu.write_to_ppu_addr(0x05);
    //
    //     ppu.read_data(); //load into buffer
    //     assert_eq!(ppu.read_data(), 0x77); //read from b
    // }
    //
    // // Vertical: https://wiki.nesdev.com/w/index.php/Mirroring
    // //   [0x2000 A ] [0x2400 B ]
    // //   [0x2800 a ] [0x2C00 b ]
    // #[test]
    // fn test_vram_vertical_mirror() {
    //     let mut ppu = NesPPU::new(vec![0; 2048], Mirroring::Vertical);
    //
    //     ppu.write_to_ppu_addr(0x20);
    //     ppu.write_to_ppu_addr(0x05);
    //
    //     ppu.write_to_data(0x66); //write to A
    //
    //     ppu.write_to_ppu_addr(0x2C);
    //     ppu.write_to_ppu_addr(0x05);
    //
    //     ppu.write_to_data(0x77); //write to b
    //
    //     ppu.write_to_ppu_addr(0x28);
    //     ppu.write_to_ppu_addr(0x05);
    //
    //     ppu.read_data(); //load into buffer
    //     assert_eq!(ppu.read_data(), 0x66); //read from a
    //
    //     ppu.write_to_ppu_addr(0x24);
    //     ppu.write_to_ppu_addr(0x05);
    //
    //     ppu.read_data(); //load into buffer
    //     assert_eq!(ppu.read_data(), 0x77); //read from B
    // }
    //
    // #[test]
    // fn test_read_status_resets_latch() {
    //     let mut ppu = NesPPU::new_empty_rom();
    //     ppu.vram[0x0305] = 0x66;
    //
    //     ppu.write_to_ppu_addr(0x21);
    //     ppu.write_to_ppu_addr(0x23);
    //     ppu.write_to_ppu_addr(0x05);
    //
    //     ppu.read_data(); //load_into_buffer
    //     assert_ne!(ppu.read_data(), 0x66);
    //
    //     ppu.read_status();
    //
    //     ppu.write_to_ppu_addr(0x23);
    //     ppu.write_to_ppu_addr(0x05);
    //
    //     ppu.read_data(); //load_into_buffer
    //     assert_eq!(ppu.read_data(), 0x66);
    // }
    //
    // #[test]
    // fn test_ppu_vram_mirroring() {
    //     let mut ppu = NesPPU::new_empty_rom();
    //     ppu.write_to_ctrl(0);
    //     ppu.vram[0x0305] = 0x66;
    //
    //     ppu.write_to_ppu_addr(0x63); //0x6305 -> 0x2305
    //     ppu.write_to_ppu_addr(0x05);
    //
    //     ppu.read_data(); //load into_buffer
    //     assert_eq!(ppu.read_data(), 0x66);
    //     // assert_eq!(ppu.addr.read(), 0x0306)
    // }
    //
    // #[test]
    // fn test_read_status_resets_vblank() {
    //     let mut ppu = NesPPU::new_empty_rom();
    //     ppu.status.set_vblank_status(true);
    //
    //     let status = ppu.read_status();
    //
    //     assert_eq!(status >> 7, 1);
    //     assert_eq!(ppu.status.snapshot() >> 7, 0);
    // }
    //
    // #[test]
    // fn test_oam_read_write() {
    //     let mut ppu = NesPPU::new_empty_rom();
    //     ppu.write_to_oam_addr(0x10);
    //     ppu.write_to_oam_data(0x66);
    //     ppu.write_to_oam_data(0x77);
    //
    //     ppu.write_to_oam_addr(0x10);
    //     assert_eq!(ppu.read_oam_data(), 0x66);
    //
    //     ppu.write_to_oam_addr(0x11);
    //     assert_eq!(ppu.read_oam_data(), 0x77);
    // }
    //
    // #[test]
    // fn test_oam_dma() {
    //     let mut ppu = NesPPU::new_empty_rom();
    //
    //     let mut data = [0x66; 256];
    //     data[0] = 0x77;
    //     data[255] = 0x88;
    //
    //     ppu.write_to_oam_addr(0x10);
    //     ppu.write_oam_dma(&data);
    //
    //     ppu.write_to_oam_addr(0xf); //wrap around
    //     assert_eq!(ppu.read_oam_data(), 0x88);
    //
    //     ppu.write_to_oam_addr(0x10);
    //     assert_eq!(ppu.read_oam_data(), 0x77);
    //
    //     ppu.write_to_oam_addr(0x11);
    //     assert_eq!(ppu.read_oam_data(), 0x66);
    // }
}

