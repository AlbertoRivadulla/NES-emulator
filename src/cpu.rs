use crate::opcodes;
use std::collections::HashMap;
use crate::bus::Bus;

bitflags! {
    /*
        Flags in the CPU status:
             7 6 5 4 3 2 1 0
            |N|V|_|B|D|I|Z|C|
            
            N -> Negative flag
            V -> Overflow flag
            B -> Break command
            D -> Decimal mode flag (not used in NES)
            I -> Interrupt disable
            Z -> Zero flag
            C -> Carry flag
    */
    pub struct CpuFlags: u8 {
        const CARRY = 0b00000001;
        const ZERO = 0b00000010;
        const INTERRUPT_DISABLE = 0b00000100;
        const DECIMAL_MODE = 0b00001000;
        const BREAK = 0b00010000;
        const BREAK2 = 0b00100000;
        const OVERFLOW = 0b01000000;
        const NEGATIVE = 0b10000000;
    }
}

const STACK: u16 = 0x0100;
const STACK_RESET: u8 = 0xFD;

pub struct CPU {
    pub register_a: u8, // accumulator
    pub register_x: u8,
    pub register_y: u8,
    pub status: CpuFlags,
    pub program_counter: u16,
    pub stack_pointer: u8,
    pub bus: Bus
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


pub trait Mem {
    fn mem_read(&self, address: u16) -> u8;

    fn mem_write(&mut self, address: u16, data: u8);

    fn mem_read_u16(&self, address: u16) -> u16 {
        // Read a 2-byte value, stored in little-endian convention
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
        self.bus.mem_read(address)
    }

    fn mem_read_u16(&self, address: u16) -> u16 {
        self.bus.mem_read_u16(address)
    }

    fn mem_write(&mut self, address: u16, data: u8) {
        self.bus.mem_write(address, data);
    }

    fn mem_write_u16(&mut self, address: u16, data: u16) {
        self.bus.mem_write_u16(address, data);
    }
}

impl CPU {
    pub fn new(bus: Bus) -> Self {
        CPU {
            register_a: 0,
            register_x: 0,
            register_y: 0,
            status: CpuFlags::from_bits_truncate(0b100100),
            program_counter: 0x8000,
            stack_pointer: STACK_RESET,
            bus: bus
        }
    }

    pub fn get_absolute_address(&self, mode: &AddressingMode, address: u16) -> u16 {
        match mode {
            AddressingMode::Immediate => address,
            AddressingMode::ZeroPage => self.mem_read(address) as u16,
            AddressingMode::Absolute => self.mem_read_u16(address),

            AddressingMode::ZeroPage_X => {
                let pos = self.mem_read(address);
                let output_address = pos.wrapping_add(self.register_x) as u16;
                output_address
            },
            AddressingMode::ZeroPage_Y => {
                let pos = self.mem_read(address);
                let output_address = pos.wrapping_add(self.register_y) as u16;
                output_address
            },
            AddressingMode::Absolute_X => {
                let base = self.mem_read_u16(address);
                let output_address = base.wrapping_add(self.register_x as u16);
                output_address
            },
            AddressingMode::Absolute_Y => {
                let base = self.mem_read_u16(address);
                let output_address = base.wrapping_add(self.register_y as u16);
                output_address
            },
            AddressingMode::Indirect_X => {
                let base = self.mem_read(address);
 
                let ptr: u8 = (base as u8).wrapping_add(self.register_x);
                let lo = self.mem_read(ptr as u16);
                let hi = self.mem_read(ptr.wrapping_add(1) as u16);
                (hi as u16) << 8 | (lo as u16)
            },
            AddressingMode::Indirect_Y => {
                let base = self.mem_read(address);
 
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

    /*
        Get the address of the next operand, depending on the addressing mode
    */
    fn get_operand_address(&self, mode: &AddressingMode) -> u16 {
        self.get_absolute_address(mode, self.program_counter)
    }

    fn stack_pop(&mut self) -> u8 {
        self.stack_pointer = self.stack_pointer.wrapping_add(1);
        self.mem_read((STACK as u16) + self.stack_pointer as u16)
    }

    fn stack_pop_u16(&mut self) -> u16 {
        // Data is stored in little-endian convention
        let lo = self.stack_pop() as u16;
        let hi = self.stack_pop() as u16;
        hi << 8 | lo
    }

    fn stack_push(&mut self, data: u8) {
        self.mem_write((STACK as u16) + self.stack_pointer as u16, data);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
    }

    fn stack_push_u16(&mut self, data: u16) {
        let hi = (data >> 8) as u8;
        let lo = (data & 0x00FF) as u8;
        self.stack_push(hi);
        self.stack_push(lo);
    }

    fn set_register_a(&mut self, value: u8) {
        self.register_a = value;
        self.update_zero_and_negative_flags(self.register_a);
    }
    
    pub fn load_and_run(&mut self, program: Vec<u8>) {
        self.load(program);
        self.reset();
        self.run();
    }

    pub fn load(&mut self, program: Vec<u8>) {
        // Temporary solution: load the program to the new VRAM.
        for i in 0..(program.len() as u16) {
            self.mem_write(0x0000 + i, program[i as usize]);
        }
        self.mem_write_u16(0xFFFC, 0x0000);

        // self.memory[0x0600 .. (0x0600 + program.len())]
        //     .copy_from_slice(&program[..]);
        // self.mem_write_u16(0xFFFC, 0x0600);

        // // The memory addresses [ 0x8000 .. 0xFFFF ] correspond to Program ROM
        // self.memory[0x8000 .. (0x8000 + program.len())]
        //     .copy_from_slice(&program[..]);
        // // Store the location of the first opcode in the address 0xFFFC, which is the first read by the NES CPU.
        // self.mem_write_u16(0xFFFC, 0x8000);
    }

    pub fn reset(&mut self) {
        self.register_a = 0;
        self.register_x = 0;
        self.register_y = 0;
        self.stack_pointer = STACK_RESET;
        self.status = CpuFlags::from_bits_truncate(0b100100);

        self.program_counter = self.mem_read_u16(0xFFFC);
    }

    pub fn run(&mut self) {
        self.run_with_callback(|_| {});
    }

    pub fn run_with_callback<F>(&mut self, mut callback: F)
    where 
        F: FnMut(&mut CPU)
    {
        let ref opcodes: HashMap<u8, &'static opcodes::OpCode> = *opcodes::OPCODES_MAP;

        loop {
            callback(self);

            let code: u8 = self.mem_read(self.program_counter);
            self.program_counter += 1;

            let program_counter_state = self.program_counter;

            let opcode = opcodes.get(&code).expect(&format!("OpCode {:x} is not recognized", code));

            match code {
                /* Arithmetic */

                // ADC
                0x69 | 0x65 | 0x75 | 0x6d | 0x7d | 0x79 | 0x61 | 0x71 => {
                    self.adc(&opcode.mode);
                }

                // SBC 
                0xe9 | 0xe5 | 0xf5 | 0xed | 0xfd | 0xf9 | 0xe1 | 0xf1 => {
                    self.sbc(&opcode.mode);
                }

                // AND
                0x29 | 0x25 | 0x35 | 0x2d | 0x3d | 0x39 | 0x21 | 0x31 => {
                    self.and(&opcode.mode);
                }

                // EOR
                0x49 | 0x45 | 0x55 | 0x4d | 0x5d | 0x59 | 0x41 | 0x51 => {
                    self.eor(&opcode.mode);
                }

                // ORA
                0x09 | 0x05 | 0x15 | 0x0d | 0x1d | 0x19 | 0x01 | 0x11 => {
                    self.ora(&opcode.mode);
                }

                /* Shifts */

                // ASL
                0x0a => self.asl_accumulator(),
                0x06 | 0x16 | 0x0e | 0x1e => {
                    self.asl(&opcode.mode);
                }

                // LSR
                0x4a => self.lsr_accumulator(),
                0x46 | 0x56 | 0x4e | 0x5e => {
                    self.lsr(&opcode.mode);
                }

                // ROL
                0x2a => self.rol_accumulator(),
                0x26 | 0x36 | 0x2e | 0x3e => {
                    self.rol(&opcode.mode);
                }

                // ROR
                0x6a => self.ror_accumulator(),
                0x66 | 0x76 | 0x6e | 0x7e => {
                    self.ror(&opcode.mode);
                }

                // INC
                0xe6 | 0xf6 | 0xee | 0xfe => {
                    self.inc(&opcode.mode);
                }

                // INX
                0xE8 => self.inx(),

                // INY
                0xC8 => self.iny(),

                // DEC
                0xc6 | 0xd6 | 0xce | 0xde => {
                    self.dec(&opcode.mode);
                }

                // DEX
                0xCA => {
                    self.dex();
                }

                // DEY
                0x88 => {
                    self.dey();
                }

                // CMP
                0xc9 | 0xc5 | 0xd5 | 0xcd | 0xdd | 0xd9 | 0xc1 | 0xd1 => {
                    self.compare(&opcode.mode, self.register_a);
                }

                // CPY
                0xc0 | 0xc4 | 0xcc => {
                    self.compare(&opcode.mode, self.register_y);
                }

                // CPX
                0xe0 | 0xe4 | 0xec => {
                    self.compare(&opcode.mode, self.register_x);
                }

                /* Branching */

                // JMP absolute
                0x4c => {
                    let mem_address = self.mem_read_u16(self.program_counter);
                    self.program_counter = mem_address;
                }

                // JMP indirect
                0x6c => {
                    let mem_address = self.mem_read_u16(self.program_counter);

                    // Manage the case in which we are reading the last byte of a page, as explained in 
                    //      http://www.6502.org/tutorials/6502opcodes.html#JMP
                    let indirect_ref = if mem_address & 0x00FF == 0x00FF {
                        let lo = self.mem_read(mem_address);
                        let hi = self.mem_read(mem_address & 0xFF00);
                        (hi as u16) << 8 | (lo as u16)
                    } else {
                        self.mem_read_u16(mem_address)
                    };

                    self.program_counter = indirect_ref;
                }

                // JSR - Jump to subroutine
                0x20 => {
                    // Add 2 to the program counter, which correspond to the 2 bytes that are read to get the address of
                    // the subroutine.
                    // Subtract 1 to account for the 1 that is added to it in the instruction RTS.
                    self.stack_push_u16(self.program_counter + 2 - 1);
                    let target_address = self.mem_read_u16(self.program_counter);
                    self.program_counter = target_address;
                }

                // RTS - Return from subroutine
                0x60 => {
                    self.program_counter = self.stack_pop_u16() + 1;
                }

                // RTI - Return from interrupt
                0x40 => {
                    self.status.bits = self.stack_pop();
                    self.status.remove(CpuFlags::BREAK);
                    self.status.insert(CpuFlags::BREAK2);
                    self.program_counter = self.stack_pop_u16();
                }

                // BNE - Branch on non equal
                0xD0 => {
                    self.branch(!self.status.contains(CpuFlags::ZERO));
                }

                // BVS - Branch on overflow set
                0x70 => {
                    self.branch(self.status.contains(CpuFlags::OVERFLOW));
                }

                // BVC - Branch on overflow clear
                0x50 => {
                    self.branch(!self.status.contains(CpuFlags::OVERFLOW));
                }

                // BMI - Branch on minus
                0x30 => {
                    self.branch(self.status.contains(CpuFlags::NEGATIVE));
                }

                // BEQ - Branch on equal
                0xF0 => {
                    self.branch(self.status.contains(CpuFlags::ZERO));
                }

                // BCS - Branch on carry set
                0xB0 => {
                    self.branch(self.status.contains(CpuFlags::CARRY));
                }

                // BCC - Branch on carry clear
                0x90 => {
                    self.branch(!self.status.contains(CpuFlags::CARRY));
                }

                // BPL - Branch on plus
                0x10 => {
                    self.branch(!self.status.contains(CpuFlags::NEGATIVE));
                }

                // BIT
                0x24 | 0x2c => {
                    self.bit(&opcode.mode);
                }

                /* Stores and loads */

                // LDA
                0xA9 | 0xA5 | 0xB5 | 0xAD | 0xBD | 0xB9 | 0xA1 | 0xB1 => {
                    self.lda(&opcode.mode);
                }

                // LDX
                0xa2 | 0xa6 | 0xb6 | 0xae | 0xbe => {
                    self.ldx(&opcode.mode);
                }

                // LDY
                0xa0 | 0xa4 | 0xb4 | 0xac | 0xbc => {
                    self.ldy(&opcode.mode);
                }

                // STA
                0x85 | 0x95 | 0x8D | 0x9D | 0x99 | 0x81 | 0x91 => {
                    self.sta(&opcode.mode);
                }

                // STX - Store X register
                0x86 | 0x96 | 0x8e => {
                    let address = self.get_operand_address(&opcode.mode);
                    self.mem_write(address, self.register_x);
                }

                // STY - Store Y register
                0x84 | 0x94 | 0x8c => {
                    let address = self.get_operand_address(&opcode.mode);
                    self.mem_write(address, self.register_y);
                }

                /* Clear flags */

                // CLD
                0xD8 => self.status.remove(CpuFlags::DECIMAL_MODE),
                // CLI
                0x58 => self.status.remove(CpuFlags::INTERRUPT_DISABLE),
                // CLV
                0xB8 => self.status.remove(CpuFlags::OVERFLOW),
                // CLC
                0x18 => self.clear_carry_flag(),
                // SEC
                0x38 => self.set_carry_flag(),
                // SEI
                0x78 => self.status.insert(CpuFlags::INTERRUPT_DISABLE),
                // SED
                0xF8 => self.status.insert(CpuFlags::DECIMAL_MODE),

                // TAX - Transfer Accumulator to X
                0xAA => self.tax(),
                // TAY - Transfer Accumulator to Y
                0xA8 => {
                    self.register_y = self.register_a;
                    self.update_zero_and_negative_flags(self.register_y);
                }
                // TSX - Transfer stack pointer to X
                0xBA => {
                    self.register_x = self.stack_pointer;
                    self.update_zero_and_negative_flags(self.register_x);
                }
                // TXA - Transfer X to A
                0x8A => {
                    self.register_a = self.register_x;
                    self.update_zero_and_negative_flags(self.register_a);
                }
                // TXS - Transfer X to stack pointer
                0x9A => {
                    self.stack_pointer = self.register_x;
                }
                // TYA - Transfer Y to A
                0x98 => {
                    self.register_a = self.register_y;
                    self.update_zero_and_negative_flags(self.register_a);
                }

                /* Stack */

                // PHA - Push accumulator
                0x48 => self.stack_push(self.register_a),
                // PLA
                0x68 => self.pla(),
                // PHP
                0x08 => self.php(),
                // PLP
                0x28 => self.plp(),

                /* Unofficial opcodes */

                /* DCP */
                0xc7 | 0xd7 | 0xCF | 0xdF | 0xdb | 0xd3 | 0xc3 => {
                    let addr = self.get_operand_address(&opcode.mode);
                    let mut data = self.mem_read(addr);
                    data = data.wrapping_sub(1);
                    self.mem_write(addr, data);
                    // self._update_zero_and_negative_flags(data);
                    if data <= self.register_a {
                        self.status.insert(CpuFlags::CARRY);
                    }

                    self.update_zero_and_negative_flags(self.register_a.wrapping_sub(data));
                }

                /* RLA */
                0x27 | 0x37 | 0x2F | 0x3F | 0x3b | 0x33 | 0x23 => {
                    let data = self.rol(&opcode.mode);
                    self.and_with_register_a(data);
                }

                /* SLO */ //todo tests
                0x07 | 0x17 | 0x0F | 0x1f | 0x1b | 0x03 | 0x13 => {
                    let data = self.asl(&opcode.mode);
                    self.or_with_register_a(data);
                }

                /* SRE */ //todo tests
                0x47 | 0x57 | 0x4F | 0x5f | 0x5b | 0x43 | 0x53 => {
                    let data = self.lsr(&opcode.mode);
                    self.xor_with_register_a(data);
                }

                /* SKB */
                0x80 | 0x82 | 0x89 | 0xc2 | 0xe2 => {
                    /* 2 byte NOP (immediate ) */
                    // todo: might be worth doing the read
                }

                /* AXS */
                0xCB => {
                    let addr = self.get_operand_address(&opcode.mode);
                    let data = self.mem_read(addr);
                    let x_and_a = self.register_x & self.register_a;
                    let result = x_and_a.wrapping_sub(data);

                    if data <= x_and_a {
                        self.status.insert(CpuFlags::CARRY);
                    }
                    self.update_zero_and_negative_flags(result);

                    self.register_x = result;
                }

                /* ARR */
                0x6B => {
                    let addr = self.get_operand_address(&opcode.mode);
                    let data = self.mem_read(addr);
                    self.and_with_register_a(data);
                    self.ror_accumulator();
                    //todo: registers
                    let result = self.register_a;
                    let bit_5 = (result >> 5) & 1;
                    let bit_6 = (result >> 6) & 1;

                    if bit_6 == 1 {
                        self.status.insert(CpuFlags::CARRY)
                    } else {
                        self.status.remove(CpuFlags::CARRY)
                    }

                    if bit_5 ^ bit_6 == 1 {
                        self.status.insert(CpuFlags::OVERFLOW);
                    } else {
                        self.status.remove(CpuFlags::OVERFLOW);
                    }

                    self.update_zero_and_negative_flags(result);
                }

                /* unofficial SBC */
                0xeb => {
                    let addr = self.get_operand_address(&opcode.mode);
                    let data = self.mem_read(addr);
                    self.sub_from_register_a(data);
                }

                /* ANC */
                0x0b | 0x2b => {
                    let addr = self.get_operand_address(&opcode.mode);
                    let data = self.mem_read(addr);
                    self.and_with_register_a(data);
                    if self.status.contains(CpuFlags::NEGATIVE) {
                        self.status.insert(CpuFlags::CARRY);
                    } else {
                        self.status.remove(CpuFlags::CARRY);
                    }
                }

                /* ALR */
                0x4b => {
                    let addr = self.get_operand_address(&opcode.mode);
                    let data = self.mem_read(addr);
                    self.and_with_register_a(data);
                    self.lsr_accumulator();
                }

                //todo: test for everything below

                /* NOP read */
                0x04 | 0x44 | 0x64 | 0x14 | 0x34 | 0x54 | 0x74 | 0xd4 | 0xf4 | 0x0c | 0x1c
                | 0x3c | 0x5c | 0x7c | 0xdc | 0xfc => {
                    let addr = self.get_operand_address(&opcode.mode);
                    let _data = self.mem_read(addr);
                    /* do nothing */
                }

                /* RRA */
                0x67 | 0x77 | 0x6f | 0x7f | 0x7b | 0x63 | 0x73 => {
                    let data = self.ror(&opcode.mode);
                    self.add_to_register_a(data);
                }

                /* ISB */
                0xe7 | 0xf7 | 0xef | 0xff | 0xfb | 0xe3 | 0xf3 => {
                    let data = self.inc(&opcode.mode);
                    self.sub_from_register_a(data);
                }

                /* NOPs */
                0x02 | 0x12 | 0x22 | 0x32 | 0x42 | 0x52 | 0x62 | 0x72 | 0x92 | 0xb2 | 0xd2
                | 0xf2 => { /* do nothing */ }

                0x1a | 0x3a | 0x5a | 0x7a | 0xda | 0xfa => { /* do nothing */ }

                /* LAX */
                0xa7 | 0xb7 | 0xaf | 0xbf | 0xa3 | 0xb3 => {
                    let addr = self.get_operand_address(&opcode.mode);
                    let data = self.mem_read(addr);
                    self.set_register_a(data);
                    self.register_x = self.register_a;
                }

                /* SAX */
                0x87 | 0x97 | 0x8f | 0x83 => {
                    let data = self.register_a & self.register_x;
                    let addr = self.get_operand_address(&opcode.mode);
                    self.mem_write(addr, data);
                }

                /* LXA */
                0xab => {
                    self.lda(&opcode.mode);
                    self.tax();
                }

                /* XAA */
                0x8b => {
                    self.register_a = self.register_x;
                    self.update_zero_and_negative_flags(self.register_a);
                    let addr = self.get_operand_address(&opcode.mode);
                    let data = self.mem_read(addr);
                    self.and_with_register_a(data);
                }

                /* LAS */
                0xbb => {
                    let addr = self.get_operand_address(&opcode.mode);
                    let mut data = self.mem_read(addr);
                    data = data & self.stack_pointer;
                    self.register_a = data;
                    self.register_x = data;
                    self.stack_pointer = data;
                    self.update_zero_and_negative_flags(data);
                }

                /* TAS */
                0x9b => {
                    let data = self.register_a & self.register_x;
                    self.stack_pointer = data;
                    let mem_address =
                        self.mem_read_u16(self.program_counter) + self.register_y as u16;

                    let data = ((mem_address >> 8) as u8 + 1) & self.stack_pointer;
                    self.mem_write(mem_address, data)
                }

                /* AHX  Indirect Y */
                0x93 => {
                    let pos: u8 = self.mem_read(self.program_counter);
                    let mem_address = self.mem_read_u16(pos as u16) + self.register_y as u16;
                    let data = self.register_a & self.register_x & (mem_address >> 8) as u8;
                    self.mem_write(mem_address, data)
                }

                /* AHX Absolute Y*/
                0x9f => {
                    let mem_address =
                        self.mem_read_u16(self.program_counter) + self.register_y as u16;

                    let data = self.register_a & self.register_x & (mem_address >> 8) as u8;
                    self.mem_write(mem_address, data)
                }

                /* SHX */
                0x9e => {
                    let mem_address =
                        self.mem_read_u16(self.program_counter) + self.register_y as u16;

                    // todo if cross page boundry {
                    //     mem_address &= (self.x as u16) << 8;
                    // }
                    let data = self.register_x & ((mem_address >> 8) as u8 + 1);
                    self.mem_write(mem_address, data)
                }

                /* SHY */
                0x9c => {
                    let mem_address =
                        self.mem_read_u16(self.program_counter) + self.register_x as u16;
                    let data = self.register_y & ((mem_address >> 8) as u8 + 1);
                    self.mem_write(mem_address, data)
                }

                // NOP - No operation
                0xEA => {}
                // BRK - Break
                0x00 => return,
            }

            // Move the program counter, if it has not been modified by the current instruction.
            if program_counter_state == self.program_counter {
                self.program_counter += (opcode.len - 1) as u16;
            }
        }
    }

    fn update_zero_and_negative_flags(&mut self, result: u8) {
        // Set the CPU flag corresponding to value equal to zero
        if result == 0 {
            self.status.insert(CpuFlags::ZERO);
        } else {
            self.status.remove(CpuFlags::ZERO);
        }

        // Set the CPU flag corresponding to negative value
        if (result & 0b1000_0000) != 0 {
            self.status.insert(CpuFlags::NEGATIVE);
        } else {
            self.status.remove(CpuFlags::NEGATIVE);
        }
    }

    /* Arithmetic */

    // Add a value to the register A, taking into account the carry and overflow flags.
    // http://www.righto.com/2012/12/the-6502-overflow-flag-explained.html
    // We do not consider decimal mode, since it is not used by the NES processor.
    fn add_to_register_a(&mut self, data: u8) {
        let sum = self.register_a as u16
                + data as u16
                + (if self.status.contains(CpuFlags::CARRY) {
                    1
                } else {
                    0
                }) as u16;

        // Set carry flag if needed
        if sum > 0xff {
            self.status.insert(CpuFlags::CARRY);
        } else {
            self.status.remove(CpuFlags::CARRY);
        }

        // Set overflow flag if needed
        let result = sum as u8;
        if (data ^ result) & (result ^ self.register_a) & 0x80 != 0 {
            self.status.insert(CpuFlags::OVERFLOW);
        } else {
            self.status.remove(CpuFlags::OVERFLOW);
        }

        self.set_register_a(result);
    }

    // ADC - Add and carry
    fn adc(&mut self, mode: &AddressingMode) {
        let address = self.get_operand_address(&mode);
        let value = self.mem_read(address);
        self.add_to_register_a(value);
    }

    // SBC - subtract and carry
    fn sbc(&mut self, mode: &AddressingMode) {
        let address = self.get_operand_address(&mode);
        let value = self.mem_read(address);
        // The quantity "((data as i8).wrapping_neg().wrapping_sub(1)) as u8" is the ones-complement of data, used to
        // compute the subtraction as an addition, as explained in:
        //      http://www.righto.com/2012/12/the-6502-overflow-flag-explained.html
        // In particular (B = 1 - C, where B = borrow and C = carry):
        //      A - N - B
        //      = A - N - B + 256
        //      = A - N - (1-C) + 256
        //      = A + (255-N) + C
        //      = A + (ones complement of N) + C
        // The addition of C is performed inside "add_to_register_a", so we need to compute the ones complemento of N.
        // In the reference for the emulator, the ones-complement is referred to as !N, but we still need to consider the 
        // borrow/carry flag, which is where the wrapping_sub(1) commes in.
        self.add_to_register_a(((value as i8).wrapping_neg().wrapping_sub(1)) as u8);
    }

    // AND - bitwise AND with accumulator
    fn and(&mut self, mode: &AddressingMode) {
        let address = self.get_operand_address(&mode);
        let value = self.mem_read(address);
        self.set_register_a(value & self.register_a);
    }

    // EOR - bitwise exclusive OR with accumulator
    fn eor(&mut self, mode: &AddressingMode) {
        let address = self.get_operand_address(&mode);
        let value = self.mem_read(address);
        self.set_register_a(value ^ self.register_a);
    }

    // ORA - bitwise OR with accumulator
    fn ora(&mut self, mode: &AddressingMode) {
        let address = self.get_operand_address(&mode);
        let value = self.mem_read(address);
        self.set_register_a(value | self.register_a);
    }

    /* Shifts */

    // ASL - Arithmetic shift left
    fn asl(&mut self, mode: &AddressingMode) -> u8 {
        let address = self.get_operand_address(&mode);
        let mut data = self.mem_read(address);

        if data >> 7 == 1 {
            self.set_carry_flag();
        } else {
            self.clear_carry_flag();
        }

        data = data << 1;
        self.mem_write(address, data);
        self.update_zero_and_negative_flags(data);
        data
    }
    // ASL - Arithmetic shift in the accumulator
    fn asl_accumulator(&mut self) {
        let mut data = self.register_a;

        if data >> 7 == 1 {
            self.set_carry_flag();
        } else {
            self.clear_carry_flag();
        }

        data = data << 1;
        self.set_register_a(data);
    }

    // LSR - Logical shift right
    fn lsr(&mut self, mode: &AddressingMode) -> u8 {
        let address = self.get_operand_address(&mode);
        let mut data = self.mem_read(address);

        if data & 1 == 1 {
            self.set_carry_flag();
        } else {
            self.clear_carry_flag();
        }

        data = data >> 1;
        self.mem_write(address, data);
        self.update_zero_and_negative_flags(data);
        data
    }
    // LSR - Logical shift right in the accumulator
    fn lsr_accumulator(&mut self) {
        let mut data = self.register_a;

        if data & 1 == 1 {
            self.set_carry_flag();
        } else {
            self.clear_carry_flag();
        }

        data = data >> 1;
        self.set_register_a(data);
    }

    // ROL - Rotate left
    fn rol(&mut self, mode: &AddressingMode) -> u8 {
        let address = self.get_operand_address(&mode);
        let mut data = self.mem_read(address);
        let old_carry = self.status.contains(CpuFlags::CARRY);
        
        if data >> 7 == 1 {
            self.set_carry_flag();
        } else {
            self.clear_carry_flag();
        }
        data = data << 1;
        if old_carry {
            data = data | 1;
        }

        self.mem_write(address, data);
        self.update_zero_and_negative_flags(data);
        data
    }
    // ROL - Rotate left the accumulator
    fn rol_accumulator(&mut self) {
        let mut data = self.register_a;
        let old_carry = self.status.contains(CpuFlags::CARRY);
        
        if data >> 7 == 1 {
            self.set_carry_flag();
        } else {
            self.clear_carry_flag();
        }
        data = data << 1;
        if old_carry {
            data = data | 1;
        }

        self.set_register_a(data);
    }

    // ROR - Rotate right
    fn ror(&mut self, mode: &AddressingMode) -> u8 {
        let address = self.get_operand_address(&mode);
        let mut data = self.mem_read(address);
        let old_carry = self.status.contains(CpuFlags::CARRY);
        
        if data & 1 == 1 {
            self.set_carry_flag();
        } else {
            self.clear_carry_flag();
        }
        data = data >> 1;
        if old_carry {
            data = data | 0b10000000;
        }

        self.mem_write(address, data);
        self.update_zero_and_negative_flags(data);
        data
    }
    // ROR - Rotate right the accumulator
    fn ror_accumulator(&mut self) {
        let mut data = self.register_a;
        let old_carry = self.status.contains(CpuFlags::CARRY);
        
        if data & 1 == 1 {
            self.set_carry_flag();
        } else {
            self.clear_carry_flag();
        }
        data = data >> 1;
        if old_carry {
            data = data | 0b10000000;
        }

        self.set_register_a(data);
    }

    // INC - Increment memory
    fn inc(&mut self, mode: &AddressingMode) -> u8 {
        let address = self.get_operand_address(&mode);
        let mut data = self.mem_read(address);

        data = data.wrapping_add(1);

        self.mem_write(address, data);
        self.update_zero_and_negative_flags(data);
        data
    }

    // INX - Increment X Register
    fn inx(&mut self) {
        // Add 1 and wrap if there is overflow.
        self.register_x = self.register_x.wrapping_add(1);
        self.update_zero_and_negative_flags(self.register_x);
    }

    // INY - Increment Y Register
    fn iny(&mut self) {
        // Add 1 and wrap if there is overflow.
        self.register_y = self.register_y.wrapping_add(1);
        self.update_zero_and_negative_flags(self.register_y);
    }

    // DEC - Decrement memory
    fn dec(&mut self, mode: &AddressingMode) -> u8 {
        let address = self.get_operand_address(&mode);
        let mut data = self.mem_read(address);

        data = data.wrapping_sub(1);

        self.mem_write(address, data);
        self.update_zero_and_negative_flags(data);
        data
    }

    // DEX - Decrement X Register
    fn dex(&mut self) {
        self.register_x = self.register_x.wrapping_sub(1);
        self.update_zero_and_negative_flags(self.register_x);
    }

    // DEY - Decrement Y Register
    fn dey(&mut self) {
        self.register_y = self.register_y.wrapping_sub(1);
        self.update_zero_and_negative_flags(self.register_y);
    }

    // CMP - Compare accumulator
    fn compare(&mut self, mode: &AddressingMode, compare_with: u8) {
        let address = self.get_operand_address(&mode);
        let data = self.mem_read(address);

        if data <= compare_with {
            self.status.insert(CpuFlags::CARRY);
        } else {
            self.status.remove(CpuFlags::CARRY);
        }

        self.update_zero_and_negative_flags(compare_with.wrapping_sub(data));
    }

    /* Branching */

    fn branch(&mut self, condition: bool) {
        if condition {
            let jump: i8 = self.mem_read(self.program_counter) as i8;
            let jump_address = self
                .program_counter
                .wrapping_add(1)
                .wrapping_add(jump as u16);

            self.program_counter = jump_address;
        }
    }

    // BIT - test BITs
    fn bit(&mut self, mode: &AddressingMode) {
        let address = self.get_operand_address(&mode);
        let value = self.mem_read(address);

        let and = self.register_a & value;

        if and == 0 {
            self.status.insert(CpuFlags::ZERO);
        } else {
            self.status.remove(CpuFlags::ZERO);
        }

        self.status.set(CpuFlags::NEGATIVE, value & 0b10000000 > 0);
        self.status.set(CpuFlags::OVERFLOW, value & 0b01000000 > 0);
    }

    /* Stores and loads */

    // LDA - Load accumulator
    fn lda(&mut self, mode: &AddressingMode) {
        let address = self.get_operand_address(&mode);
        let value = self.mem_read(address);

        self.set_register_a(value);
    }

    // LDX - Load X register
    fn ldx(&mut self, mode: &AddressingMode) {
        let address = self.get_operand_address(mode);
        let value = self.mem_read(address);

        self.register_x = value;
        self.update_zero_and_negative_flags(self.register_x);
    }

    // LDY - Load Y register
    fn ldy(&mut self, mode: &AddressingMode) {
        let address = self.get_operand_address(&mode);
        let value = self.mem_read(address);

        self.register_y = value;
        self.update_zero_and_negative_flags(self.register_y);
    }

    // STA - Store accumulator (saves value in A to a given address in memory)
    fn sta(&mut self, mode: &AddressingMode) {
        let address = self.get_operand_address(mode);
        self.mem_write(address, self.register_a);
    }

    /* Clear flags */

    fn set_carry_flag(&mut self) {
        self.status.insert(CpuFlags::CARRY);
    }

    fn clear_carry_flag(&mut self) {
        self.status.remove(CpuFlags::CARRY);
    }

    // TAX - Transfer Accumulator to X
    fn tax(&mut self) {
        self.register_x = self.register_a;
        self.update_zero_and_negative_flags(self.register_x);
    }

    /* Stack */

    // PLA - Pull accumulator
    fn pla(&mut self) {
        let data = self.stack_pop();
        self.set_register_a(data);
    }

    // PHP - Push processor status
    fn php(&mut self) {
        let mut flags = self.status.clone();
        flags.insert(CpuFlags::BREAK);
        flags.insert(CpuFlags::BREAK2);
        self.stack_push(flags.bits());
    }

    // PLP - Pull processor status
    fn plp(&mut self) {
        self.status.bits = self.stack_pop();
        self.status.remove(CpuFlags::BREAK);
        self.status.insert(CpuFlags::BREAK2);
    }

    /* Unofficial opcodes */
    fn sub_from_register_a(&mut self, data: u8) {
        self.add_to_register_a(((data as u8).wrapping_neg().wrapping_sub(1)) as u8);
    }

    fn and_with_register_a(&mut self, data: u8) {
        self.set_register_a(data & self.register_a);
    }

    fn xor_with_register_a(&mut self, data: u8) {
        self.set_register_a(data ^ self.register_a);
    }

    fn or_with_register_a(&mut self, data: u8) {
        self.set_register_a(data | self.register_a);
    }
}


#[cfg(test)]
mod test {
    use super::*;
    use crate::cartridge::test;

    // Tests from section 3.1

    #[test]
    fn test_0xa9_lda_immediate_load_data() {
        let bus = Bus::new(test::test_rom(vec![0xa9, 0x05, 0x00]));
        let mut cpu = CPU::new(bus);

        cpu.run();

        assert_eq!(cpu.register_a, 0x05);
        assert!(cpu.status.bits() & 0b0000_0010 == 0x00);
        assert!(cpu.status.bits() & 0b1000_0000 == 0x00);
    }

    #[test]
    fn test_0xa9_lda_zero_flag() {
        let bus = Bus::new(test::test_rom(vec![0xA9, 0x00, 0x00]));
        let mut cpu = CPU::new(bus);

        cpu.run();

        assert!(cpu.status.bits() & 0b0000_0010 == 0b10);
    }

    #[test]
    fn test_0xxx_tax_move_a_to_x() {
        let bus = Bus::new(test::test_rom(vec![0xA9, 0x0A, 0xAA, 0x00]));
        let mut cpu = CPU::new(bus);

        cpu.run();

        assert_eq!(cpu.register_x, 10);
    }

    #[test]
    fn test_0xe8_inx_overflow() {
        let bus = Bus::new(test::test_rom(vec![0xA9, 0xFF, 0xAA, 0xE8, 0xE8, 0x00]));
        let mut cpu = CPU::new(bus);

        cpu.run();

        assert_eq!(cpu.register_x, 1);
    }

    #[test]
    fn test_5_ops_together() {
        let bus = Bus::new(test::test_rom(vec![0xA9, 0xC0, 0xAA, 0xE8, 0x00]));
        let mut cpu = CPU::new(bus);

        cpu.run();

        assert_eq!(cpu.register_x, 0xC1);
    }

    // Tests from section 3.2

    #[test]
    fn test_lda_from_memory() {
        let bus = Bus::new(test::test_rom(vec![0xa5, 0x10, 0x00]));
        let mut cpu = CPU::new(bus);
        cpu.mem_write(0x10, 0x55);

        cpu.run();

        assert_eq!(cpu.register_a, 0x55);
    }
}
