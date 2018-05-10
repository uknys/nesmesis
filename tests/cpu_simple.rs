extern crate nesmesis;

use nesmesis::cpu::CPU;
use nesmesis::cpu::reg::Register;
use nesmesis::cart::nrom::NROM;
use nesmesis::cart::Mapper;
use nesmesis::MMU;

use std::io::BufReader;
use std::io::BufRead;

// #region TestMemory Struct
pub struct TestMemory {
    ram: [u8; 0x800],
    rom: NROM,
}

impl TestMemory {
    pub fn new(d: &[u8]) -> TestMemory {
        TestMemory {
            ram: [0; 0x800],
            rom: NROM::new(d),
        }
    }
}

impl MMU for TestMemory {
    fn read(&self, a: u16) -> u8 {
        match a {
            0x0000...0x1FFF => self.ram[a as usize % 0x800],
            _ => self.rom.cpu_read(a),
        }
    }

    fn write(&mut self, a: u16, v: u8) {
        match a {
            0x0000...0x1FFF => self.ram[a as usize % 0x800] = v,
            _ => self.rom.cpu_write(a, v),
        }
    }

    fn cycle(&mut self) {}
}
// #endregion

// #region NESTEST
const ROM: &[u8] = include_bytes!("nestest/nestest.nes");
const LOG: &[u8] = include_bytes!("nestest/log");

#[test]
fn cpu_nestest() {
    use self::Register::*;

    let mut r = TestMemory::new(ROM);
    let mut c = CPU::new(&mut r);
    c.init();
    c.reg.write_pc(0xC000);

    let r = BufReader::new(LOG);
    for line in r.lines() {
        assert_eq!(
            line.unwrap(),
            format!(
                "{:04X} A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X}",
                c.reg.read_pc(),
                c.reg.read(A),
                c.reg.read(X),
                c.reg.read(Y),
                c.reg.read(P),
                c.reg.read(SP)
            )
        );
        c.execute();
    }
}
// #endregion

// #region Single Instructions Tests
const INSTRUCTIONS_SINGLES: [(&[u8], &'static str); 0x10] =
    [
        (include_bytes!("ins/01-basics.nes"), "01-basics"),
        (include_bytes!("ins/02-implied.nes"), "02-implied"),
        (include_bytes!("ins/03-immediate.nes"), "03-immediate"),
        (include_bytes!("ins/04-zero_page.nes"), "04-zero_page"),
        (include_bytes!("ins/05-zp_xy.nes"), "05-zp_xy"),
        (include_bytes!("ins/06-absolute.nes"), "06-absolute"),
        (include_bytes!("ins/07-abs_xy.nes"), "07-abs_xy"),
        (include_bytes!("ins/08-ind_x.nes"), "08-ind_x"),
        (include_bytes!("ins/09-ind_y.nes"), "09-ind_y"),
        (include_bytes!("ins/10-branches.nes"), "10-branches"),
        (include_bytes!("ins/11-stack.nes"), "11-stack"),
        (include_bytes!("ins/12-jmp_jsr.nes"), "12-jmp_jsr"),
        (include_bytes!("ins/13-rts.nes"), "13-rts"),
        (include_bytes!("ins/14-rti.nes"), "14-rti"),
        (include_bytes!("ins/15-brk.nes"), "15-brk"),
        (include_bytes!("ins/16-special.nes"), "16-special"),
    ];

fn cpu_instruction_test(x: &[u8], s: &str) -> String {
    let mut a = TestMemory::new(x);
    let mut c = CPU::new(&mut a);
    c.init();
    
    loop {
        c.execute();
        let mut x = 0u8;
        if c.bus.read(0x6001) == 0xDE && c.bus.read(0x6002) == 0xB0 && c.bus.read(0x6003) == 0x61 {
            let mut vec = vec![];

            loop {
                match c.bus.read(0x6004 + x as u16) {
                    10 if !vec.is_empty() => vec.push(0x20),
                    10 if vec.is_empty() => (),
                    0 => break,
                    a => vec.push(a),
                }

                x += 1;
            }

            if x == (s.len() + 9) as u8 {
                return String::from_utf8(vec).unwrap();
            }
        }
    }
}

#[test]
fn cpu_instructions_test() {
    for a in &INSTRUCTIONS_SINGLES {
        let e = format!("{}  Passed", a.1);
        assert_eq!(cpu_instruction_test(a.0, a.1), e);
    }
}
// #endregion