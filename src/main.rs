use std::env;
use std::fs::File;
use std::io::Read;
use std::process;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;

struct MOS6502 {
    a: u8,
    x: u8,
    y: u8,
    s: u8,
    pc: u16,
    carry: bool,
    zero: bool,
    interrupt_disable: bool,
    decimal_mode: bool,
    overflow: bool,
    negative: bool,

    ram: [u8; 0x800],
    // mirror_ram: [u8; 0x800 * 3],
    ppu_regs: [u8; 8],
    // mirror_ppu_regs: [u8; 8 * 0x3ff],
    apu_and_io_regs: [u8; 0x18],
    // test_regs: [u8; 8],
    // unused: [u8; 0x1fe0],
    cartridge: [u8; 0x10000 - 0x4020],
}

enum Button {
    Right,
    Left,
    Up,
    Down,
    A,
    B,
    Select,
    Start,
}

impl Default for MOS6502 {
    fn default() -> MOS6502 {
        MOS6502 {
            a: 0,
            x: 0,
            y: 0,
            s: 0xfd,
            pc: 0xfffc,
            carry: false,
            zero: false,
            interrupt_disable: true,
            decimal_mode: false,
            overflow: false,
            negative: false,
            ram: [0; 0x800],
            ppu_regs: [0, 0, 0b10100000, 0, 0, 0, 0, 0],
            apu_and_io_regs: [0; 0x18], // TODO
            cartridge: [0; 0x10000 - 0x4020],
        }
    }
}

impl MOS6502 {
    fn new(rom_file: &mut File) -> Self {
        let mut result: Self = Default::default();
        rom_file
            .read(&mut result.cartridge)
            .expect("Couldn't read rom file");
        result
    }

    fn dump_regs(&self) {
        println!("A: {:02x} X: {:02x} Y: {:02x}", self.a, self.x, self.y);
        println!("SP: {:04x} PC: {:04x}", self.s, self.pc);
    }

    fn flag_updation(&mut self, val: u8) {
        if val == 0 {
            self.zero = true;
        }
        self.negative = (val >> 7) != 0;
    }

    fn get_flags_byte(&self, b: bool) -> u8 {
        ((self.negative as u8) << 7)
            | ((self.overflow as u8) << 6)
            | (1u8 << 5)
            | ((b as u8) << 4)
            | ((self.decimal_mode as u8) << 3)
            | ((self.interrupt_disable as u8) << 2)
            | ((self.zero as u8) << 1)
            | (self.carry as u8)
    }

    fn push(&mut self, val: u8) {
        self.write((self.s as u16).wrapping_add(0x100), val);
        self.s = self.s.wrapping_sub(1);
    }

    fn push16(&mut self, val: u16) {
        self.push(val as u8);
        self.push((val >> 8) as u8);
    }

    fn pop(&mut self) -> u8 {
        self.s = self.s.wrapping_add(1);
        self.read((self.s as u16).wrapping_add(0x100))
    }

    fn pop16(&mut self) -> u16 {
        ((self.pop() as u16) << 8) | self.pop() as u16
    }

    fn pop_flags(&mut self) {
        let result: u8 = self.pop();
        self.negative = (result & 0b10000000) != 0;
        self.overflow = (result & 0b01000000) != 0;
        self.interrupt_disable = (result & 0b00000100) != 0; // In some cases, should be delayed by 1 instruction
        self.decimal_mode = (result & 0b00001000) != 0;
        self.carry = (result & 0b00000001) != 0;
        self.zero = (result & 0b00000010) != 0;
    }

    fn key_down(&mut self, _b: Button) {}

    fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..0x2000 => self.ram[(addr % 0x0800) as usize],
            0x2000..0x4000 => self.ppu_regs[(addr % 8) as usize],
            0x4000..0x4018 => self.apu_and_io_regs[(addr - 0x4000) as usize],
            0x4020..=0xffff => self.cartridge[(addr - 0x4020) as usize],
            _ => panic!("Invalid memory read!"),
        }
    }

    fn read16(&self, addr: u16) -> u16 {
        ((self.read(addr) as u16) << 8) | (self.read(addr.wrapping_add(1)) as u16)
    }

    fn write(&mut self, addr: u16, val: u8) {
        match addr {
            0x0000..0x2000 => self.ram[(addr % 0x0800) as usize] = val,
            0x2000..0x4000 => self.ppu_regs[(addr % 8) as usize] = val,
            0x4000..0x4018 => self.apu_and_io_regs[(addr - 0x4000) as usize] = val,
            0x4020..=0xffff => self.cartridge[(addr - 0x4020) as usize] = val,
            _ => panic!("Invalid memory write!"),
        }
    }

    fn adc(&mut self, op: u8) -> u8 {
        let result_16: u16 = (self.a as u16)
            .wrapping_add(op as u16)
            .wrapping_add(self.carry as u16);
        let result = result_16 as u8;

        self.carry = result_16 > 255;
        self.overflow =
            (is_negative(self.a) == is_negative(op)) && (is_negative(result) != is_negative(op));
        self.flag_updation(result);

        result
    }

    fn and(&mut self, op: u8) -> u8 {
        let result: u8 = self.a & op;
        self.flag_updation(result);
        result
    }

    fn asl(&mut self, op: u8) -> u8 {
        let result: u8 = op << 1;
        self.flag_updation(result);
        self.carry = is_negative(op);
        result
    }

    fn bit(&mut self, op: u8) {
        let result: u8 = self.a & op;

        self.zero = result == 0;
        self.overflow = (op & 0b01000000) != 0;
        self.negative = is_negative(op);
    }

    fn cmp(&mut self, op1: u8, op2: u8) {
        self.carry = op1 >= op2;
        self.flag_updation(op2.wrapping_sub(op1));
    }

    fn dec(&mut self, val: u8) -> u8 {
        let result: u8 = val.wrapping_sub(1);
        self.flag_updation(result);
        result
    }

    fn eor(&mut self, op: u8) -> u8 {
        let result: u8 = self.a ^ op;
        self.flag_updation(result);
        result
    }

    fn inc(&mut self, val: u8) -> u8 {
        let result: u8 = val.wrapping_sub(1);
        self.flag_updation(result);
        result
    }

    fn lsr(&mut self, op: u8) -> u8 {
        let result: u8 = op << 1;
        self.flag_updation(result);
        self.carry = (op & 1) != 0;
        result
    }

    fn ora(&mut self, op: u8) -> u8 {
        let result: u8 = self.a | op;
        self.flag_updation(result);
        result
    }

    fn rol(&mut self, op: u8) -> u8 {
        let result: u8 = (op << 1) | (self.carry as u8);
        self.carry = is_negative(op);
        result
    }

    fn ror(&mut self, op: u8) -> u8 {
        let result: u8 = ((op as u8) << 7) | (op >> 1);
        self.carry = (op & 1) != 0;
        result
    }

    fn sbc(&mut self, op: u8) -> u8 {
        let result_16: u16 = (self.a as u16)
            .wrapping_add(!op as i8 as u16)
            .wrapping_add(self.carry as u16);
        let result: u8 = result_16 as u8;
        self.carry = result_16 > 255;
        self.overflow = (is_negative(result) != is_negative(self.a))
            && (is_negative(result) == is_negative(op));
        self.flag_updation(result);
        result
    }

    fn step(&mut self) {
        let opcode: u8 = self.read(self.pc);

        let imm16: u16 = self.read16(self.pc.wrapping_add(1));

        let immediate_arg: u8 = self.read(self.pc.wrapping_add(1));

        let zero_page_addr: u16 = immediate_arg as u16;
        let zero_page_x_addr: u16 = (immediate_arg.wrapping_add(self.x)) as u16;
        let zero_page_y_addr: u16 = (immediate_arg.wrapping_add(self.y)) as u16;
        let absolute_addr: u16 = imm16;
        let absolute_x_addr: u16 = imm16.wrapping_add(self.x as u16);
        let absolute_y_addr: u16 = imm16.wrapping_add(self.y as u16);
        let indirect_addr: u16 = self.read16(absolute_addr);
        let indirect_x_addr: u16 =
            ((self.read((immediate_arg.wrapping_add(self.x).wrapping_add(1)) as u16) as u16) << 8)
                | (self.read((immediate_arg.wrapping_add(self.x)) as u16) as u16);
        let indirect_y_addr: u16 = (((self.read((immediate_arg.wrapping_add(1)) as u16) as u16)
            << 8)
            | self.read(immediate_arg as u16) as u16)
            .wrapping_add(self.y as u16);

        let zero_page_arg: u8 = self.read(zero_page_addr);
        let zero_page_x_arg: u8 = self.read(zero_page_x_addr);
        let zero_page_y_arg: u8 = self.read(zero_page_y_addr);
        let absolute_arg: u8 = self.read(absolute_addr);
        let absolute_x_arg: u8 = self.read(absolute_x_addr);
        let absolute_y_arg: u8 = self.read(absolute_y_addr);
        let indirect_arg: u16 = self.read16(indirect_addr);
        let indirect_x_arg: u8 = self.read(indirect_x_addr);
        let indirect_y_arg: u8 = self.read(indirect_y_addr);

        println!("Executing {:02x}", opcode);
        match opcode {
            // ADC
            0x69 => {
                self.a = self.adc(immediate_arg);
                self.pc = self.pc.wrapping_add(2);
            }
            0x65 => {
                self.a = self.adc(zero_page_arg);
                self.pc = self.pc.wrapping_add(2);
            }
            0x75 => {
                self.a = self.adc(zero_page_x_arg);
                self.pc = self.pc.wrapping_add(2);
            }
            0x6d => {
                self.a = self.adc(absolute_arg);
                self.pc = self.pc.wrapping_add(3);
            }
            0x7d => {
                self.a = self.adc(absolute_x_arg);
                self.pc = self.pc.wrapping_add(3);
            }
            0x79 => {
                self.a = self.adc(absolute_y_arg);
                self.pc = self.pc.wrapping_add(3);
            }
            0x61 => {
                self.a = self.adc(indirect_x_arg);
                self.pc = self.pc.wrapping_add(2);
            }
            0x71 => {
                self.a = self.adc(indirect_y_arg);
                self.pc = self.pc.wrapping_add(2);
            }

            // AND
            0x29 => {
                self.a = self.and(immediate_arg);
                self.pc = self.pc.wrapping_add(2);
            }
            0x25 => {
                self.a = self.and(zero_page_arg);
                self.pc = self.pc.wrapping_add(2);
            }
            0x35 => {
                self.a = self.and(zero_page_x_arg);
                self.pc = self.pc.wrapping_add(2);
            }
            0x2d => {
                self.a = self.and(absolute_arg);
                self.pc = self.pc.wrapping_add(3);
            }
            0x3d => {
                self.a = self.and(absolute_x_arg);
                self.pc = self.pc.wrapping_add(3);
            }
            0x39 => {
                self.a = self.and(absolute_y_arg);
                self.pc = self.pc.wrapping_add(3);
            }
            0x21 => {
                self.a = self.and(indirect_x_arg);
                self.pc = self.pc.wrapping_add(2);
            }
            0x31 => {
                self.a = self.and(indirect_y_arg);
                self.pc = self.pc.wrapping_add(2);
            }

            // ASL
            0x0a => {
                self.a = self.asl(self.a);
                self.pc = self.pc.wrapping_add(1);
            }
            0x06 => {
                let result: u8 = self.asl(zero_page_arg);
                self.write(zero_page_addr, result);
                self.pc = self.pc.wrapping_add(2);
            }
            0x16 => {
                let result: u8 = self.asl(zero_page_x_arg);
                self.write(zero_page_x_addr, result);
                self.pc = self.pc.wrapping_add(2);
            }
            0x0e => {
                let result: u8 = self.asl(absolute_arg);
                self.write(absolute_addr, result);
                self.pc = self.pc.wrapping_add(3);
            }
            0x1e => {
                let result: u8 = self.asl(absolute_x_arg);
                self.write(absolute_x_addr, result);
                self.pc = self.pc.wrapping_add(3);
            }

            // BCC
            0x90 => {
                if !self.carry {
                    self.pc = self
                        .pc
                        .wrapping_add(2)
                        .wrapping_add(immediate_arg as i8 as u16);
                }
            }

            // BCS
            0xB0 => {
                if self.carry {
                    self.pc = self
                        .pc
                        .wrapping_add(2)
                        .wrapping_add(immediate_arg as i8 as u16);
                }
            }

            // BEQ
            0xF0 => {
                if self.zero {
                    self.pc = self
                        .pc
                        .wrapping_add(2)
                        .wrapping_add(immediate_arg as i8 as u16);
                }
            }

            // BIT
            0x24 => {
                self.bit(zero_page_arg);
                self.pc = self.pc.wrapping_add(2);
            }
            0x2c => {
                self.bit(absolute_arg);
                self.pc = self.pc.wrapping_add(3);
            }

            // BMI
            0x30 => {
                if self.negative {
                    self.pc = self
                        .pc
                        .wrapping_add(2)
                        .wrapping_add(immediate_arg as i8 as u16);
                }
            }

            // BNE
            0xd0 => {
                if !self.zero {
                    self.pc = self
                        .pc
                        .wrapping_add(2)
                        .wrapping_add(immediate_arg as i8 as u16);
                }
            }

            // BPL
            0x10 => {
                if !self.negative {
                    self.pc = self
                        .pc
                        .wrapping_add(2)
                        .wrapping_add(immediate_arg as i8 as u16);
                }
            }

            // BRK
            0x00 => {
                self.push16(self.pc.wrapping_add(2));
                self.push(self.get_flags_byte(true));
                self.pc = self.read16(0xfffe);
                self.interrupt_disable = true;
            }

            // BVC
            0x50 => {
                if !self.overflow {
                    self.pc = self
                        .pc
                        .wrapping_add(2)
                        .wrapping_add(immediate_arg as i8 as u16);
                }
            }

            // BVS
            0x70 => {
                if self.overflow {
                    self.pc = self
                        .pc
                        .wrapping_add(2)
                        .wrapping_add(immediate_arg as i8 as u16);
                }
            }

            // CLC
            0x18 => {
                self.carry = false;
                self.pc = self.pc.wrapping_add(1);
            }

            // CLD
            0xd8 => {
                self.decimal_mode = false;
                self.pc = self.pc.wrapping_add(1);
            }

            // CLI
            0x58 => {
                self.interrupt_disable = false;
                self.pc = self.pc.wrapping_add(1);
            }

            // CLV
            0xb8 => {
                self.overflow = false;
                self.pc = self.pc.wrapping_add(1);
            }

            // CMP
            0xc9 => {
                self.cmp(self.a, immediate_arg);
                self.pc = self.pc.wrapping_add(2);
            }
            0xc5 => {
                self.cmp(self.a, zero_page_arg);
                self.pc = self.pc.wrapping_add(2);
            }
            0xd5 => {
                self.cmp(self.a, zero_page_x_arg);
                self.pc = self.pc.wrapping_add(2);
            }
            0xcd => {
                self.cmp(self.a, absolute_arg);
                self.pc = self.pc.wrapping_add(3);
            }
            0xdd => {
                self.cmp(self.a, absolute_x_arg);
                self.pc = self.pc.wrapping_add(3);
            }
            0xd9 => {
                self.cmp(self.a, absolute_y_arg);
                self.pc = self.pc.wrapping_add(3);
            }
            0xc1 => {
                self.cmp(self.a, indirect_x_arg);
                self.pc = self.pc.wrapping_add(2);
            }
            0xd1 => {
                self.cmp(self.a, indirect_y_arg);
                self.pc = self.pc.wrapping_add(2);
            }

            // CPX
            0xe0 => {
                self.cmp(self.x, immediate_arg);
                self.pc = self.pc.wrapping_add(2);
            }
            0xe4 => {
                self.cmp(self.x, zero_page_arg);
                self.pc = self.pc.wrapping_add(2);
            }
            0xec => {
                self.cmp(self.x, absolute_arg);
                self.pc = self.pc.wrapping_add(3);
            }

            // CPY
            0xc0 => {
                self.cmp(self.y, immediate_arg);
                self.pc = self.pc.wrapping_add(2);
            }
            0xc4 => {
                self.cmp(self.y, zero_page_arg);
                self.pc = self.pc.wrapping_add(2);
            }
            0xcc => {
                self.cmp(self.y, absolute_arg);
                self.pc = self.pc.wrapping_add(3);
            }

            // DEC
            0xc6 => {
                let result: u8 = self.dec(zero_page_arg);
                self.write(zero_page_addr, result);
                self.pc = self.pc.wrapping_add(2);
            }
            0xd6 => {
                let result: u8 = self.dec(zero_page_x_arg);
                self.write(zero_page_x_addr, result);
                self.pc = self.pc.wrapping_add(2);
            }
            0xce => {
                let result: u8 = self.dec(absolute_arg);
                self.write(absolute_addr, result);
                self.pc = self.pc.wrapping_add(3);
            }
            0xde => {
                let result: u8 = self.dec(absolute_x_arg);
                self.write(absolute_x_addr, result);
                self.pc = self.pc.wrapping_add(3);
            }

            // DEX
            0xca => {
                self.x = self.dec(self.x);
                self.pc = self.pc.wrapping_add(1);
            }

            // DEY
            0x88 => {
                self.y = self.dec(self.y);
                self.pc = self.pc.wrapping_add(1);
            }

            // EOR
            0x49 => {
                self.a = self.eor(immediate_arg);
                self.pc = self.pc.wrapping_add(2);
            }
            0x45 => {
                self.a = self.eor(zero_page_arg);
                self.pc = self.pc.wrapping_add(2);
            }
            0x55 => {
                self.a = self.eor(zero_page_x_arg);
                self.pc = self.pc.wrapping_add(2);
            }
            0x4d => {
                self.a = self.eor(absolute_arg);
                self.pc = self.pc.wrapping_add(3);
            }
            0x5d => {
                self.a = self.eor(absolute_x_arg);
                self.pc = self.pc.wrapping_add(3);
            }
            0x59 => {
                self.a = self.eor(absolute_y_arg);
                self.pc = self.pc.wrapping_add(3);
            }
            0x41 => {
                self.a = self.eor(indirect_x_arg);
                self.pc = self.pc.wrapping_add(2);
            }
            0x51 => {
                self.a = self.eor(indirect_y_arg);
                self.pc = self.pc.wrapping_add(2);
            }

            // INC
            0xe6 => {
                let result: u8 = self.inc(zero_page_arg);
                self.write(zero_page_addr, result);
                self.pc = self.pc.wrapping_add(2);
            }
            0xf6 => {
                let result: u8 = self.inc(zero_page_x_arg);
                self.write(zero_page_x_addr, result);
                self.pc = self.pc.wrapping_add(2);
            }
            0xee => {
                let result: u8 = self.inc(absolute_arg);
                self.write(absolute_addr, result);
                self.pc = self.pc.wrapping_add(3);
            }
            0xfe => {
                let result: u8 = self.inc(absolute_x_arg);
                self.write(absolute_x_addr, result);
                self.pc = self.pc.wrapping_add(3);
            }

            // INX
            0xe8 => {
                self.x = self.inc(self.x);
                self.pc = self.pc.wrapping_add(1);
            }

            // INY
            0xc8 => {
                self.y = self.inc(self.y);
                self.pc = self.pc.wrapping_add(1);
            }

            // JMP
            0x4c => {
                self.pc = absolute_addr;
            }
            0x6c => {
                self.pc = indirect_arg;
            }

            // JSR
            0x20 => {
                self.push16(self.pc.wrapping_add(2));
                self.pc = absolute_addr;
            }

            // LDA
            0xa9 => {
                self.a = immediate_arg;
                self.flag_updation(self.a);
                self.pc = self.pc.wrapping_add(2);
            }
            0xa5 => {
                self.a = zero_page_arg;
                self.flag_updation(self.a);
                self.pc = self.pc.wrapping_add(2);
            }
            0xb5 => {
                self.a = zero_page_x_arg;
                self.flag_updation(self.a);
                self.pc = self.pc.wrapping_add(2);
            }
            0xad => {
                self.a = absolute_arg;
                self.flag_updation(self.a);
                self.pc = self.pc.wrapping_add(3);
            }
            0xbd => {
                self.a = absolute_x_arg;
                self.flag_updation(self.a);
                self.pc = self.pc.wrapping_add(3);
            }
            0xb9 => {
                self.a = absolute_y_arg;
                self.flag_updation(self.a);
                self.pc = self.pc.wrapping_add(3);
            }
            0xa1 => {
                self.a = indirect_x_arg;
                self.flag_updation(self.a);
                self.pc = self.pc.wrapping_add(2);
            }
            0xb1 => {
                self.a = indirect_y_arg;
                self.flag_updation(self.a);
                self.pc = self.pc.wrapping_add(2);
            }

            // LDX
            0xa2 => {
                self.x = immediate_arg;
                self.flag_updation(self.x);
                self.pc = self.pc.wrapping_add(2);
            }
            0xa6 => {
                self.x = zero_page_arg;
                self.flag_updation(self.x);
                self.pc = self.pc.wrapping_add(2);
            }
            0xb6 => {
                self.x = zero_page_y_arg;
                self.flag_updation(self.x);
                self.pc = self.pc.wrapping_add(2);
            }
            0xae => {
                self.x = absolute_arg;
                self.flag_updation(self.x);
                self.pc = self.pc.wrapping_add(3);
            }
            0xbe => {
                self.x = absolute_y_arg;
                self.flag_updation(self.x);
                self.pc = self.pc.wrapping_add(3);
            }

            // LDY
            0xa0 => {
                self.y = immediate_arg;
                self.flag_updation(self.y);
                self.pc = self.pc.wrapping_add(2);
            }
            0xa4 => {
                self.y = zero_page_arg;
                self.flag_updation(self.y);
                self.pc = self.pc.wrapping_add(2);
            }
            0xb4 => {
                self.y = zero_page_x_arg;
                self.flag_updation(self.y);
                self.pc = self.pc.wrapping_add(2);
            }
            0xac => {
                self.y = absolute_arg;
                self.flag_updation(self.y);
                self.pc = self.pc.wrapping_add(3);
            }
            0xbc => {
                self.y = absolute_x_arg;
                self.flag_updation(self.y);
                self.pc = self.pc.wrapping_add(3);
            }

            // LSR
            0x4a => {
                self.a = self.lsr(self.a);
                self.pc = self.pc.wrapping_add(1);
            }
            0x46 => {
                let result: u8 = self.lsr(zero_page_arg);
                self.write(zero_page_addr, result);
                self.pc = self.pc.wrapping_add(2);
            }
            0x56 => {
                let result: u8 = self.lsr(zero_page_x_arg);
                self.write(zero_page_x_addr, result);
                self.pc = self.pc.wrapping_add(2);
            }
            0x4e => {
                let result: u8 = self.lsr(absolute_arg);
                self.write(absolute_addr, result);
                self.pc = self.pc.wrapping_add(3);
            }
            0x5e => {
                let result: u8 = self.lsr(absolute_x_arg);
                self.write(absolute_x_addr, result);
                self.pc = self.pc.wrapping_add(3);
            }

            // NOP
            0xea => {
                self.pc = self.pc.wrapping_add(1);
            }

            // ORA
            0x09 => {
                self.a = self.ora(immediate_arg);
                self.pc = self.pc.wrapping_add(2);
            }
            0x05 => {
                self.a = self.ora(zero_page_arg);
                self.pc = self.pc.wrapping_add(2);
            }
            0x15 => {
                self.a = self.ora(zero_page_x_arg);
                self.pc = self.pc.wrapping_add(2);
            }
            0x0d => {
                self.a = self.ora(absolute_arg);
                self.pc = self.pc.wrapping_add(3);
            }
            0x1d => {
                self.a = self.ora(absolute_x_arg);
                self.pc = self.pc.wrapping_add(3);
            }
            0x19 => {
                self.a = self.ora(absolute_y_arg);
                self.pc = self.pc.wrapping_add(3);
            }
            0x01 => {
                self.a = self.ora(indirect_x_arg);
                self.pc = self.pc.wrapping_add(2);
            }
            0x11 => {
                self.a = self.ora(indirect_y_arg);
                self.pc = self.pc.wrapping_add(2);
            }

            // PHA
            0x48 => {
                self.push(self.a);
                self.pc = self.pc.wrapping_add(1);
            }

            // PHP
            0x08 => {
                self.push(self.get_flags_byte(true));
                self.pc = self.pc.wrapping_add(1);
            }

            // PLA
            0x68 => {
                let result: u8 = self.pop();
                self.flag_updation(result);
                self.pc = self.pc.wrapping_add(1);
            }

            // PLP
            0x28 => {
                self.pop_flags();
                self.pc = self.pc.wrapping_add(1);
            }

            // ROL
            0x2a => {
                self.a = self.rol(self.a);
                self.pc = self.pc.wrapping_add(1);
            }
            0x26 => {
                let result: u8 = self.rol(zero_page_arg);
                self.write(zero_page_addr, result);
                self.pc = self.pc.wrapping_add(2);
            }
            0x36 => {
                let result: u8 = self.rol(zero_page_x_arg);
                self.write(zero_page_x_addr, result);
                self.pc = self.pc.wrapping_add(2);
            }
            0x2e => {
                let result: u8 = self.rol(absolute_arg);
                self.write(absolute_addr, result);
                self.pc = self.pc.wrapping_add(3);
            }
            0x3e => {
                let result: u8 = self.rol(absolute_x_arg);
                self.write(absolute_x_addr, result);
                self.pc = self.pc.wrapping_add(3);
            }

            // ROR
            0x6a => {
                self.a = self.ror(self.a);
                self.pc = self.pc.wrapping_add(1);
            }
            0x66 => {
                let result: u8 = self.ror(zero_page_arg);
                self.write(zero_page_addr, result);
                self.pc = self.pc.wrapping_add(2);
            }
            0x76 => {
                let result: u8 = self.ror(zero_page_x_arg);
                self.write(zero_page_x_addr, result);
                self.pc = self.pc.wrapping_add(2);
            }
            0x6e => {
                let result: u8 = self.ror(absolute_arg);
                self.write(absolute_addr, result);
                self.pc = self.pc.wrapping_add(3);
            }
            0x7e => {
                let result: u8 = self.ror(absolute_x_arg);
                self.write(absolute_x_addr, result);
                self.pc = self.pc.wrapping_add(3);
            }

            // RTI
            0x40 => {
                self.pop_flags();
                self.pc = self.pop16();
            }

            // RTS
            0x60 => {
                self.pc = self.pop16().wrapping_add(1);
            }

            // SBC
            0xe9 => {
                self.a = self.sbc(immediate_arg);
                self.pc = self.pc.wrapping_add(2);
            }
            0xe5 => {
                self.a = self.sbc(zero_page_arg);
                self.pc = self.pc.wrapping_add(2);
            }
            0xf5 => {
                self.a = self.sbc(zero_page_x_arg);
                self.pc = self.pc.wrapping_add(2);
            }
            0xed => {
                self.a = self.sbc(absolute_arg);
                self.pc = self.pc.wrapping_add(3);
            }
            0xfd => {
                self.a = self.sbc(absolute_x_arg);
                self.pc = self.pc.wrapping_add(3);
            }
            0xf9 => {
                self.a = self.sbc(absolute_y_arg);
                self.pc = self.pc.wrapping_add(3);
            }
            0xe1 => {
                self.a = self.sbc(indirect_x_arg);
                self.pc = self.pc.wrapping_add(2);
            }
            0xf1 => {
                self.a = self.sbc(indirect_y_arg);
                self.pc = self.pc.wrapping_add(2);
            }

            // SEC
            0x38 => {
                self.carry = true;
                self.pc = self.pc.wrapping_add(1);
            }

            // SED
            0xf8 => {
                self.decimal_mode = true;
                self.pc = self.pc.wrapping_add(1);
            }

            // SEI
            0x78 => {
                self.interrupt_disable = true;
                self.pc = self.pc.wrapping_add(1);
            }

            // STA
            0x85 => {
                self.write(zero_page_addr, self.a);
                self.pc = self.pc.wrapping_add(2);
            }
            0x95 => {
                self.write(zero_page_x_addr, self.a);
                self.pc = self.pc.wrapping_add(2);
            }
            0x8d => {
                self.write(absolute_addr, self.a);
                self.pc = self.pc.wrapping_add(3);
            }
            0x9d => {
                self.write(absolute_x_addr, self.a);
                self.pc = self.pc.wrapping_add(3);
            }
            0x99 => {
                self.write(absolute_y_addr, self.a);
                self.pc = self.pc.wrapping_add(3);
            }
            0x81 => {
                self.write(indirect_x_addr, self.a);
                self.pc = self.pc.wrapping_add(2);
            }
            0x91 => {
                self.write(indirect_y_addr, self.a);
                self.pc = self.pc.wrapping_add(2);
            }

            // STX
            0x86 => {
                self.write(zero_page_addr, self.x);
                self.pc = self.pc.wrapping_add(2);
            }
            0x96 => {
                self.write(zero_page_y_addr, self.x);
                self.pc = self.pc.wrapping_add(2);
            }
            0x8e => {
                self.write(absolute_addr, self.x);
                self.pc = self.pc.wrapping_add(3);
            }

            // STY
            0x84 => {
                self.write(zero_page_addr, self.y);
                self.pc = self.pc.wrapping_add(2);
            }
            0x94 => {
                self.write(zero_page_x_addr, self.y);
                self.pc = self.pc.wrapping_add(2);
            }
            0x8c => {
                self.write(absolute_addr, self.y);
                self.pc = self.pc.wrapping_add(3);
            }

            // TAX
            0xaa => {
                self.x = self.a;
                self.flag_updation(self.x);
                self.pc = self.pc.wrapping_add(1);
            }

            // TAY
            0xa8 => {
                self.y = self.a;
                self.flag_updation(self.y);
                self.pc = self.pc.wrapping_add(1);
            }

            // TSX
            0xba => {
                self.x = self.s;
                self.flag_updation(self.x);
                self.pc = self.pc.wrapping_add(1);
            }

            // TXA
            0x8a => {
                self.a = self.x;
                self.flag_updation(self.a);
                self.pc = self.pc.wrapping_add(1);
            }

            // TXS
            0x9a => {
                self.s = self.x;
                self.flag_updation(self.s);
                self.pc = self.pc.wrapping_add(1);
            }

            // TYA
            0x98 => {
                self.a = self.y;
                self.flag_updation(self.a);
                self.pc = self.pc.wrapping_add(1);
            }

            _ => panic!("Invalid opcode: {:02x}", opcode),
        }
    }
}

fn is_negative(val: u8) -> bool {
    val & 0b10000000 != 0
}

fn main() {
    let args: Vec<_> = env::args_os().collect();
    if args.len() != 2 {
        println!("Usage: ./nespump <rom>");
        process::exit(1);
    }

    let mut rom_file = File::open(&args[1]).expect("Couldn't open rom file");

    let mut cpu = MOS6502::new(&mut rom_file);

    let sdl_context = sdl2::init().expect("Couldn't initialize SDL2");
    let video_subsystem = sdl_context
        .video()
        .expect("Couldn't initialize video subsystem");

    let window = video_subsystem
        .window("nespump", 256, 240)
        .position_centered()
        .build()
        .expect("Couldn't build window");

    let mut canvas = window.into_canvas().build().expect("Couldn't build canvas");
    canvas.present();
    let mut event_pump = sdl_context.event_pump().expect("Couldn't make event pump");

    println!("Started up!");
    cpu.dump_regs();
    'gameloop: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'gameloop,
                Event::KeyDown {
                    keycode: Some(Keycode::W),
                    ..
                } => cpu.key_down(Button::Up),
                Event::KeyDown {
                    keycode: Some(Keycode::S),
                    ..
                } => cpu.key_down(Button::Down),
                Event::KeyDown {
                    keycode: Some(Keycode::A),
                    ..
                } => cpu.key_down(Button::Left),
                Event::KeyDown {
                    keycode: Some(Keycode::D),
                    ..
                } => cpu.key_down(Button::Right),
                Event::KeyDown {
                    keycode: Some(Keycode::E),
                    ..
                } => cpu.key_down(Button::A),
                Event::KeyDown {
                    keycode: Some(Keycode::Q),
                    ..
                } => cpu.key_down(Button::B),
                Event::KeyDown {
                    keycode: Some(Keycode::LShift),
                    ..
                } => cpu.key_down(Button::Start),
                Event::KeyDown {
                    keycode: Some(Keycode::RShift),
                    ..
                } => cpu.key_down(Button::Select),
                _ => {}
            }
        }
        cpu.step();
        cpu.dump_regs();
        canvas.present();
    }
}
