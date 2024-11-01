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
        self.write(self.s as u16 + 0x100, val);
        self.s -= 1;
    }

    fn push16(&mut self, val: u16) {
        self.push(val as u8);
        self.push((val >> 8) as u8);
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

    fn read_within_zero_page(&self, addr: u8) -> u8 {
        self.read(addr as u16)
    }

    fn write_within_zero_page(&mut self, addr: u8, val: u8) {
        self.write(addr as u16, val);
    }

    fn read16(&self, addr: u16) -> u16 {
        ((self.read(addr) as u16) << 8) | (self.read(addr + 1) as u16)
    }

    fn read16_within_zero_page(&self, addr: u8) -> u16 {
        ((self.read_within_zero_page(addr) as u16) << 8)
            | self.read_within_zero_page(addr + 1) as u16
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
        let result_16: u16 = (self.a as u16) + (op as u16) + (self.carry as u16);
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

    fn asl(&mut self, op1: u8) -> u8 {
        let result: u8 = op1 << 1;

        self.flag_updation(result);
        self.carry = is_negative(op1);
        result
    }

    fn bit(&mut self, op: u8) {
        let result: u8 = self.a & op;

        self.zero = result == 0;
        self.overflow = (op & 0b01000000) != 0;
        self.negative = is_negative(op);
    }

    fn cmp(&mut self, op1:u8, op2: u8) {
        self.carry = op1 >= op2;
        self.flag_updation(op2 - op1);
    }

    fn dec(&mut self, addr: u16) {
        let result: u8 = self.read(addr) - 1;
        self.flag_updation(result);
        self.write(addr, result);
    }

    fn xor(&mut self, op: u8) -> u8 {
        let result: u8 = self.a ^ op;
        self.flag_updation(result);
        result
    }

    fn inc(&mut self, addr: u16) {
        let result: u8 = self.read(addr) + 1;
        self.flag_updation(result);
        self.write(addr, result);
    }

    fn step(&mut self) {
        let opcode: u8 = self.read(self.pc);

        let arg: u8 = self.read(self.pc + 1);
        let arg16: u16 = self.read16(self.pc + 1);

        println!("Executing {:02x}", opcode);
        match opcode {
            // ADC
            0x69 => {
                self.a = self.adc(arg);
                self.pc += 2;
            }
            0x65 => {
                self.a = self.adc(self.read(arg as u16));
                self.pc += 2;
            }
            0x75 => {
                self.a = self.adc(self.read((arg + self.x) as u16));
                self.pc += 2;
            }
            0x6d => {
                self.a = self.adc(self.read(arg16));
                self.pc += 3;
            }
            0x7d => {
                self.a = self.adc(self.read(arg16 + self.x as u16));
                self.pc += 3;
            }
            0x79 => {
                self.a = self.adc(self.read(arg16 + self.y as u16));
                self.pc += 3;
            }
            0x61 => {
                self.a = self.adc(self.read(self.read16_within_zero_page(arg + self.x)));
                self.pc += 2;
            }
            0x71 => {
                self.a =
                    self.adc(self.read(self.read16_within_zero_page(arg) + self.y as u16));
                self.pc += 2;
            }

            // AND
            0x29 => {
                self.a = self.and(arg);
                self.pc += 2;
            }
            0x25 => {
                self.a = self.and(self.read_within_zero_page(arg));
                self.pc += 2;
            }
            0x35 => {
                self.a = self.and(self.read_within_zero_page(arg + self.x));
                self.pc += 2;
            }
            0x2d => {
                self.a = self.and(self.read(arg16));
                self.pc += 3;
            }
            0x3d => {
                self.a = self.and(self.read(arg16 + self.x as u16));
                self.pc += 3;
            }
            0x39 => {
                self.a = self.and(self.read(arg16 + self.y as u16));
                self.pc += 3;
            }
            0x21 => {
                self.a = self.and(self.read(self.read16_within_zero_page(arg + self.x)));
                self.pc += 2;
            }
            0x31 => {
                self.a =
                    self.and(self.read(self.read16_within_zero_page(arg) + self.y as u16));
                self.pc += 2;
            }

            // ASL
            0x0a => {
                self.a = self.asl(self.a);
                self.pc += 1;
            }
            0x06 => {
                let result = self.asl(self.read_within_zero_page(arg));
                self.write_within_zero_page(arg, result);
                self.pc += 2;
            }
            0x16 => {
                let result = self.asl(self.read_within_zero_page(arg + self.x));
                self.write_within_zero_page(arg, result);
                self.pc += 2;
            }
            0x0e => {
                let result = self.asl(self.read(arg16));
                self.write(arg16, result);
                self.pc += 3;
            }
            0x1e => {
                let result = self.asl(self.read(arg16 + self.x as u16));
                self.write(arg16 + self.x as u16, result);
                self.pc += 3;
            }

            // BCC
            0x90 => {
                if !self.carry {
                    self.pc += 2 + arg as i8 as u16;
                }
            }

            // BCS
            0xB0 => {
                if self.carry {
                    self.pc += 2 + arg as i8 as u16;
                }
            }

            // BEQ
            0xF0 => {
                if self.zero {
                    self.pc += 2 + arg as i8 as u16;
                }
            }

            // BIT
            0x24 => {
                self.bit(self.read_within_zero_page(arg));
                self.pc += 2;
            }
            0x2c => {
                self.bit(self.read(arg16));
                self.pc += 3;
            }

            // BMI
            0x30 => {
                if self.negative {
                    self.pc += 2 + arg as i8 as u16;
                }
            }

            // BNE
            0xd0 => {
                if !self.zero {
                    self.pc += 2 + arg as i8 as u16;
                }
            }

            // BPL
            0x10 => {
                if !self.negative {
                    self.pc += 2 + arg as i8 as u16;
                }
            }

            // BRK
            0x00 => {
                self.push16(self.pc + 2);
                self.push(self.get_flags_byte(true));
                self.pc = self.read16(0xfffe);
                self.interrupt_disable = true;
            }

            // BVC
            0x50 => {
                if !self.overflow {
                    self.pc += 2 + arg as i8 as u16;
                }
            }

            // BVS
            0x70 => {
                if self.overflow {
                    self.pc += 2 + arg as i8 as u16;
                }
            }

            // CLC
            0x18 => {
                self.carry = false;
                self.pc += 1;
            }

            // CLD
            0xd8 => {
                self.decimal_mode = false;
                self.pc += 1;
            }

            // CLI
            0x58 => {
                self.interrupt_disable = false;
                self.pc += 1;
            }

            // CLV
            0xb8 => {
                self.overflow = false;
                self.pc += 1;
            }

            // CMP
            0xc9 => {
                self.cmp(self.a, arg);
                self.pc += 2;
            }
            0xc5 => {
                self.cmp(self.a, self.read_within_zero_page(arg));
                self.pc += 2;
            }
            0xd5 => {
                self.cmp(self.a, self.read_within_zero_page(arg + self.x));
                self.pc += 2;
            }
            0xcd => {
                self.cmp(self.a, self.read(arg16));
                self.pc += 3;
            }
            0xdd => {
                self.cmp(self.a, self.read(arg16 + self.x as u16));
                self.pc += 3;
            }
            0xd9 => {
                self.cmp(self.a, self.read(arg16 + self.y as u16));
                self.pc += 3;
            }
            0xc1 => {
                self.cmp(self.a, self.read(self.read16_within_zero_page(arg + self.x)));
                self.pc += 2;
            }
            0xd1 => {
                self.cmp(self.a, self.read(self.read16_within_zero_page(arg) + self.y as u16));
                self.pc += 2;
            }

            // CPX
            0xe0 => {
                self.cmp(self.x, arg);
                self.pc += 2;
            }
            0xe4 => {
                self.cmp(self.x, self.read_within_zero_page(arg));
                self.pc += 2;
            }
            0xec => {
                self.cmp(self.x, self.read(arg16));
                self.pc += 3;
            }

            // CPY
            0xc0 => {
                self.cmp(self.y, arg);
                self.pc += 2;
            }
            0xc4 => {
                self.cmp(self.y, self.read_within_zero_page(arg));
                self.pc += 2;
            }
            0xcc => {
                self.cmp(self.y, self.read(arg16));
                self.pc += 3;
            }

            // DEC
            0xc6 => {
                self.dec(arg as u16);
                self.pc += 2;
            }
            0xd6 => {
                self.dec((arg + self.x) as u16);
                self.pc += 2;
            }
            0xce => {
                self.dec(arg16);
                self.pc += 3;
            }
            0xde => {
                self.dec(arg16 + self.x as u16);
                self.pc += 3;
            }

            // DEX
            0xca => {
                self.x -= 1;
                self.flag_updation(self.x);
                self.pc += 1;
            }

            // DEY
            0x88 => {
                self.y -= 1;
                self.flag_updation(self.y);
                self.pc += 1;
            }

            // EOR
            0x49 => {
                self.a = self.xor(arg);
                self.pc += 2;
            }
            0x45 => {
                self.a = self.xor(self.read_within_zero_page(arg));
                self.pc += 2;
            }
            0x55 => {
                self.a = self.xor(self.read_within_zero_page(arg + self.x));
                self.pc += 2;
            }
            0x4d => {
                self.a = self.xor(self.read(arg16));
                self.pc += 3;
            }
            0x5d => {
                self.a = self.xor(self.read(arg16 + self.x as u16));
                self.pc += 3;
            }
            0x59 => {
                self.a = self.xor(self.read(arg16 + self.y as u16));
                self.pc += 3;
            }
            0x41 => {
                self.a = self.xor(self.read(self.read16_within_zero_page(arg + self.x)));
                self.pc += 2;
            }
            0x51 => {
                self.a =
                    self.xor(self.read(self.read16_within_zero_page(arg) + self.y as u16));
                self.pc += 2;
            }

            // INC
            0xe6 => {
                self.inc(arg as u16);
                self.pc += 2;
            }
            0xf6 => {
                self.inc((arg + self.x) as u16);
                self.pc += 2;
            }
            0xee => {
                self.inc(arg16);
                self.pc += 3;
            }
            0xfe => {
                self.inc(arg16 + self.x as u16);
                self.pc += 3;
            }

            // INX
            0xe8 => {
                self.x += 1;
                self.flag_updation(self.x);
                self.pc += 1;
            }

            // INY
            0xc8 => {
                self.y += 1;
                self.flag_updation(self.y);
                self.pc += 1;
            }

            // JMP
            0x4c => {
                self.pc = arg16;
            }
            0x6c => {
                self.pc = self.read16(arg16);
            }

            // JSR
            0x20 => {
                self.push16(self.pc + 2);
                self.pc = arg16;
            }

            // LDA
            0xa9 => {
                self.a = arg;
                self.flag_updation(self.a);
                self.pc += 2;
            }
            0xa5 => {
                self.a = self.read_within_zero_page(arg);
                self.flag_updation(self.a);
                self.pc += 2;
            }
            0xb5 => {
                self.a = self.read_within_zero_page(arg + self.x);
                self.flag_updation(self.a);
                self.pc += 2;
            }
            0xad => {
                self.a = self.read(arg16);
                self.flag_updation(self.a);
                self.pc += 3;
            }
            0xbd => {
                self.a = self.read(arg16 + self.x as u16);
                self.flag_updation(self.a);
                self.pc += 3;
            }
            0xb9 => {
                self.a = self.read(arg16 + self.y as u16);
                self.flag_updation(self.a);
                self.pc += 3;
            }
            0xa1 => {
                self.a = self.read(self.read16_within_zero_page(arg + self.x));
                self.flag_updation(self.a);
                self.pc += 2;
            }
            0xb1 => {
                self.a = self.read(self.read16_within_zero_page(arg) + self.y as u16);
                self.flag_updation(self.a);
                self.pc += 2;
            }

            _ => todo!(),
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
