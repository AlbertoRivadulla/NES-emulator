/*
    The mask register instructs the PPU how to render sprites and background.
    Its 8 bits have the following meaning:
        7  bit  0
        ---- ----
        BGRs bMmG
        |||| ||||
        |||| |||+- Greyscale (0: normal color, 1: produce a greyscale display)
        |||| ||+-- 1: Show background in leftmost 8 pixels of screen, 0: Hide
        |||| |+--- 1: Show sprites in leftmost 8 pixels of screen, 0: Hide
        |||| +---- 1: Show background
        |||+------ 1: Show sprites
        ||+------- Emphasize red
        |+-------- Emphasize green
        +--------- Emphasize blue
*/

bitflags!{
    pub struct MaskRegister : u8 {
        const GRAYSCALE = 0b00000001;
        const LEFTMOST_8PXL_BACKGROUND = 0b00000010;
        const LEFTMOST_8PXL_SPRITE = 0b00000100;
        const SHOW_BACKGROUND = 0b00001000;
        const SHOW_SPRITES = 0b00010000;
        const EMPHASIZE_RED = 0b00100000;
        const EMPHASIZE_GREEN = 0b01000000;
        const EMPHASIZE_BLUE = 0b10000000;
    }
}

pub enum Color {
    Red,
    Green,
    Blue
}

impl MaskRegister {
    pub fn new() -> Self {
        MaskRegister::from_bits_truncate(0b00000000)
    }

    pub fn is_grayscale(&self) -> bool {
        self.contains(MaskRegister::GRAYSCALE)
    }

    pub fn leftmost_8xpl_background(&self) -> bool {
        self.contains(MaskRegister::LEFTMOST_8PXL_BACKGROUND)
    }

    pub fn leftmost_8xpl_sprite(&self) -> bool {
        self.contains(MaskRegister::LEFTMOST_8PXL_SPRITE)
    }

    pub fn show_background(&self) -> bool {
        self.contains(MaskRegister::SHOW_BACKGROUND)
    }

    pub fn show_sprites(&self) -> bool {
        self.contains(MaskRegister::SHOW_SPRITES)
    }

    pub fn emphasize(&mut self) -> Vec<Color> {
        let mut result = Vec::<Color>::new();
        if self.contains(MaskRegister::EMPHASIZE_RED) {
            result.push(Color::Red);
        }
        if self.contains(MaskRegister::EMPHASIZE_GREEN) {
            result.push(Color::Green);
        }
        if self.contains(MaskRegister::EMPHASIZE_BLUE) {
            result.push(Color::Blue);
        }

        result
    }

    pub fn update(&mut self, data: u8) {
        self.bits = data;
    }
}

