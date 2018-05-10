#[derive(Clone, Copy)]
pub enum Register {
    A,
    X,
    Y,
    P,
    SP,
}

#[derive(Clone, Copy)]
pub enum Flag {
    Overflow,
    Negative,
    Zero,
    Carry,
    Decimal,
    Interrupt,
}

bitflags!{
    pub struct ProcessorStatus: u8 {
        const NEGATIVE      = 0b1000_0000;
        const OVERFLOW      = 0b0100_0000;
        const ALWAYS_ONE    = 0b0010_0000;
        const BREAK         = 0b0001_0000;
        const DECIMAL       = 0b0000_1000;
        const INTERRUPT     = 0b0000_0100;
        const ZERO          = 0b0000_0010;
        const CARRY         = 0b0000_0001;
    }
}

pub struct Registers {
    a: u8,
    x: u8,
    y: u8,
    p: ProcessorStatus,
    sp: u8,
    pc: u16,
}

impl Registers {
    pub fn new() -> Registers {
        Registers {
            a: 0,
            x: 0,
            y: 0,
            p: ProcessorStatus::empty(),
            sp: 0,
            pc: 0,
        }
    }

    // #region Read / Write
    pub fn read(&self, a: Register) -> u8 {
        use self::Register::*;
        match a {
            A => self.a,
            P => self.p.bits(),
            X => self.x,
            Y => self.y,
            SP => self.sp,
        }
    }

    pub fn write(&mut self, a: Register, v: u8) {
        use self::Register::*;
        match a {
            A => {
                self.update_zn(v);
                self.a = v
            }
            X => {
                self.update_zn(v);
                self.x = v
            }
            Y => {
                self.update_zn(v);
                self.y = v
            }
            P => self.p = ProcessorStatus::from_bits(v & 0xCF | 0x20).unwrap(),
            SP => self.sp = v,
        }
    }

    pub fn read_pc(&self) -> u16 {
        self.pc
    }

    pub fn write_pc(&mut self, v: u16) {
        self.pc = v
    }
    // #endregion

    // #region Flags
    pub fn check_flag(&self, f: Flag) -> bool {
        use self::Flag::*;
        match f {
            Interrupt => self.p.contains(ProcessorStatus::INTERRUPT),
            Overflow => self.p.contains(ProcessorStatus::OVERFLOW),
            Negative => self.p.contains(ProcessorStatus::NEGATIVE),
            Decimal => self.p.contains(ProcessorStatus::DECIMAL),
            Carry => self.p.contains(ProcessorStatus::CARRY),
            Zero => self.p.contains(ProcessorStatus::ZERO),
        }
    }

    pub fn update_flag(&mut self, f: Flag, v: bool) {
        use self::Flag::*;
        match (f, v) {
            (Interrupt, false) => self.p.remove(ProcessorStatus::INTERRUPT),
            (Interrupt, true) => self.p.insert(ProcessorStatus::INTERRUPT),
            (Overflow, false) => self.p.remove(ProcessorStatus::OVERFLOW),
            (Overflow, true) => self.p.insert(ProcessorStatus::OVERFLOW),
            (Negative, false) => self.p.remove(ProcessorStatus::NEGATIVE),
            (Negative, true) => self.p.insert(ProcessorStatus::NEGATIVE),
            (Decimal, false) => self.p.remove(ProcessorStatus::DECIMAL),
            (Decimal, true) => self.p.insert(ProcessorStatus::DECIMAL),
            (Carry, false) => self.p.remove(ProcessorStatus::CARRY),
            (Carry, true) => self.p.insert(ProcessorStatus::CARRY),
            (Zero, false) => self.p.remove(ProcessorStatus::ZERO),
            (Zero, true) => self.p.insert(ProcessorStatus::ZERO),
        }
    }

    pub fn update_zn(&mut self, v: u8) {
        if v == 0 {
            self.p.insert(ProcessorStatus::ZERO)
        } else {
            self.p.remove(ProcessorStatus::ZERO)
        }

        if v & 0x80 == 0x80 {
            self.p.insert(ProcessorStatus::NEGATIVE)
        } else {
            self.p.remove(ProcessorStatus::NEGATIVE)
        }
    }

    pub fn update_cv(&mut self, x: u8, y: u8, r: u16) {
        if r > 0xFF {
            self.p.insert(ProcessorStatus::CARRY) 
        } else {
            self.p.remove(ProcessorStatus::CARRY)
        }

        if u16::from(!(x ^ y)) & (u16::from(x) ^ r) & 0x80 != 0 {
            self.p.insert(ProcessorStatus::OVERFLOW)
        } else {
            self.p.remove(ProcessorStatus::OVERFLOW)
        }
    }
    // #endregion
}

impl Default for Registers {
    fn default() -> Self {
        Self::new()
    }
}
