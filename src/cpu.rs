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
    pub status: u8
}

impl CPU {
    pub fn new() -> Self {
        CPU {
            program_counter: 0,
            register_a: 0,
            register_x: 0,
            register_y: 0,
            status: 0
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
    fn lda(&mut self, value: u8) {
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
                    // This takes an argument as value, which is the next element in the program.
                    let param = program[self.program_counter as usize];
                    self.program_counter += 1;
                    self.lda(param);
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
}
