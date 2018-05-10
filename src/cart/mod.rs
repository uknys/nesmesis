pub mod nrom;

pub trait Mapper {
    fn cpu_read(&self, a: u16) -> u8;
    fn cpu_write(&mut self, a: u16, v: u8);
    fn ppu_read(&self, a: u16) -> u8;
    fn ppu_write(&mut self, a: u16, v: u8);
    fn cycle(&mut self);
}
