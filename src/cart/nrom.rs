use cart::Mapper;

const PRG_ROM_PAGE_SIZE: u16 = 1_6384;
const CHR_ROM_PAGE_SIZE: u16 = 8192;
const PRG_RAM_PAGE_SIZE: u16 = 8192;

pub struct NROM {
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    prg_ram: Vec<u8>,
    prg_num: u8,
}

impl NROM {
    pub fn new(d: &[u8]) -> NROM {
        let prg_num = d[4];
        let prg_size = prg_num as u16 * PRG_ROM_PAGE_SIZE;
        let chr_size = d[5] as u16 * CHR_ROM_PAGE_SIZE;
        let ram_size = if d[8] == 0 {
            PRG_RAM_PAGE_SIZE
        } else {
            d[8] as u16 * PRG_RAM_PAGE_SIZE
        };

        NROM {
            prg_rom: d[16..(16 + prg_size) as usize].to_vec(),
            chr_rom: d[(16 + prg_size) as usize..(16 + prg_size + chr_size) as usize].to_vec(),
            prg_ram: vec![0; ram_size as usize],
            prg_num: prg_num,
        }
    }
}

impl Mapper for NROM {
    fn cpu_read(&self, a: u16) -> u8 {
        match a {
            0x6000...0x7FFF => self.prg_ram[a as usize % 0x6000],
            0x8000...0xBFFF => self.prg_rom[a as usize % 0x8000],
            0xC000...0xFFFF => match self.prg_num {
                2 => self.prg_rom[a as usize % 0x8000],
                _ => self.prg_rom[a as usize % 0xC000],
            },
            _ => 0,
        }
    }

    fn cpu_write(&mut self, a: u16, v: u8) {
        match a {
            0x6000...0x7FFF => self.prg_ram[a as usize % 0x6000] = v,
            0x8000...0xBFFF => self.prg_rom[a as usize % 0x8000] = v,
            0xC000...0xFFFF => match self.prg_num {
                2 => self.prg_rom[a as usize % 0x8000] = v,
                _ => self.prg_rom[a as usize % 0xC000] = v,
            },
            _ => (),
        }
    }

    fn ppu_read(&self, _: u16) -> u8 {
        0
    }

    fn ppu_write(&mut self, _: u16, _: u8) {}
    fn cycle(&mut self) {}
}
