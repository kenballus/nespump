use std::env;
use std::fs::File;
use std::io::Read;
use std::process;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Point;
use sdl2::render::Canvas;
use sdl2::video::Window;

const SCALE_FACTOR: usize = 1;

fn plot_px(canvas: &mut Canvas<Window>, color: Color, r: usize, c: usize) {
    canvas.set_draw_color(color);
    let mut points: [Point; SCALE_FACTOR * SCALE_FACTOR] = [Point::new(0, 0); SCALE_FACTOR * SCALE_FACTOR];
    for i in 0..SCALE_FACTOR {
        for j in 0..SCALE_FACTOR {
            points[i * SCALE_FACTOR + j] = Point::new((c * SCALE_FACTOR + j) as i32, (r * SCALE_FACTOR + i) as i32)
        }
    }
    canvas.draw_points(points.as_slice()).expect("Couldn't plot pixel");
}

fn plot_tile(canvas: &mut Canvas<Window>, tile: Tile, palette: Palette, y: usize, x: usize, h_flip: bool, v_flip: bool, allow_transparency: bool) {
    for i in 0..tile.data.len() {
        let r = if v_flip { tile.data.len() - i - 1 } else { i };
        for (j, &px) in tile.data[i].iter().enumerate() {
            if !allow_transparency || px != 0 {
                let c = if h_flip { tile.data[i].len() - j - 1 } else { j };
                plot_px(canvas, palette.data[px as usize], y + r, x + c)
            }
        }
    }
}

// Colors obtained from https://bugzmanov.github.io/nes_ebook/chapter_6_3.html
pub static SYSTEM_PALETTE: [Color; 64] = [
    Color { r: 0x80, g: 0x80, b: 0x80, a: 1 },
    Color { r: 0x00, g: 0x3D, b: 0xA6, a: 1 },
    Color { r: 0x00, g: 0x12, b: 0xB0, a: 1 },
    Color { r: 0x44, g: 0x00, b: 0x96, a: 1 },
    Color { r: 0xA1, g: 0x00, b: 0x5E, a: 1 },
    Color { r: 0xC7, g: 0x00, b: 0x28, a: 1 },
    Color { r: 0xBA, g: 0x06, b: 0x00, a: 1 },
    Color { r: 0x8C, g: 0x17, b: 0x00, a: 1 },
    Color { r: 0x5C, g: 0x2F, b: 0x00, a: 1 },
    Color { r: 0x10, g: 0x45, b: 0x00, a: 1 },
    Color { r: 0x05, g: 0x4A, b: 0x00, a: 1 },
    Color { r: 0x00, g: 0x47, b: 0x2E, a: 1 },
    Color { r: 0x00, g: 0x41, b: 0x66, a: 1 },
    Color { r: 0x00, g: 0x00, b: 0x00, a: 1 },
    Color { r: 0x05, g: 0x05, b: 0x05, a: 1 },
    Color { r: 0x05, g: 0x05, b: 0x05, a: 1 },
    Color { r: 0xC7, g: 0xC7, b: 0xC7, a: 1 },
    Color { r: 0x00, g: 0x77, b: 0xFF, a: 1 },
    Color { r: 0x21, g: 0x55, b: 0xFF, a: 1 },
    Color { r: 0x82, g: 0x37, b: 0xFA, a: 1 },
    Color { r: 0xEB, g: 0x2F, b: 0xB5, a: 1 },
    Color { r: 0xFF, g: 0x29, b: 0x50, a: 1 },
    Color { r: 0xFF, g: 0x22, b: 0x00, a: 1 },
    Color { r: 0xD6, g: 0x32, b: 0x00, a: 1 },
    Color { r: 0xC4, g: 0x62, b: 0x00, a: 1 },
    Color { r: 0x35, g: 0x80, b: 0x00, a: 1 },
    Color { r: 0x05, g: 0x8F, b: 0x00, a: 1 },
    Color { r: 0x00, g: 0x8A, b: 0x55, a: 1 },
    Color { r: 0x00, g: 0x99, b: 0xCC, a: 1 },
    Color { r: 0x21, g: 0x21, b: 0x21, a: 1 },
    Color { r: 0x09, g: 0x09, b: 0x09, a: 1 },
    Color { r: 0x09, g: 0x09, b: 0x09, a: 1 },
    Color { r: 0xFF, g: 0xFF, b: 0xFF, a: 1 },
    Color { r: 0x0F, g: 0xD7, b: 0xFF, a: 1 },
    Color { r: 0x69, g: 0xA2, b: 0xFF, a: 1 },
    Color { r: 0xD4, g: 0x80, b: 0xFF, a: 1 },
    Color { r: 0xFF, g: 0x45, b: 0xF3, a: 1 },
    Color { r: 0xFF, g: 0x61, b: 0x8B, a: 1 },
    Color { r: 0xFF, g: 0x88, b: 0x33, a: 1 },
    Color { r: 0xFF, g: 0x9C, b: 0x12, a: 1 },
    Color { r: 0xFA, g: 0xBC, b: 0x20, a: 1 },
    Color { r: 0x9F, g: 0xE3, b: 0x0E, a: 1 },
    Color { r: 0x2B, g: 0xF0, b: 0x35, a: 1 },
    Color { r: 0x0C, g: 0xF0, b: 0xA4, a: 1 },
    Color { r: 0x05, g: 0xFB, b: 0xFF, a: 1 },
    Color { r: 0x5E, g: 0x5E, b: 0x5E, a: 1 },
    Color { r: 0x0D, g: 0x0D, b: 0x0D, a: 1 },
    Color { r: 0x0D, g: 0x0D, b: 0x0D, a: 1 },
    Color { r: 0xFF, g: 0xFF, b: 0xFF, a: 1 },
    Color { r: 0xA6, g: 0xFC, b: 0xFF, a: 1 },
    Color { r: 0xB3, g: 0xEC, b: 0xFF, a: 1 },
    Color { r: 0xDA, g: 0xAB, b: 0xEB, a: 1 },
    Color { r: 0xFF, g: 0xA8, b: 0xF9, a: 1 },
    Color { r: 0xFF, g: 0xAB, b: 0xB3, a: 1 },
    Color { r: 0xFF, g: 0xD2, b: 0xB0, a: 1 },
    Color { r: 0xFF, g: 0xEF, b: 0xA6, a: 1 },
    Color { r: 0xFF, g: 0xF7, b: 0x9C, a: 1 },
    Color { r: 0xD7, g: 0xE8, b: 0x95, a: 1 },
    Color { r: 0xA6, g: 0xED, b: 0xAF, a: 1 },
    Color { r: 0xA2, g: 0xF2, b: 0xDA, a: 1 },
    Color { r: 0x99, g: 0xFF, b: 0xFC, a: 1 },
    Color { r: 0xDD, g: 0xDD, b: 0xDD, a: 1 },
    Color { r: 0x11, g: 0x11, b: 0x11, a: 1 },
    Color { r: 0x11, g: 0x11, b: 0x11, a: 1 },
];

struct Nes {
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

    cycles: u64,

    ram: [u8; 0x800],
    ppu_regs: [u8; 8],
    apu_and_io_regs: [u8; 0x18],
    cartridge: [u8; 0xbfe0],

    ppu_cartridge: [u8; 0x3f00],
    ppu_ram: [u8; 0x20],
    oam: [u8; 0x100],
    w: bool,
    ppuaddr: u16,
    ppudata: u8,
    internal_x_scroll: u8,
    internal_y_scroll: u8,
    oamdata_is_ff: bool,

    buttons: [bool; 8],
    current_button: usize,
    strobe_mode: bool,
}

struct Sprite {
    c: u8,
    r: u8,
    pattern_table_index: u8,
    palette_index: u8,
    priority: bool,
    h_flip: bool,
    v_flip: bool,
}

struct Tile {
    data: [[u8; 8]; 8],
}

struct Palette {
    data: [Color; 4],
}

fn parse_palette(data: [u8; 4]) -> Palette {
    Palette { data: [SYSTEM_PALETTE[data[0] as usize], SYSTEM_PALETTE[data[1] as usize], SYSTEM_PALETTE[data[2] as usize], SYSTEM_PALETTE[data[3] as usize]] }
}

fn parse_tile(data: [u8; 16]) -> Tile {
    let mut result: [[u8; 8]; 8] = [[0; 8]; 8];
    for (i, &byte) in data.iter().enumerate() {
        if i < 8 {
            for j in 0..8 {
                result[i][j] |= (byte >> (7 - j)) & 1
            }
        } else {
            for j in 0..8 {
                result[i - 8][j] |= ((byte >> (7 - j)) & 1) << 1
            }
        }
    }
    Tile { data: result }
}

fn parse_sprite(data: [u8; 4]) -> Sprite {
    Sprite {
        c: data[3],
        r: data[0],
        pattern_table_index: data[1],
        palette_index: data[2] & 0b11,
        priority: (data[2] & 0b100000) != 0,
        h_flip: (data[2] & 0b1000000) != 0,
        v_flip: (data[2] & 0b10000000) != 0,
    }
}

impl Default for Nes {
    fn default() -> Nes {
        Nes {
            a: 0,
            x: 0,
            y: 0,
            s: 0xfd,
            pc: 0x0000, // Gets filled in by Nes::new
            carry: false,
            zero: false,
            interrupt_disable: true,
            decimal_mode: false,
            overflow: false,
            negative: false,
            cycles: 0,
            ram: [0; 0x800],
            ppu_regs: [0, 0, 0b10100000, 0, 0, 0, 0, 0],
            apu_and_io_regs: [0; 0x18],
            cartridge: [0; 0xbfe0],
            ppu_cartridge: [0; 0x3f00],
            ppu_ram: [0; 0x20],
            oam: [0; 0x100],
            w: false,
            ppuaddr: 0,
            ppudata: 0,
            internal_x_scroll: 0,
            internal_y_scroll: 0,
            oamdata_is_ff: false,
            buttons: [false; 8],
            current_button: 0,
            strobe_mode: false,
        }
    }
}

const RESET_VECTOR: u16 = 0xfffc;
const BRK_VECTOR: u16 = 0xfffe;
const NMI_VECTOR: u16 = 0xfffa;
const PPUCTRL: u16 = 0x2000;
const PPUCTRL_I: u16 = PPUCTRL % 8;
const OAMADDR: u16 = 0x2003;
const OAMDATA: u16 = 0x2004;
const OAMDATA_I: u16 = OAMDATA % 8;
const PPUSCROLL: u16 = 0x2005;
const PPUSCROLL_I: u16 = PPUSCROLL % 8;
const PPUADDR: u16 = 0x2006;
const PPUADDR_I: u16 = PPUADDR % 8;
const PPUDATA: u16 = 0x2007;
const PPUDATA_I: u16 = PPUDATA % 8;
const PPUSTATUS: u16 = 0x2002;
const PPUSTATUS_I: u16 = PPUSTATUS % 8;
const OAMDMA: u16 = 0x4014;
const OAMDMA_I: u16 = OAMDMA % 0x18;
const JOYPAD: u16 = 0x4016;
const JOYPAD_I: u16 = JOYPAD % 0x18;

impl Nes {
    fn new(rom_file: &mut File) -> Self {
        let mut result: Self = Default::default();

        let mut magic: [u8; 4] = [0; 4];
        rom_file.read_exact(&mut magic).expect("Couldn't read magic");
        if magic != [0x4e, 0x45, 0x53, 0x1a] {
            panic!("Invalid iNes magic");
        }

        let mut raw_prg_rom_size: [u8; 1] = [0];
        rom_file.read_exact(&mut raw_prg_rom_size).expect("Couldn't read PRG ROM size");
        let prg_rom_size: u16 = raw_prg_rom_size[0] as u16;

        let mut raw_chr_rom_size: [u8; 1] = [0];
        rom_file.read_exact(&mut raw_chr_rom_size).expect("Couldn't read CHR ROM size");
        let chr_rom_size: u16 = raw_chr_rom_size[0] as u16;

        let mut raw_flags_6: [u8; 1] = [0];
        rom_file.read_exact(&mut raw_flags_6).expect("Couldn't read flags 6");

        let mut raw_flags_7: [u8; 1] = [0];
        rom_file.read_exact(&mut raw_flags_7).expect("Couldn't read flags 7");

        let mut raw_flags_8: [u8; 1] = [0];
        rom_file.read_exact(&mut raw_flags_8).expect("Couldn't read flags 8");

        let mut raw_flags_9: [u8; 1] = [0];
        rom_file.read_exact(&mut raw_flags_9).expect("Couldn't read flags 9");

        let mut raw_flags_10: [u8; 1] = [0];
        rom_file.read_exact(&mut raw_flags_10).expect("Couldn't read flags 10");

        let mut unused: [u8; 5] = [0; 5];
        rom_file.read_exact(&mut unused).expect("Couldn't read header padding");

        if prg_rom_size > 2 {
            panic!("iNes parser doesn't yet support larger PRG ROMs");
        }
        for prg_rom_no in 0..prg_rom_size {
            let mut buf: [u8; 0x4000] = [0; 0x4000];
            rom_file.read_exact(&mut buf).expect("Couldn't read PRG ROM");
            for (i, &byte) in buf.iter().enumerate() {
                result.write((if prg_rom_size == 2 { 0x8000 } else { 0xc000 }) + prg_rom_no * 0x4000 + i as u16, byte);
            }
        }

        if chr_rom_size > 1 {
            panic!("iNes parser doesn't yet support larger CHR ROMs");
        }
        for chr_rom_no in 0..chr_rom_size {
            let mut buf: [u8; 0x2000] = [0; 0x2000];
            rom_file.read_exact(&mut buf).expect("Couldn't read CHR ROM");
            for (i, &byte) in buf.iter().enumerate() {
                result.ppu_write(chr_rom_no * 0x2000 + i as u16, byte);
            }
        }

        result.pc = result.read16(RESET_VECTOR);
        result
    }

    fn get_bg_pattern_table_base(&mut self) -> u16 {
        (((self.read(PPUCTRL) >> 4) & 1) as u16) * 0x1000
    }

    fn get_sprite_pattern_table_base(&mut self) -> u16 {
        (((self.read(PPUCTRL) >> 3) & 1) as u16) * 0x1000
    }

    fn get_name_table_base(&mut self) -> u16 {
        0x2000 + ((self.read(PPUCTRL) & 0b11) as u16) * 0x400
    }

    fn get_attribute_table_base(&mut self) -> u16 {
        self.get_name_table_base() + 0x3c0
    }

    fn render_bg(&mut self, canvas: &mut Canvas<Window>) {
        let pattern_table_base = self.get_bg_pattern_table_base(); // (PPU addr)
        let name_table_base = self.get_name_table_base(); // (PPU addr)
        let attribute_table_base = self.get_attribute_table_base(); // (PPU addr)
        for r in 0..30 {
            for c in 0..32 {
                let name_table_entry: u8 = self.ppu_read(name_table_base + r * 32 + c);

                let mut raw_tile_data: [u8; 16] = [0; 16];
                for i in 0..raw_tile_data.len() {
                    raw_tile_data[i] = self.ppu_read(pattern_table_base + name_table_entry as u16 * 16 + i as u16);
                }
                let tile: Tile = parse_tile(raw_tile_data);

                let attribute_table_entry: u8 = self.ppu_read(attribute_table_base + (r / 4) * 8 + (c / 4));

                let palette_index: u16 = if r % 2 == r % 4 && c % 2 == c % 4 {
                    // upper left
                    attribute_table_entry & 0b11
                } else if r % 2 == r % 4 && c % 2 != c % 4 {
                    // upper right
                    (attribute_table_entry >> 2) & 0b11
                } else if r % 2 != r % 4 && c % 2 == c % 4 {
                    // lower left
                    (attribute_table_entry >> 4) & 0b11
                } else {
                    // lower right
                    (attribute_table_entry >> 6) & 0b11
                } as u16;

                let palette_base: u16 = 0x3f00 + 4 * palette_index; // BG_PALETTE_ADDR + sizeof(palette) * palette_index
                let mut raw_palette_data: [u8; 4] = [0; 4];
                for i in 0..raw_palette_data.len() {
                    raw_palette_data[i] = self.ppu_read(palette_base + i as u16);
                }
                let palette: Palette = parse_palette(raw_palette_data);

                plot_tile(canvas, tile, palette, (r * 8) as usize, (c * 8) as usize, false, false, false);
            }
        }
    }

    fn render_pattern_table(&mut self, canvas: &mut Canvas<Window>) {
        let pattern_table_base = self.get_bg_pattern_table_base(); // (PPU addr)
        let name_table_base = self.get_name_table_base(); // (PPU addr)
        let attribute_table_base = self.get_attribute_table_base(); // (PPU addr)
        for r in 0..30 {
            for c in 0..32 {
                let name_table_entry: u8 = (r as u16 * 32 + c as u16) as u8;

                let mut raw_tile_data: [u8; 16] = [0; 16];
                for i in 0..raw_tile_data.len() {
                    raw_tile_data[i] = self.ppu_read(pattern_table_base + name_table_entry as u16 * 16 + i as u16);
                }
                let tile: Tile = parse_tile(raw_tile_data);

                let attribute_table_entry: u8 = self.ppu_read(attribute_table_base + (r / 4) * 8 + (c / 4));

                let palette_index: u16 = if r % 2 == r % 4 && c % 2 == c % 4 {
                    // upper left
                    attribute_table_entry & 0b11
                } else if r % 2 == r % 4 && c % 2 != c % 4 {
                    // upper right
                    (attribute_table_entry >> 2) & 0b11
                } else if r % 2 != r % 4 && c % 2 == c % 4 {
                    // lower left
                    (attribute_table_entry >> 4) & 0b11
                } else {
                    // lower right
                    (attribute_table_entry >> 6) & 0b11
                } as u16;

                let palette_base: u16 = 0x3f00 + 4 * palette_index; // BG_PALETTE_ADDR + sizeof(palette) * palette_index
                let mut raw_palette_data: [u8; 4] = [0; 4];
                for i in 0..raw_palette_data.len() {
                    raw_palette_data[i] = self.ppu_read(palette_base + i as u16);
                }
                let palette: Palette = parse_palette(raw_palette_data);

                plot_tile(canvas, tile, palette, (r * 8) as usize, (c * 8) as usize, false, false, false);
            }
        }
    }

    fn render_sprites(&mut self, canvas: &mut Canvas<Window>) {
        let pattern_table_base = self.get_sprite_pattern_table_base(); // (PPU addr)
        for i in 0..(self.oam.len() / 4) {
            // Number of sprites in OAM
            let mut raw_sprite_data = [0; 4];
            raw_sprite_data.copy_from_slice(&self.oam[i * 4..(i + 1) * 4]);
            let sprite: Sprite = parse_sprite(raw_sprite_data);

            let mut raw_tile_data: [u8; 16] = [0; 16];
            for i in 0..raw_tile_data.len() {
                raw_tile_data[i] = self.ppu_read(pattern_table_base + sprite.pattern_table_index as u16 * 16 + i as u16);
            }
            let tile: Tile = parse_tile(raw_tile_data);

            let palette_base: u16 = 0x3f10 + 4 * sprite.palette_index as u16;
            let mut raw_palette_data: [u8; 4] = [0; 4];
            for i in 0..raw_palette_data.len() {
                raw_palette_data[i] = self.ppu_read(palette_base + i as u16);
            }
            let palette: Palette = parse_palette(raw_palette_data);

            plot_tile(canvas, tile, palette, sprite.r as usize, sprite.c as usize, sprite.h_flip, sprite.v_flip, true)
        }
    }

    fn ppu_read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..0x2400 => self.ppu_cartridge[addr as usize],
            0x2400..0x3000 => self.ppu_cartridge[(addr as usize - 0x2000) % 0x400],
            0x3000..0x3f00 => self.ppu_cartridge[addr as usize],
            0x3f00..0x4000 => self.ppu_ram[(addr % 0x20) as usize],
            0x4000..=0xffff => self.ppu_read(addr % 0x4000),
        }
    }

    fn ppu_write(&mut self, addr: u16, val: u8) {
        match addr {
            0x0000..0x2400 => self.ppu_cartridge[addr as usize] = val,
            0x2400..0x3000 => self.ppu_cartridge[(addr as usize - 0x2000) % 0x400] = val,
            0x3000..0x3f00 => self.ppu_cartridge[addr as usize] = val,
            0x3f00..0x4000 => self.ppu_ram[(addr % 0x20) as usize] = val,
            0x4000..=0xffff => self.ppu_write(addr % 0x4000, val),
        }
    }

    fn is_in_8x16_mode(&mut self) -> bool {
        (self.read(PPUCTRL) & 0b00100000) != 0
    }

    fn sprite_is_enabled(&mut self) -> bool {
        (self.read(PPUCTRL) & 0b00010000) != 0
    }

    fn background_is_enabled(&mut self) -> bool {
        (self.read(PPUCTRL) & 0b00001000) != 0
    }

    fn dump_regs(&self) {
        println!("A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X} PPUADDR: {:04X} CYC:{}", self.a, self.x, self.y, self.get_flags_byte(false), self.s, self.ppuaddr, self.cycles);
    }

    fn update_nz_flags(&mut self, val: u8) {
        self.zero = val == 0;
        self.negative = (val >> 7) != 0;
    }

    fn get_flags_byte(&self, b: bool) -> u8 {
        ((self.negative as u8) << 7) | ((self.overflow as u8) << 6) | (1u8 << 5) | ((b as u8) << 4) | ((self.decimal_mode as u8) << 3) | ((self.interrupt_disable as u8) << 2) | ((self.zero as u8) << 1) | (self.carry as u8)
    }

    fn push(&mut self, val: u8) {
        self.write((self.s as u16).wrapping_add(0x100), val);
        self.s = self.s.wrapping_sub(1);
    }

    fn push16(&mut self, val: u16) {
        self.push((val >> 8) as u8);
        self.push(val as u8);
    }

    fn pop(&mut self) -> u8 {
        self.s = self.s.wrapping_add(1);
        self.read((self.s as u16).wrapping_add(0x100))
    }

    fn pop16(&mut self) -> u16 {
        let low_bits: u8 = self.pop();
        let high_bits: u8 = self.pop();
        ((high_bits as u16) << 8) | (low_bits as u16)
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

    fn key_down(&mut self, b: usize) {
        self.buttons[b] = true;
    }

    fn key_up(&mut self, b: usize) {
        self.buttons[b] = false;
    }

    fn read(&mut self, addr: u16) -> u8 {
        // This function needs `&mut self` because reading from some memory-mapped registers can change
        // the state of the system
        match addr {
            0x0000..0x2000 => self.ram[(addr % 0x0800) as usize],
            0x2000..0x4000 => match addr % 8 {
                PPUSTATUS_I => {
                    self.w = false;
                    self.ppu_regs[(addr % 8) as usize]
                }
                PPUDATA_I => {
                    let result: u8 = self.ppudata;
                    self.ppudata = self.ppu_read(self.ppuaddr);
                    self.ppuaddr += if (self.read(PPUCTRL) & 0b100) == 0 { 1 } else { 32 };
                    result
                }
                OAMDATA_I => {
                    if self.oamdata_is_ff {
                        0xff
                    } else {
                        self.oam[self.read(OAMADDR) as usize]
                    }
                }
                _ => self.ppu_regs[(addr % 8) as usize],
            },
            0x4000..0x4018 => match addr % 0x18 {
                JOYPAD_I => {
                    let result: u8 = self.buttons[self.current_button] as u8;
                    if !self.strobe_mode {
                        self.current_button = (self.current_button + 1) % 8;
                    }
                    result
                }
                _ => self.apu_and_io_regs[(addr - 0x4000) as usize],
            },
            0x4018..0x4020 => 0,
            0x4020..=0xffff => self.cartridge[(addr - 0x4020) as usize],
        }
    }

    fn write(&mut self, addr: u16, val: u8) {
        match addr {
            0x0000..0x2000 => self.ram[(addr % 0x0800) as usize] = val,
            0x2000..0x4000 => match addr % 8 {
                OAMDATA_I => {
                    let oam_addr: u8 = self.read(OAMADDR);
                    self.oam[oam_addr as usize] = val;
                    self.write(OAMADDR, oam_addr.wrapping_add(1));
                }
                PPUADDR_I => {
                    self.ppuaddr &= if self.w { 0xff00 } else { 0x00ff };
                    self.ppuaddr |= (val as u16) << (if self.w { 0 } else { 8 });
                    self.w = !self.w;
                }
                PPUSCROLL_I => {
                    if self.w {
                        self.internal_y_scroll = val;
                    } else {
                        self.internal_x_scroll = val;
                    }
                    self.w = !self.w;
                }
                PPUDATA_I => {
                    self.ppu_write(self.ppuaddr, val);
                    self.ppuaddr += if (self.read(PPUCTRL) & 0b100) == 0 { 1 } else { 32 };
                }
                PPUCTRL_I => {
                    let interrupts_disabled: bool = !(self.read(PPUCTRL) >> 7) != 0;
                    self.ppu_regs[PPUCTRL_I as usize] = val;
                    if interrupts_disabled && ((val >> 7) != 0) {
                        self.nmi_interrupt()
                    }
                }
                _ => {
                    self.ppu_regs[(addr % 8) as usize] = val;
                }
            },
            0x4000..0x4018 => match addr % 0x18 {
                OAMDMA_I => {
                    for i in 0x00..0xff {
                        self.oam[i as usize] = self.read(((val as u16) << 8) | i);
                    }
                    self.cycles += 513 + self.cycles % 2;
                }
                JOYPAD_I => {
                    if val & 0b1 > self.strobe_mode as u8 {
                        // Entering strobe_mode
                        self.current_button = 0;
                        self.strobe_mode = true;
                    } else if val & 0b1 < self.strobe_mode as u8 {
                        // Leaving strobe_mode
                        self.strobe_mode = false;
                    }
                    self.apu_and_io_regs[(addr - 0x4000) as usize] = val & 0b111
                }
                _ => self.apu_and_io_regs[(addr - 0x4000) as usize] = val,
            },
            0x4018..0x4020 => {}
            0x4020..=0xffff => self.cartridge[(addr - 0x4020) as usize] = val,
        }
    }

    fn read16(&mut self, addr: u16) -> u16 {
        ((self.read(addr.wrapping_add(1)) as u16) << 8) | (self.read(addr) as u16)
    }

    fn get_x_scroll(&mut self) -> u16 {
        (((self.read(PPUCTRL) & 1) as u16) << 8) | (self.internal_x_scroll as u16)
    }

    fn get_y_scroll(&mut self) -> u16 {
        (((self.read(PPUCTRL) & 0b10) as u16) << 7) | (self.internal_y_scroll as u16)
    }

    fn adc(&mut self, op: u8) -> u8 {
        let result_16: u16 = (self.a as u16).wrapping_add(op as u16).wrapping_add(self.carry as u16);
        let result = result_16 as u8;

        self.carry = result_16 > 255;
        self.overflow = (is_negative(self.a) == is_negative(op)) && (is_negative(result) != is_negative(op));
        self.update_nz_flags(result);

        result
    }

    fn and(&mut self, op: u8) -> u8 {
        let result: u8 = self.a & op;
        self.update_nz_flags(result);
        result
    }

    fn asl(&mut self, op: u8) -> u8 {
        let result: u8 = op << 1;
        self.update_nz_flags(result);
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
        self.update_nz_flags(op1.wrapping_sub(op2));
    }

    fn dec(&mut self, val: u8) -> u8 {
        let result: u8 = val.wrapping_sub(1);
        self.update_nz_flags(result);
        result
    }

    fn eor(&mut self, op: u8) -> u8 {
        let result: u8 = self.a ^ op;
        self.update_nz_flags(result);
        result
    }

    fn inc(&mut self, val: u8) -> u8 {
        let result: u8 = val.wrapping_add(1);
        self.update_nz_flags(result);
        result
    }

    fn lsr(&mut self, op: u8) -> u8 {
        let result: u8 = op >> 1;
        self.update_nz_flags(result);
        self.carry = (op & 1) != 0;
        result
    }

    fn ora(&mut self, op: u8) -> u8 {
        let result: u8 = self.a | op;
        self.update_nz_flags(result);
        result
    }

    fn rol(&mut self, op: u8) -> u8 {
        let result: u8 = (op << 1) | (self.carry as u8);
        self.carry = is_negative(op);
        self.update_nz_flags(result);
        result
    }

    fn ror(&mut self, op: u8) -> u8 {
        let result: u8 = ((self.carry as u8) << 7) | (op >> 1);
        self.carry = (op & 1) != 0;
        self.update_nz_flags(result);
        result
    }

    fn sbc(&mut self, op: u8) -> u8 {
        let result_16: i16 = (self.a as i16) - (op as i16) - (!self.carry as i16);
        let result: u8 = result_16 as u8;
        self.carry = result_16 >= 0;
        self.overflow = (is_negative(result) != is_negative(self.a)) && (is_negative(result) == is_negative(op));
        self.update_nz_flags(result);
        result
    }

    fn branch(&mut self, cond: bool, op: u8) {
        self.pc = self.pc.wrapping_add(2);
        let mut new_pc = self.pc;
        if cond {
            new_pc = self.pc.wrapping_add(op as i8 as u16);
        }
        self.cycles += 2 + (cond as u64) + ((cond as u64) * ((new_pc & 0xff00 != self.pc & 0xff00) as u64));
        self.pc = new_pc;
    }

    fn nmi_interrupt(&mut self) {
        self.push16(self.pc);
        self.push(self.get_flags_byte(false));
        self.interrupt_disable = true;
        self.pc = self.read16(NMI_VECTOR);
        self.cycles += 7; // TODO: Figure out what this should be.
    }

    fn step(&mut self) {
        let old_cycles = self.cycles;

        // All 6502 instructions begin with a 1-byte opcode
        let opcode: u8 = self.read(self.pc);

        // 2-byte instruction operand
        let imm16: u16 = self.read16(self.pc.wrapping_add(1));

        // 1-byte instruction operand
        let imm8: u8 = self.read(self.pc.wrapping_add(1));

        // The addresses of the operands of all addressing modes
        let zero_page_addr: u16 = imm8 as u16;
        let zero_page_x_addr: u16 = (imm8.wrapping_add(self.x)) as u16;
        let zero_page_y_addr: u16 = (imm8.wrapping_add(self.y)) as u16;
        let absolute_addr: u16 = imm16;
        let absolute_x_addr: u16 = imm16.wrapping_add(self.x as u16);
        let absolute_y_addr: u16 = imm16.wrapping_add(self.y as u16);

        let indirect_x_addr: u16 = ((self.read((imm8.wrapping_add(self.x).wrapping_add(1)) as u16) as u16) << 8) | (self.read((imm8.wrapping_add(self.x)) as u16) as u16);

        let indirect_y_base: u16 = ((self.read((imm8.wrapping_add(1)) as u16) as u16) << 8) | self.read(imm8 as u16) as u16;
        let indirect_y_addr: u16 = indirect_y_base.wrapping_add(self.y as u16);

        let absolute_x_crossed_page: bool = absolute_x_addr & 0xff00 != imm16 & 0xff00;
        let absolute_y_crossed_page: bool = absolute_y_addr & 0xff00 != imm16 & 0xff00;
        let indirect_y_crossed_page: bool = indirect_y_addr & 0xff00 != indirect_y_base & 0xff00;

        //print!("{:04X} ", self.pc);
        //self.dump_regs();
        match opcode {
            // ADC
            0x69 => {
                self.a = self.adc(imm8);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 2;
            }
            0x65 => {
                let zero_page_arg = self.read(zero_page_addr);
                self.a = self.adc(zero_page_arg);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 3;
            }
            0x75 => {
                let zero_page_x_arg = self.read(zero_page_x_addr);
                self.a = self.adc(zero_page_x_arg);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 4;
            }
            0x6d => {
                let absolute_arg = self.read(absolute_addr);
                self.a = self.adc(absolute_arg);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 4;
            }
            0x7d => {
                let absolute_x_arg = self.read(absolute_x_addr);
                self.a = self.adc(absolute_x_arg);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 4 + (absolute_x_crossed_page as u64);
            }
            0x79 => {
                let absolute_y_arg = self.read(absolute_y_addr);
                self.a = self.adc(absolute_y_arg);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 4 + (absolute_y_crossed_page as u64);
            }
            0x61 => {
                let indirect_x_arg = self.read(indirect_x_addr);
                self.a = self.adc(indirect_x_arg);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 6;
            }
            0x71 => {
                let indirect_y_arg = self.read(indirect_y_addr);
                self.a = self.adc(indirect_y_arg);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 5 + (indirect_y_crossed_page as u64);
            }

            // AND
            0x29 => {
                self.a = self.and(imm8);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 2;
            }
            0x25 => {
                let zero_page_arg = self.read(zero_page_addr);
                self.a = self.and(zero_page_arg);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 3;
            }
            0x35 => {
                let zero_page_x_arg = self.read(zero_page_x_addr);
                self.a = self.and(zero_page_x_arg);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 4;
            }
            0x2d => {
                let absolute_arg = self.read(absolute_addr);
                self.a = self.and(absolute_arg);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 4;
            }
            0x3d => {
                let absolute_x_arg = self.read(absolute_x_addr);
                self.a = self.and(absolute_x_arg);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 4 + (absolute_x_crossed_page as u64);
            }
            0x39 => {
                let absolute_y_arg = self.read(absolute_y_addr);
                self.a = self.and(absolute_y_arg);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 4 + (absolute_y_crossed_page as u64);
            }
            0x21 => {
                let indirect_x_arg = self.read(indirect_x_addr);
                self.a = self.and(indirect_x_arg);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 6;
            }
            0x31 => {
                let indirect_y_arg = self.read(indirect_y_addr);
                self.a = self.and(indirect_y_arg);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 5 + (indirect_y_crossed_page as u64);
            }

            // ASL
            0x0a => {
                self.a = self.asl(self.a);
                self.pc = self.pc.wrapping_add(1);
                self.cycles += 2;
            }
            0x06 => {
                let zero_page_arg = self.read(zero_page_addr);
                let result: u8 = self.asl(zero_page_arg);
                self.write(zero_page_addr, result);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 5;
            }
            0x16 => {
                let zero_page_x_arg = self.read(zero_page_x_addr);
                let result: u8 = self.asl(zero_page_x_arg);
                self.write(zero_page_x_addr, result);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 6;
            }
            0x0e => {
                let absolute_arg = self.read(absolute_addr);
                let result: u8 = self.asl(absolute_arg);
                self.write(absolute_addr, result);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 6;
            }
            0x1e => {
                let absolute_x_arg = self.read(absolute_x_addr);
                let result: u8 = self.asl(absolute_x_arg);
                self.write(absolute_x_addr, result);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 7;
            }

            // BCC
            0x90 => {
                self.branch(!self.carry, imm8);
            }

            // BCS
            0xB0 => {
                self.branch(self.carry, imm8);
            }

            // BEQ
            0xF0 => {
                self.branch(self.zero, imm8);
            }

            // BIT
            0x24 => {
                let zero_page_arg = self.read(zero_page_addr);
                self.bit(zero_page_arg);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 3;
            }
            0x2c => {
                let absolute_arg = self.read(absolute_addr);
                self.bit(absolute_arg);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 4;
            }

            // BMI
            0x30 => {
                self.branch(self.negative, imm8);
            }

            // BNE
            0xd0 => {
                self.branch(!self.zero, imm8);
            }

            // BPL
            0x10 => {
                self.branch(!self.negative, imm8);
            }

            // BRK
            0x00 => {
                self.push16(self.pc.wrapping_add(2));
                self.push(self.get_flags_byte(true));
                self.pc = self.read16(BRK_VECTOR);
                self.interrupt_disable = true;
                self.cycles += 7;
            }

            // BVC
            0x50 => {
                self.branch(!self.overflow, imm8);
            }

            // BVS
            0x70 => {
                self.branch(self.overflow, imm8);
            }

            // CLC
            0x18 => {
                self.carry = false;
                self.pc = self.pc.wrapping_add(1);
                self.cycles += 2;
            }

            // CLD
            0xd8 => {
                self.decimal_mode = false;
                self.pc = self.pc.wrapping_add(1);
                self.cycles += 2;
            }

            // CLI
            0x58 => {
                self.interrupt_disable = false;
                self.pc = self.pc.wrapping_add(1);
                self.cycles += 2;
            }

            // CLV
            0xb8 => {
                self.overflow = false;
                self.pc = self.pc.wrapping_add(1);
                self.cycles += 2;
            }

            // CMP
            0xc9 => {
                self.cmp(self.a, imm8);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 2;
            }
            0xc5 => {
                let zero_page_arg = self.read(zero_page_addr);
                self.cmp(self.a, zero_page_arg);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 3;
            }
            0xd5 => {
                let zero_page_x_arg = self.read(zero_page_x_addr);
                self.cmp(self.a, zero_page_x_arg);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 4;
            }
            0xcd => {
                let absolute_arg = self.read(absolute_addr);
                self.cmp(self.a, absolute_arg);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 4;
            }
            0xdd => {
                let absolute_x_arg = self.read(absolute_x_addr);
                self.cmp(self.a, absolute_x_arg);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 4 + (absolute_x_crossed_page as u64);
            }
            0xd9 => {
                let absolute_y_arg = self.read(absolute_y_addr);
                self.cmp(self.a, absolute_y_arg);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 4 + (absolute_y_crossed_page as u64);
            }
            0xc1 => {
                let indirect_x_arg = self.read(indirect_x_addr);
                self.cmp(self.a, indirect_x_arg);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 6;
            }
            0xd1 => {
                let indirect_y_arg = self.read(indirect_y_addr);
                self.cmp(self.a, indirect_y_arg);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 5 + (indirect_y_crossed_page as u64);
            }

            // CPX
            0xe0 => {
                self.cmp(self.x, imm8);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 2;
            }
            0xe4 => {
                let zero_page_arg = self.read(zero_page_addr);
                self.cmp(self.x, zero_page_arg);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 3;
            }
            0xec => {
                let absolute_arg = self.read(absolute_addr);
                self.cmp(self.x, absolute_arg);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 4;
            }

            // CPY
            0xc0 => {
                self.cmp(self.y, imm8);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 2;
            }
            0xc4 => {
                let zero_page_arg = self.read(zero_page_addr);
                self.cmp(self.y, zero_page_arg);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 3;
            }
            0xcc => {
                let absolute_arg = self.read(absolute_addr);
                self.cmp(self.y, absolute_arg);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 4;
            }

            // DEC
            0xc6 => {
                let zero_page_arg = self.read(zero_page_addr);
                let result: u8 = self.dec(zero_page_arg);
                self.write(zero_page_addr, result);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 5;
            }
            0xd6 => {
                let zero_page_x_arg = self.read(zero_page_x_addr);
                let result: u8 = self.dec(zero_page_x_arg);
                self.write(zero_page_x_addr, result);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 6;
            }
            0xce => {
                let absolute_arg = self.read(absolute_addr);
                let result: u8 = self.dec(absolute_arg);
                self.write(absolute_addr, result);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 6;
            }
            0xde => {
                let absolute_x_arg = self.read(absolute_x_addr);
                let result: u8 = self.dec(absolute_x_arg);
                self.write(absolute_x_addr, result);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 7;
            }

            // DEX
            0xca => {
                self.x = self.dec(self.x);
                self.pc = self.pc.wrapping_add(1);
                self.cycles += 2;
            }

            // DEY
            0x88 => {
                self.y = self.dec(self.y);
                self.pc = self.pc.wrapping_add(1);
                self.cycles += 2;
            }

            // EOR
            0x49 => {
                self.a = self.eor(imm8);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 2;
            }
            0x45 => {
                let zero_page_arg = self.read(zero_page_addr);
                self.a = self.eor(zero_page_arg);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 3;
            }
            0x55 => {
                let zero_page_x_arg = self.read(zero_page_x_addr);
                self.a = self.eor(zero_page_x_arg);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 4;
            }
            0x4d => {
                let absolute_arg = self.read(absolute_addr);
                self.a = self.eor(absolute_arg);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 4;
            }
            0x5d => {
                let absolute_x_arg = self.read(absolute_x_addr);
                self.a = self.eor(absolute_x_arg);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 4 + (absolute_x_crossed_page as u64);
            }
            0x59 => {
                let absolute_y_arg = self.read(absolute_y_addr);
                self.a = self.eor(absolute_y_arg);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 4 + (absolute_y_crossed_page as u64);
            }
            0x41 => {
                let indirect_x_arg = self.read(indirect_x_addr);
                self.a = self.eor(indirect_x_arg);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 6;
            }
            0x51 => {
                let indirect_y_arg = self.read(indirect_y_addr);
                self.a = self.eor(indirect_y_arg);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 5 + (indirect_y_crossed_page as u64);
            }

            // INC
            0xe6 => {
                let zero_page_arg = self.read(zero_page_addr);
                let result: u8 = self.inc(zero_page_arg);
                self.write(zero_page_addr, result);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 5;
            }
            0xf6 => {
                let zero_page_x_arg = self.read(zero_page_x_addr);
                let result: u8 = self.inc(zero_page_x_arg);
                self.write(zero_page_x_addr, result);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 6;
            }
            0xee => {
                let absolute_arg = self.read(absolute_addr);
                let result: u8 = self.inc(absolute_arg);
                self.write(absolute_addr, result);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 6;
            }
            0xfe => {
                let absolute_x_arg = self.read(absolute_x_addr);
                let result: u8 = self.inc(absolute_x_arg);
                self.write(absolute_x_addr, result);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 7;
            }

            // INX
            0xe8 => {
                self.x = self.inc(self.x);
                self.pc = self.pc.wrapping_add(1);
                self.cycles += 2;
            }

            // INY
            0xc8 => {
                self.y = self.inc(self.y);
                self.pc = self.pc.wrapping_add(1);
                self.cycles += 2;
            }

            // JMP
            0x4c => {
                self.pc = absolute_addr;
                self.cycles += 3;
            }
            0x6c => {
                let indirect_addr: u16 = ((self.read((absolute_addr & 0xff00) | ((absolute_addr as u8).wrapping_add(1) as u16)) as u16) << 8) | (self.read(absolute_addr) as u16);
                self.pc = indirect_addr;
                self.cycles += 5;
            }

            // JSR
            0x20 => {
                self.push16(self.pc.wrapping_add(2));
                self.pc = absolute_addr;
                self.cycles += 6;
            }

            // LDA
            0xa9 => {
                self.a = imm8;
                self.update_nz_flags(self.a);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 2;
            }
            0xa5 => {
                let zero_page_arg = self.read(zero_page_addr);
                self.a = zero_page_arg;
                self.update_nz_flags(self.a);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 3;
            }
            0xb5 => {
                let zero_page_x_arg = self.read(zero_page_x_addr);
                self.a = zero_page_x_arg;
                self.update_nz_flags(self.a);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 4;
            }
            0xad => {
                let absolute_arg = self.read(absolute_addr);
                self.a = absolute_arg;
                self.update_nz_flags(self.a);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 4;
            }
            0xbd => {
                let absolute_x_arg = self.read(absolute_x_addr);
                self.a = absolute_x_arg;
                self.update_nz_flags(self.a);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 4 + (absolute_x_crossed_page as u64);
            }
            0xb9 => {
                let absolute_y_arg = self.read(absolute_y_addr);
                self.a = absolute_y_arg;
                self.update_nz_flags(self.a);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 4 + (absolute_y_crossed_page as u64);
            }
            0xa1 => {
                let indirect_x_arg = self.read(indirect_x_addr);
                self.a = indirect_x_arg;
                self.update_nz_flags(self.a);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 6;
            }
            0xb1 => {
                let indirect_y_arg = self.read(indirect_y_addr);
                self.a = indirect_y_arg;
                self.update_nz_flags(self.a);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 5 + (indirect_y_crossed_page as u64);
            }

            // LDX
            0xa2 => {
                self.x = imm8;
                self.update_nz_flags(self.x);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 2;
            }
            0xa6 => {
                let zero_page_arg = self.read(zero_page_addr);
                self.x = zero_page_arg;
                self.update_nz_flags(self.x);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 3;
            }
            0xb6 => {
                let zero_page_y_arg = self.read(zero_page_y_addr);
                self.x = zero_page_y_arg;
                self.update_nz_flags(self.x);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 4;
            }
            0xae => {
                let absolute_arg = self.read(absolute_addr);
                self.x = absolute_arg;
                self.update_nz_flags(self.x);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 4;
            }
            0xbe => {
                let absolute_y_arg = self.read(absolute_y_addr);
                self.x = absolute_y_arg;
                self.update_nz_flags(self.x);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 4 + (absolute_y_crossed_page as u64);
            }

            // LDY
            0xa0 => {
                self.y = imm8;
                self.update_nz_flags(self.y);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 2;
            }
            0xa4 => {
                let zero_page_arg = self.read(zero_page_addr);
                self.y = zero_page_arg;
                self.update_nz_flags(self.y);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 3;
            }
            0xb4 => {
                let zero_page_x_arg = self.read(zero_page_x_addr);
                self.y = zero_page_x_arg;
                self.update_nz_flags(self.y);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 4;
            }
            0xac => {
                let absolute_arg = self.read(absolute_addr);
                self.y = absolute_arg;
                self.update_nz_flags(self.y);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 4;
            }
            0xbc => {
                let absolute_x_arg = self.read(absolute_x_addr);
                self.y = absolute_x_arg;
                self.update_nz_flags(self.y);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 4 + (absolute_x_crossed_page as u64);
            }

            // LSR
            0x4a => {
                self.a = self.lsr(self.a);
                self.pc = self.pc.wrapping_add(1);
                self.cycles += 2;
            }
            0x46 => {
                let zero_page_arg = self.read(zero_page_addr);
                let result: u8 = self.lsr(zero_page_arg);
                self.write(zero_page_addr, result);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 5;
            }
            0x56 => {
                let zero_page_x_arg = self.read(zero_page_x_addr);
                let result: u8 = self.lsr(zero_page_x_arg);
                self.write(zero_page_x_addr, result);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 6;
            }
            0x4e => {
                let absolute_arg = self.read(absolute_addr);
                let result: u8 = self.lsr(absolute_arg);
                self.write(absolute_addr, result);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 6;
            }
            0x5e => {
                let absolute_x_arg = self.read(absolute_x_addr);
                let result: u8 = self.lsr(absolute_x_arg);
                self.write(absolute_x_addr, result);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 7;
            }

            // NOP
            0xea => {
                self.pc = self.pc.wrapping_add(1);
                self.cycles += 2;
            }

            // ORA
            0x09 => {
                self.a = self.ora(imm8);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 2;
            }
            0x05 => {
                let zero_page_arg = self.read(zero_page_addr);
                self.a = self.ora(zero_page_arg);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 3;
            }
            0x15 => {
                let zero_page_x_arg = self.read(zero_page_x_addr);
                self.a = self.ora(zero_page_x_arg);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 4;
            }
            0x0d => {
                let absolute_arg = self.read(absolute_addr);
                self.a = self.ora(absolute_arg);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 4;
            }
            0x1d => {
                let absolute_x_arg = self.read(absolute_x_addr);
                self.a = self.ora(absolute_x_arg);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 4 + (absolute_x_crossed_page as u64);
            }
            0x19 => {
                let absolute_y_arg = self.read(absolute_y_addr);
                self.a = self.ora(absolute_y_arg);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 4 + (absolute_y_crossed_page as u64);
            }
            0x01 => {
                let indirect_x_arg = self.read(indirect_x_addr);
                self.a = self.ora(indirect_x_arg);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 6;
            }
            0x11 => {
                let indirect_y_arg = self.read(indirect_y_addr);
                self.a = self.ora(indirect_y_arg);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 5 + (indirect_y_crossed_page as u64);
            }

            // PHA
            0x48 => {
                self.push(self.a);
                self.pc = self.pc.wrapping_add(1);
                self.cycles += 3;
            }

            // PHP
            0x08 => {
                self.push(self.get_flags_byte(true));
                self.pc = self.pc.wrapping_add(1);
                self.cycles += 3;
            }

            // PLA
            0x68 => {
                self.a = self.pop();
                self.update_nz_flags(self.a);
                self.pc = self.pc.wrapping_add(1);
                self.cycles += 4;
            }

            // PLP
            0x28 => {
                self.pop_flags();
                self.pc = self.pc.wrapping_add(1);
                self.cycles += 4;
            }

            // ROL
            0x2a => {
                self.a = self.rol(self.a);
                self.pc = self.pc.wrapping_add(1);
                self.cycles += 2;
            }
            0x26 => {
                let zero_page_arg = self.read(zero_page_addr);
                let result: u8 = self.rol(zero_page_arg);
                self.write(zero_page_addr, result);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 5;
            }
            0x36 => {
                let zero_page_x_arg = self.read(zero_page_x_addr);
                let result: u8 = self.rol(zero_page_x_arg);
                self.write(zero_page_x_addr, result);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 6;
            }
            0x2e => {
                let absolute_arg = self.read(absolute_addr);
                let result: u8 = self.rol(absolute_arg);
                self.write(absolute_addr, result);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 6;
            }
            0x3e => {
                let absolute_x_arg = self.read(absolute_x_addr);
                let result: u8 = self.rol(absolute_x_arg);
                self.write(absolute_x_addr, result);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 7;
            }

            // ROR
            0x6a => {
                self.a = self.ror(self.a);
                self.pc = self.pc.wrapping_add(1);
                self.cycles += 2;
            }
            0x66 => {
                let zero_page_arg = self.read(zero_page_addr);
                let result: u8 = self.ror(zero_page_arg);
                self.write(zero_page_addr, result);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 5;
            }
            0x76 => {
                let zero_page_x_arg = self.read(zero_page_x_addr);
                let result: u8 = self.ror(zero_page_x_arg);
                self.write(zero_page_x_addr, result);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 6;
            }
            0x6e => {
                let absolute_arg = self.read(absolute_addr);
                let result: u8 = self.ror(absolute_arg);
                self.write(absolute_addr, result);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 6;
            }
            0x7e => {
                let absolute_x_arg = self.read(absolute_x_addr);
                let result: u8 = self.ror(absolute_x_arg);
                self.write(absolute_x_addr, result);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 7;
            }

            // RTI
            0x40 => {
                self.pop_flags();
                self.pc = self.pop16();
                self.cycles += 6;
            }

            // RTS
            0x60 => {
                self.pc = self.pop16().wrapping_add(1);
                self.cycles += 6;
            }

            // SBC
            0xe9 => {
                self.a = self.sbc(imm8);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 2;
            }
            0xe5 => {
                let zero_page_arg = self.read(zero_page_addr);
                self.a = self.sbc(zero_page_arg);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 3;
            }
            0xf5 => {
                let zero_page_x_arg = self.read(zero_page_x_addr);
                self.a = self.sbc(zero_page_x_arg);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 4;
            }
            0xed => {
                let absolute_arg = self.read(absolute_addr);
                self.a = self.sbc(absolute_arg);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 4;
            }
            0xfd => {
                let absolute_x_arg = self.read(absolute_x_addr);
                self.a = self.sbc(absolute_x_arg);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 4 + (absolute_x_crossed_page as u64);
            }
            0xf9 => {
                let absolute_y_arg = self.read(absolute_y_addr);
                self.a = self.sbc(absolute_y_arg);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 4 + (absolute_y_crossed_page as u64);
            }
            0xe1 => {
                let indirect_x_arg = self.read(indirect_x_addr);
                self.a = self.sbc(indirect_x_arg);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 6;
            }
            0xf1 => {
                let indirect_y_arg = self.read(indirect_y_addr);
                self.a = self.sbc(indirect_y_arg);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 5 + (indirect_y_crossed_page as u64);
            }

            // SEC
            0x38 => {
                self.carry = true;
                self.pc = self.pc.wrapping_add(1);
                self.cycles += 2;
            }

            // SED
            0xf8 => {
                self.decimal_mode = true;
                self.pc = self.pc.wrapping_add(1);
                self.cycles += 2;
            }

            // SEI
            0x78 => {
                self.interrupt_disable = true;
                self.pc = self.pc.wrapping_add(1);
                self.cycles += 2;
            }

            // STA
            0x85 => {
                self.write(zero_page_addr, self.a);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 3;
            }
            0x95 => {
                self.write(zero_page_x_addr, self.a);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 4;
            }
            0x8d => {
                self.write(absolute_addr, self.a);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 4;
            }
            0x9d => {
                self.write(absolute_x_addr, self.a);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 5;
            }
            0x99 => {
                self.write(absolute_y_addr, self.a);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 5;
            }
            0x81 => {
                self.write(indirect_x_addr, self.a);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 6;
            }
            0x91 => {
                self.write(indirect_y_addr, self.a);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 6;
            }

            // STX
            0x86 => {
                self.write(zero_page_addr, self.x);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 3;
            }
            0x96 => {
                self.write(zero_page_y_addr, self.x);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 4;
            }
            0x8e => {
                self.write(absolute_addr, self.x);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 4;
            }

            // STY
            0x84 => {
                self.write(zero_page_addr, self.y);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 3;
            }
            0x94 => {
                self.write(zero_page_x_addr, self.y);
                self.pc = self.pc.wrapping_add(2);
                self.cycles += 4;
            }
            0x8c => {
                self.write(absolute_addr, self.y);
                self.pc = self.pc.wrapping_add(3);
                self.cycles += 4;
            }

            // TAX
            0xaa => {
                self.x = self.a;
                self.update_nz_flags(self.x);
                self.pc = self.pc.wrapping_add(1);
                self.cycles += 2;
            }

            // TAY
            0xa8 => {
                self.y = self.a;
                self.update_nz_flags(self.y);
                self.pc = self.pc.wrapping_add(1);
                self.cycles += 2;
            }

            // TSX
            0xba => {
                self.x = self.s;
                self.update_nz_flags(self.x);
                self.pc = self.pc.wrapping_add(1);
                self.cycles += 2;
            }
            // TXA
            0x8a => {
                self.a = self.x;
                self.update_nz_flags(self.a);
                self.pc = self.pc.wrapping_add(1);
                self.cycles += 2;
            }

            // TXS
            0x9a => {
                self.s = self.x;
                self.pc = self.pc.wrapping_add(1);
                self.cycles += 2;
            }

            // TYA
            0x98 => {
                self.a = self.y;
                self.update_nz_flags(self.a);
                self.pc = self.pc.wrapping_add(1);
                self.cycles += 2;
            }

            _ => {
                self.pc += 1;
                panic!("Invalid opcode: 0x{:02x}", opcode);
            }
        }

        let cycles_elapsed = self.cycles - old_cycles;
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

    let mut nes = Nes::new(&mut rom_file);

    let sdl_context = sdl2::init().expect("Couldn't initialize SDL2");
    let video_subsystem = sdl_context.video().expect("Couldn't initialize video subsystem");

    let window = video_subsystem.window("nespump", 256, 240).position_centered().build().expect("Couldn't build window");

    let mut canvas: Canvas<Window> = window.into_canvas().build().expect("Couldn't build canvas");
    canvas.clear();
    canvas.present();
    let mut event_pump = sdl_context.event_pump().expect("Couldn't make event pump");

    let mut steps: u64 = 0;

    let mut paused: bool = false;

    'gameloop: loop {
        match event_pump.poll_event() {
            Some(Event::Quit { .. }) => break 'gameloop,
            Some(Event::KeyUp { keycode: Some(Keycode::Up), .. }) => nes.key_up(4),
            Some(Event::KeyUp { keycode: Some(Keycode::Down), .. }) => nes.key_up(5),
            Some(Event::KeyUp { keycode: Some(Keycode::Left), .. }) => nes.key_up(6),
            Some(Event::KeyUp { keycode: Some(Keycode::Right), .. }) => nes.key_up(7),
            Some(Event::KeyUp { keycode: Some(Keycode::A), .. }) => nes.key_up(0),
            Some(Event::KeyUp { keycode: Some(Keycode::B), .. }) => nes.key_up(1),
            Some(Event::KeyUp { keycode: Some(Keycode::LShift), .. }) => nes.key_up(3),
            Some(Event::KeyUp { keycode: Some(Keycode::RShift), .. }) => nes.key_up(2),

            Some(Event::KeyDown { keycode: Some(Keycode::Up), .. }) => nes.key_down(4),
            Some(Event::KeyDown { keycode: Some(Keycode::Down), .. }) => nes.key_down(5),
            Some(Event::KeyDown { keycode: Some(Keycode::Left), .. }) => nes.key_down(6),
            Some(Event::KeyDown { keycode: Some(Keycode::Right), .. }) => nes.key_down(7),
            Some(Event::KeyDown { keycode: Some(Keycode::A), .. }) => nes.key_down(0),
            Some(Event::KeyDown { keycode: Some(Keycode::B), .. }) => nes.key_down(1),
            Some(Event::KeyDown { keycode: Some(Keycode::LShift), .. }) => nes.key_down(3),
            Some(Event::KeyDown { keycode: Some(Keycode::RShift), .. }) => nes.key_down(2),
            Some(Event::KeyDown { keycode: Some(Keycode::Space), .. }) => paused = !paused,
            Some(Event::KeyDown { keycode: Some(Keycode::Q), .. }) => break 'gameloop,

            _ => {}
        }
        if !paused {
            nes.step();
            steps += 1;
            if steps % 8192 == 0 {
                // nes.render_pattern_table(&mut canvas);
                nes.render_bg(&mut canvas);
                nes.render_sprites(&mut canvas);
                canvas.present();
            }
        }
    }
}
