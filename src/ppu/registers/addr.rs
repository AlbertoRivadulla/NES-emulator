/*
    The address register is one byte used to provide access to the PPU memory map to the CPU.
*/
pub struct AddrRegister {
    // Value in the register, composed of two bytes. The total value is expressed in big endian
    value: (u8, u8),
    hi_ptr: bool
}

impl AddrRegister {
    pub fn new() -> Self {
        AddrRegister {
            // High byte first, low byte second
            value: (0, 0),
            hi_ptr: true
        }
    }

    fn set(&mut self, data: u16) {
        self.value.0 = (data >> 8) as u8;
        self.value.1 = (data & 0xFF) as u8;
    }

    pub fn update(&mut self, data: u8) {
        if self.hi_ptr {
            self.value.0 = data;
        } else {
            self.value.1 = data;
        }

        // Addresses after 0x4000 should mirror those in [0x0000, 0x3FFF]
        if self.get() > 0x3FFF {
            self.set(self.get() & 0b0011111111111111); // This binary nr is equal to 0x3FFF
        }

        self.hi_ptr = !self.hi_ptr;
    }

    pub fn increment(&mut self, inc: u8) {
        let lo_before = self.value.0;
        self.value.1 = self.value.1.wrapping_add(inc);
        if lo_before > self.value.1 {
            self.value.0 = self.value.0.wrapping_add(1);
        }

        if self.get() > 0x3FFF {
            self.set(self.get() & 0b0011111111111111);
        }
    }

    fn reset_latch(&mut self) {
        self.hi_ptr = true;
    }

    pub fn get(&mut self) -> u16 {
        ((self.value.0 as u16) << 8) | (self.value.1 as u16)
    }
}

