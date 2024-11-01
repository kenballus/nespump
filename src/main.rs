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

    fn set_flags(&mut self, val: u8) {
        if val == 0 {
            self.zero = true;
        }
        self.negative = (val >> 7) != 0;
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
        self.set_flags(result);

        result
    }

    fn and(&mut self, op: u8) -> u8 {
        let result: u8 = self.a & op;
        self.set_flags(result);
        result
    }

    fn asl(&mut self, op1: u8) -> u8 {
        let result: u8 = op1 << 1;

        self.set_flags(result);
        self.carry = is_negative(op1);
        result
    }

    fn step(&mut self) {
        let opcode: u8 = self.read(self.pc);
        println!("Executing {:02x}", opcode);
        match opcode {
            // ADC
            0x69 => {
                let arg: u8 = self.read(self.pc + 1);
                self.a = self.adc(arg);
                self.pc += 2;
            }
            0x65 => {
                let arg: u8 = self.read(self.pc + 1);
                self.a = self.adc(self.read(arg as u16));
                self.pc += 2;
            }
            0x75 => {
                let arg: u8 = self.read(self.pc + 1);
                self.a = self.adc(self.read((arg + self.x) as u16));
                self.pc += 2;
            }
            0x6d => {
                let arg: u16 = self.read16(self.pc + 1);
                self.a = self.adc(self.read(arg));
                self.pc += 3;
            }
            0x7d => {
                let arg: u16 = self.read16(self.pc + 1);
                self.a = self.adc(self.read(arg + self.x as u16));
                self.pc += 3;
            }
            0x79 => {
                let arg: u16 = self.read16(self.pc + 1);
                self.a = self.adc(self.read(arg + self.y as u16));
                self.pc += 3;
            }
            0x61 => {
                let arg: u16 = self.read16(self.pc + 1);
                self.a = self.adc(self.read(self.read16_within_zero_page(arg as u8 + self.x)));
                self.pc += 2;
            }
            0x71 => {
                let arg: u16 = self.read16(self.pc + 1);
                self.a = self.adc(self.read(self.read16_within_zero_page(arg as u8) + self.y as u16));
                self.pc += 2;
            }

            // AND
            0x29 => {
                let arg: u8 = self.read(self.pc + 1);
                self.a = self.and(arg);
                self.pc += 2;
            }
            0x25 => {
                let arg: u8 = self.read(self.pc + 1);
                self.a = self.and(self.read(arg as u16));
                self.pc += 2;
            }
            0x35 => {
                let arg: u8 = self.read(self.pc + 1);
                self.a = self.and(self.read((arg + self.x) as u16));
                self.pc += 2;
            }
            0x2d => {
                let arg: u16 = self.read16(self.pc + 1);
                self.a = self.and(self.read(arg));
                self.pc += 3;
            }
            0x3d => {
                let arg: u16 = self.read16(self.pc + 1);
                self.a = self.and(self.read(arg + self.x as u16));
                self.pc += 3;
            }
            0x39 => {
                let arg: u16 = self.read16(self.pc + 1);
                self.a = self.and(self.read(arg + self.y as u16));
                self.pc += 3;
            }
            0x21 => {
                let arg: u16 = self.read16(self.pc + 1);
                self.a = self.and(self.read(self.read16_within_zero_page(arg as u8 + self.x)));
                self.pc += 2;
            }
            0x31 => {
                let arg: u16 = self.read16(self.pc + 1);
                self.a = self.and(self.read(self.read16_within_zero_page(arg as u8) + self.y as u16));
                self.pc += 2;
            }

            // ASL
            0x0a => {
                self.a = self.asl(self.a);
                self.pc += 1;
            }
            0x06 => {
                let arg: u8 = self.read(self.pc + 1);
                let result = self.asl(self.read_within_zero_page(arg));
                self.write_within_zero_page(arg, result);
                self.pc += 2;
            }
            0x16 => {
                let arg: u8 = self.read(self.pc + 1);
                let result = self.asl(self.read_within_zero_page(arg + self.x));
                self.write_within_zero_page(arg, result);
                self.pc += 2;
            }
            0x0e => {
                let arg: u16 = self.read16(self.pc + 1);
                let result = self.asl(self.read(arg));
                self.write(arg, result);
                self.pc += 3;
            }
            0x1e => {
                let arg: u16 = self.read16(self.pc + 1);
                let result = self.asl(self.read(arg + self.x as u16));
                self.write(arg + self.x as u16, result);
                self.pc += 3;
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
