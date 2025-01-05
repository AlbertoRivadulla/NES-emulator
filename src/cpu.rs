use crate::opcodes;
/*
    Flags in the CPU status:

        |N|V|_|B|D|I|Z|C|
        
        N -> Negative flag
        V -> Overflow flag
        B -> Break command
        D -> Decimal mode flag
        I -> Interrupt disable
        Z -> Zero flag
        C -> Carry flag
*/
pub struct CPU {
    pub program_counter: u16,
    pub register_a: u8, // accumulator
    pub register_x: u8,
    pub register_y: u8,
    pub status: u8,
    memory: [u8; 0xFFFF]
}

#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum AddressingMode {
   Immediate,
   ZeroPage,
   ZeroPage_X,
   ZeroPage_Y,
   Absolute,
   Absolute_X,
   Absolute_Y,
   Indirect_X,
   Indirect_Y,
   NoneAddressing,
}


trait Mem {
    fn mem_read(&self, address: u16) -> u8;

    fn mem_write(&mut self, address: u16, data: u8);

    fn mem_read_u16(&self, address: u16) -> u16 {
        // Read a 2-byte value, stored in little-endian.
        let lo = self.mem_read(address) as u16;
        let hi = self.mem_read(address + 1) as u16;
        (hi << 8) | lo
    }

    fn mem_write_u16(&mut self, address: u16, data: u16) {
        let hi = (data >> 8) as u8;
        let lo = (data & 0x00ff) as u8;
        self.mem_write(address, lo);
        self.mem_write(address + 1, hi);
    }
}

impl Mem for CPU {
    fn mem_read(&self, address: u16) -> u8 {
        self.memory[address as usize]
    }

    fn mem_write(&mut self, address: u16, data: u8) {
        self.memory[address as usize] = data;
    }
}

impl CPU {
    pub fn new() -> Self {
        CPU {
            program_counter: 0,
            register_a: 0,
            register_x: 0,
            register_y: 0,
            status: 0,
            memory: [0; 0xFFFF]
        }
    }

    /*
        Get the address of the next operand, depending on the addressing mode
    */
    fn get_operand_address(&self, mode: &AddressingMode) -> u16 {
        match mode {
           AddressingMode::Immediate => self.program_counter,
           AddressingMode::ZeroPage => self.mem_read(self.program_counter) as u16,
           AddressingMode::Absolute => self.mem_read_u16(self.program_counter),
           AddressingMode::ZeroPage_X => {
               let pos = self.mem_read(self.program_counter);
               let address = pos.wrapping_add(self.register_x) as u16;
               address
           },
           AddressingMode::ZeroPage_Y => {
               let pos = self.mem_read(self.program_counter);
               let address = pos.wrapping_add(self.register_x) as u16;
               address
           },
           AddressingMode::Absolute_X => {
               let base = self.mem_read_u16(self.program_counter);
               let address = base.wrapping_add(self.register_x as u16);
               address
           },
           AddressingMode::Absolute_Y => {
               let base = self.mem_read_u16(self.program_counter);
               let address = base.wrapping_add(self.register_y as u16);
               address
           },
           AddressingMode::Indirect_X => {
               let base = self.mem_read(self.program_counter);

               let ptr: u8 = (base as u8).wrapping_add(self.register_x);
               let lo = self.mem_read(ptr as u16);
               let hi = self.mem_read(ptr.wrapping_add(1) as u16);
               (hi as u16) << 8 | (lo as u16)
           },
           AddressingMode::Indirect_Y => {
               let base = self.mem_read(self.program_counter);

               let lo = self.mem_read(base as u16);
               let hi = self.mem_read((base as u8).wrapping_add(1) as u16);
               let deref_base = (hi as u16) << 8 | (lo as u16);
               let deref = deref_base.wrapping_add(self.register_y as u16);
               deref
           },
           AddressingMode::NoneAddressing => {
               panic!("Addressing mode {:?} is not supported.", mode)
           }
        }
    }
    
    pub fn load_and_run(&mut self, program: Vec<u8>) {
        self.load(program);
        self.reset();
        self.run();
    }

    fn load(&mut self, program: Vec<u8>) {
        // The memory addresses [ 0x8000 .. 0xFFFF ] correspond to Program ROM
        self.memory[0x8000 .. (0x8000 + program.len())]
            .copy_from_slice(&program[..]);
        // Store the location of the first opcode in the address 0xFFFC, which is the first read by the NES CPU.
        self.mem_write_u16(0xFFFC, 0x8000);
    }

    fn reset(&mut self) {
        self.register_a = 0;
        self.register_x = 0;
        self.register_y = 0;
        self.status = 0;

        self.program_counter = self.mem_read_u16(0xFFFC);
    }

    fn run(&mut self) {
        loop {
            let opscode: u8 = self.mem_read(self.program_counter);
            self.program_counter += 1;

            match opscode {
                // LDA - Load accumulator
                0xA9 => {
                    self.lda(&AddressingMode::Immediate);
                    self.program_counter += 1;
                }
                0xA5 => {
                    self.lda(&AddressingMode::ZeroPage);
                    self.program_counter += 1;
                }
                // 0xB5 => {
                //
                // }
                0xAD => {
                    self.lda(&AddressingMode::Absolute);
                    self.program_counter += 1;
                }
                // TAX - Transfer Accumulator to X
                0xAA => self.tax(),
                // INX - Increment X Register
                0xE8 => self.inx(),
                // BRK - Break
                0x00 => return,
                _ => todo!()
            }
        }
    }

    fn update_zero_and_negative_flags(&mut self, result: u8) {
        // Set the CPU flag corresponding to value equal to zero
        if result == 0 {
            self.status = self.status | 0b0000_0010;
        } else {
            self.status = self.status & 0b1111_1101;
        }

        // Set the CPU flag corresponding to negative value
        if (result & 0b1000_0000) != 0 {
            self.status = self.status | 0b1000_0000;
        } else {
            self.status = self.status & 0b0111_1111;
        }
    }

    // LDA - Load accumulator
    fn lda(&mut self, mode: &AddressingMode) {
        let address = self.get_operand_address(&mode);
        let value = self.mem_read(address);

        self.register_a = value;
        self.update_zero_and_negative_flags(self.register_a);
    }

    // TAX - Transfer Accumulator to X
    fn tax(&mut self) {
        self.register_x = self.register_a;
        self.update_zero_and_negative_flags(self.register_x);
    }
    // INX - Increment X Register
    fn inx(&mut self) {
        // self.register_x += 1;
        // Add 1 and wrap if there is overflow.
        self.register_x = self.register_x.wrapping_add(1);
        self.update_zero_and_negative_flags(self.register_x);
    }

    /*
        Interprets an entire program, given as a vector of instructions.
    */
    pub fn interpret(&mut self, program: Vec<u8>) {
        self.program_counter = 0;

        loop {
            let opscode: u8 = program[self.program_counter as usize];
            self.program_counter += 1;

            match opscode {
                // LDA - Load accumulator
                0xA9 => {
                    self.lda(&AddressingMode::Immediate);
                    self.program_counter += 1;
                }
                0xA5 => {
                    self.lda(&AddressingMode::ZeroPage);
                    self.program_counter += 1;
                }
                // 0xB5 => {
                //
                // }
                0xAD => {
                    self.lda(&AddressingMode::Absolute);
                    self.program_counter += 1;
                }
                // TAX - Transfer Accumulator to X
                0xAA => self.tax(),
                // INX - Increment X Register
                0xE8 => self.inx(),
                // BRK - Break
                0x00 => return,
                _ => todo!()
            }
        }
    }
}


#[cfg(test)]
mod test {
    use super::*;

    // Tests from section 3.1

    #[test]
    fn test_0xa9_lda_immediate_load_data() {
        let mut cpu = CPU::new();
        cpu.interpret(vec![0xA9, 0x05, 0x00]);

        assert_eq!(cpu.register_a, 0x05);
        assert!(cpu.status & 0b0000_0010 == 0x00);
        assert!(cpu.status & 0b1000_0000 == 0x00);
    }

    #[test]
    fn test_0xa9_lda_zero_flag() {
        let mut cpu = CPU::new();
        cpu.interpret(vec![0xA9, 0x00, 0x00]);

        assert!(cpu.status & 0b0000_0010 == 0b10);
    }

    #[test]
    fn test_0xxx_tax_move_a_to_x() {
        let mut cpu = CPU::new();
        cpu.register_a = 10;
        cpu.interpret(vec![0xAA, 0x00]);

        assert_eq!(cpu.register_x, 10);
    }

    #[test]
    fn test_0xe8_inx_overflow() {
        let mut cpu = CPU::new();
        cpu.register_x = 0xFF;
        cpu.interpret(vec![0xE8, 0xE8, 0x00]);

        assert_eq!(cpu.register_x, 1);
    }

    #[test]
    fn test_5_ops_together() {
        let mut cpu = CPU::new();
        cpu.interpret(vec![0xA9, 0xC0, 0xAA, 0xE8, 0x00]);

        assert_eq!(cpu.register_x, 0xC1);
    }

    // Tests from section 3.2

    #[test]
    fn test_lda_from_memory() {
        let mut cpu = CPU::new();
        cpu.mem_write(0x10, 0x55);
        cpu.load_and_run(vec![0xa5, 0x10, 0x00]);

        assert_eq!(cpu.register_a, 0x55);
    }
}
