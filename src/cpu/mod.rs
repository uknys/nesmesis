pub mod ops;
pub mod reg;

use MMU;
use cpu::ops::{AddressingMode, Operation};
use cpu::reg::{Flag, Register, Registers};

const NMI_VECTOR: u16 = 0xFFFA;
const RESET_VECTOR: u16 = 0xFFFC;
const IRQ_VECTOR: u16 = 0xFFFE;

pub struct CPU<'a> {
    pub reg: Registers,
    pub bus: &'a mut MMU,
    nmi: bool,
}

impl<'a> CPU<'a> {
    pub fn new(bus: &'a mut MMU) -> CPU {
        CPU {
            reg: Registers::default(),
            bus,
            nmi: false,
        }
    }

    fn cross(a: u16, b: u8) -> bool {
        ((a.wrapping_add(u16::from(b))) & 0xFF00) != (a & 0xFF00)
    }

    // #region Execution
    pub fn init(&mut self) {
        let reset = self.read16(RESET_VECTOR);
        self.reg.write_pc(reset);
        self.reg.write(Register::SP, 0xFD);
        self.reg.write(Register::P, 0x24);
    }

    pub fn execute(&mut self) -> Result<(), String> {
        let p = self.imm();
        let ins: Operation = self.read(p).into();
        use self::Operation::*;

        match ins {
            Load(r, m) => Ok(self.load(r, m)),
            Store(r, m) => Ok(self.store(r, m)),
            Transfer(r1, r2) => Ok(self.transfer(r1, r2)),
            Add(m) => Ok(self.add(m)),
            Inc(Some(r), _) => Ok(self.inc_r(r)),
            Inc(_, Some(m)) => Ok(self.inc_m(m)),
            Dec(Some(r), _) => Ok(self.dec_r(r)),
            Dec(_, Some(m)) => Ok(self.dec_m(m)),
            Sub(m) => Ok(self.sub(m)),
            And(m) => Ok(self.and(m)),
            Asl(None) => Ok(self.asl_a()),
            Asl(Some(m)) => Ok(self.asl(m)),
            Bits(m) => Ok(self.bits(m)),
            Xor(m) => Ok(self.xor(m)),
            Lsr(None) => Ok(self.lsr_a()),
            Lsr(Some(m)) => Ok(self.lsr(m)),
            Or(m) => Ok(self.or(m)),
            Rol(None) => Ok(self.rol_a()),
            Rol(Some(m)) => Ok(self.rol(m)),
            Ror(None) => Ok(self.ror_a()),
            Ror(Some(m)) => Ok(self.ror(m)),
            Branch(f, b) => Ok(self.branch(f, b)),
            Jump(None) => Ok(self.jsr()),
            Jump(Some(m)) => Ok(self.jump(m)),
            Ret(true) => Ok(self.rts()),
            Ret(false) => Ok(self.rti()),
            Flag(f, b) => Ok(self.flag(f, b)),
            Compare(r, m) => Ok(self.compare(r, m)),
            Stack(r, b) => Ok(self.stack(r, b)),
            Break => Ok(self.brk()),
            Nop(m) => Ok(self.nop(m)),
            Lax(m) => Ok(self.lax(m)),
            Sax(m) => Ok(self.sax(m)),
            Dcp(m) => Ok(self.dcp(m)),
            Isb(m) => Ok(self.isb(m)),
            Slo(m) => Ok(self.slo(m)),
            Rla(m) => Ok(self.rla(m)),
            Sre(m) => Ok(self.sre(m)),
            Rra(m) => Ok(self.rra(m)),
            Aac(m) => Ok(self.aac(m)),
            Asr(m) => Ok(self.asr(m)),
            Arr(m) => Ok(self.arr(m)),
            Atx(m) => Ok(self.atx(m)),
            Axs(m) => Ok(self.axs(m)),
            Sa(r, m) => Ok(self.sa(r, m)),
            _ => Err(format!("Bad Instruction {:02X}", p)),
        }
    }
    // #endregion

    // #region Read / Write
    fn read(&mut self, a: u16) -> u8 {
        self.bus.cycle();
        self.bus.read(a)
    }

    fn write(&mut self, a: u16, v: u8) {
        self.bus.cycle();
        self.bus.write(a, v)
    }

    fn read16(&mut self, a: u16) -> u16 {
        u16::from(self.read(a)) | (u16::from(self.read(a + 1)) << 8)
    }

    fn write16(&mut self, a: u16, v: u16) {
        self.write(a, (v >> 8) as u8);
        self.write(a + 1, (v & 0xFF) as u8);
    }
    // #endregion

    // #region Stack
    fn push(&mut self, v: u8) {
        let sp = self.reg.read(Register::SP);
        self.write(u16::from(sp) + 0x100, v);
        self.reg.write(Register::SP, sp.wrapping_sub(1));
    }

    fn push16(&mut self, v: u16) {
        self.push((v >> 8) as u8);
        self.push((v & 0xFF) as u8)
    }

    fn pop(&mut self) -> u8 {
        let sp = self.reg.read(Register::SP).wrapping_add(1);
        self.reg.write(Register::SP, sp);
        self.read(u16::from(sp) + 0x100)
    }

    fn pop16(&mut self) -> u16 {
        u16::from(self.pop()) | (u16::from(self.pop()) << 8)
    }
    // #endregion

    // #region Addressing Modes
    fn imm(&mut self) -> u16 {
        let p = self.reg.read_pc();
        self.reg.write_pc(p.wrapping_add(1));
        p
    }

    fn imm16(&mut self) -> u16 {
        let p = self.reg.read_pc();
        self.reg.write_pc(p.wrapping_add(2));
        p
    }

    fn abs(&mut self) -> u16 {
        let imm = self.imm16();
        self.read16(imm)
    }

    fn abi(&mut self, extra: bool, r: Register) -> u16 {
        let a = self.abs();
        let reg = self.reg.read(r);

        if extra && CPU::cross(a, reg) {
            self.bus.cycle();
        }

        a.wrapping_add(u16::from(reg))
    }

    fn zp(&mut self) -> u16 {
        let imm = self.imm();
        u16::from(self.read(imm))
    }

    fn zpi(&mut self, r: Register) -> u16 {
        let a = self.zp();
        self.bus.cycle();
        (a + u16::from(self.reg.read(r))) & 0xFF
    }

    fn izx(&mut self) -> u16 {
        let imm = self.imm();
        let res = self.read(imm).wrapping_add(self.reg.read(Register::X));

        self.bus.cycle();

        if res == 0xFF {
            u16::from(self.read(0xFF)) | (u16::from(self.read(0x00)) << 8)
        } else {
            self.read16(u16::from(res))
        }
    }

    fn izy(&mut self, extra: bool) -> u16 {
        let imm = self.imm();
        let zero = self.read(imm);
        let y = self.reg.read(Register::Y);

        self.bus.cycle();

        let addr = if zero == 0xFF {
            u16::from(self.read(0xFF)) | (u16::from(self.read(0x00)) << 8)
        } else {
            self.read16(u16::from(zero))
        };

        if extra && CPU::cross(addr.wrapping_sub(u16::from(y)), y) {
            self.bus.cycle();
        }

        addr.wrapping_add(u16::from(y))
    }

    fn ind(&mut self) -> u16 {
        let imm = self.imm16();
        let addr = self.read16(imm);

        if (addr & 0xFF) == 0xFF {
            u16::from(self.read(addr)) | (u16::from(self.read(addr - 0xFF)) << 8)
        } else {
            self.read16(addr)
        }
    }

    fn resolve_addr(&mut self, m: AddressingMode) -> u16 {
        use self::AddressingMode::*;
        match m {
            Immediate => self.imm(),
            Absolute => self.abs(),
            AbsoluteX(s) => self.abi(s, Register::X),
            AbsoluteY(s) => self.abi(s, Register::Y),
            ZeroPage => self.zp(),
            ZeroPageX => self.zpi(Register::X),
            ZeroPageY => self.zpi(Register::Y),
            Indirect => self.ind(),
            IndirectX => self.izx(),
            IndirectY(s) => self.izy(s),
        }
    }
    // #endregion

    // #region Legal Instructions
    fn load(&mut self, r: Register, m: AddressingMode) {
        let addr = self.resolve_addr(m);
        let value = self.read(addr);
        self.reg.write(r, value);
    }

    fn store(&mut self, r: Register, m: AddressingMode) {
        let addr = self.resolve_addr(m);
        let value = self.reg.read(r);
        self.write(addr, value);
    }

    fn transfer(&mut self, from: Register, to: Register) {
        let s = self.reg.read(from);
        self.reg.write(to, s);
    }

    fn add(&mut self, m: AddressingMode) {
        let addr = self.resolve_addr(m);

        let a = self.reg.read(Register::A);
        let b = self.read(addr);
        let c = if self.reg.check_flag(Flag::Carry) {
            1u16
        } else {
            0u16
        };

        let result = u16::from(a) + u16::from(b) + c;

        self.reg.update_cv(a, b, result);
        self.reg.write(Register::A, result as u8);
    }

    fn dec_m(&mut self, m: AddressingMode) {
        let addr = self.resolve_addr(m);
        let value = self.read(addr).wrapping_sub(1);
        self.bus.cycle();

        self.reg.update_zn(value);
        self.write(addr, value);
    }

    fn dec_r(&mut self, r: Register) {
        let v = self.reg.read(r).wrapping_sub(1);
        self.reg.write(r, v);
        self.bus.cycle();
    }

    fn inc_m(&mut self, m: AddressingMode) {
        let addr = self.resolve_addr(m);
        let value = self.read(addr).wrapping_add(1);
        self.bus.cycle();

        self.reg.update_zn(value);
        self.write(addr, value);
    }

    fn inc_r(&mut self, r: Register) {
        let v = self.reg.read(r).wrapping_add(1);
        self.reg.write(r, v);
        self.bus.cycle();
    }

    fn sub(&mut self, m: AddressingMode) {
        let addr = self.resolve_addr(m);

        let a = self.reg.read(Register::A);
        let b = self.read(addr) ^ 0xFF;
        let c = if self.reg.check_flag(Flag::Carry) {
            1u16
        } else {
            0u16
        };

        let result = u16::from(a) + u16::from(b) + c;

        self.reg.update_cv(a, b, result);
        self.reg.write(Register::A, result as u8);
    }

    fn and(&mut self, m: AddressingMode) {
        let addr = self.resolve_addr(m);
        let value = self.read(addr);
        let a = self.reg.read(Register::A);
        self.reg.write(Register::A, a & value);
    }

    fn asl_a(&mut self) {
        let value = self.reg.read(Register::A);

        self.reg.update_flag(Flag::Carry, value & 0x80 == 0x80);
        self.reg.write(Register::A, value << 1);
        self.bus.cycle();
    }

    fn asl(&mut self, r: AddressingMode) {
        let addr = self.resolve_addr(r);
        let value = self.read(addr);

        self.reg.update_flag(Flag::Carry, value & 0x80 == 0x80);
        self.bus.cycle();

        self.reg.update_zn(value << 1);
        self.write(addr, value << 1);
    }

    fn bits(&mut self, m: AddressingMode) {
        let addr = self.resolve_addr(m);
        let value = self.read(addr);

        let b = self.reg.read(Register::A) & value == 0;
        self.reg.update_flag(Flag::Zero, b);
        self.reg.update_flag(Flag::Overflow, value & 0x40 == 0x40);
        self.reg.update_flag(Flag::Negative, value & 0x80 == 0x80);
    }

    fn xor(&mut self, m: AddressingMode) {
        let addr = self.resolve_addr(m);
        let value = self.read(addr);
        let a = self.reg.read(Register::A);
        self.reg.write(Register::A, a ^ value);
    }

    fn lsr_a(&mut self) {
        let value = self.reg.read(Register::A);

        self.reg.update_flag(Flag::Carry, value & 0x01 == 0x01);
        self.reg.write(Register::A, value >> 1);
        self.bus.cycle();
    }

    fn lsr(&mut self, r: AddressingMode) {
        let addr = self.resolve_addr(r);
        let value = self.read(addr);

        self.reg.update_flag(Flag::Carry, value & 0x01 == 0x01);
        self.bus.cycle();

        self.reg.update_zn(value >> 1);
        self.write(addr, value >> 1);
    }

    fn or(&mut self, m: AddressingMode) {
        let addr = self.resolve_addr(m);
        let value = self.read(addr);
        let a = self.reg.read(Register::A);
        self.reg.write(Register::A, a | value);
    }

    fn flag(&mut self, f: Flag, s: bool) {
        self.reg.update_flag(f, s);
        self.bus.cycle();
    }

    fn compare(&mut self, r: Register, m: AddressingMode) {
        let reg = self.reg.read(r);
        let addr = self.resolve_addr(m);
        let value = self.read(addr);

        self.reg.update_flag(Flag::Carry, reg >= value);
        self.reg.update_flag(Flag::Zero, reg == value);
        self.reg.update_flag(
            Flag::Negative,
            (i16::from(reg) - i16::from(value)) & 0x80 == 0x80,
        );
    }

    fn jsr(&mut self) {
        let t = self.reg.read_pc().wrapping_add(1);
        self.bus.cycle();
        self.push16(t);
        let addr = self.imm16();
        let value = self.read16(addr);
        self.reg.write_pc(value);
    }

    fn jump(&mut self, m: AddressingMode) {
        let addr = self.resolve_addr(m);
        self.reg.write_pc(addr);
    }

    fn stack(&mut self, r: Register, push: bool) {
        if push {
            self.bus.cycle();

            let value = match r {
                Register::P => self.reg.read(Register::P) | 0b0001_0000,
                r => self.reg.read(r),
            };

            self.push(value);
        } else {
            self.bus.cycle();
            self.bus.cycle();
            let value = self.pop();
            self.reg.write(r, value);
        }
    }

    fn rti(&mut self) {
        self.stack(Register::P, false);
        let addr = self.pop16();
        self.reg.write_pc(addr);
    }

    fn rts(&mut self) {
        self.bus.cycle();
        self.bus.cycle();
        let addr = self.pop16().wrapping_add(1);
        self.reg.write_pc(addr);
        self.bus.cycle();
    }

    fn brk(&mut self) {
        let addr = self.reg.read_pc().wrapping_add(1);
        self.push16(addr);

        let flags = self.reg.read(Register::P) | 0b0001_0000;

        self.push(flags);
        self.reg.update_flag(Flag::Interrupt, true);

        let val = if self.nmi {
            self.read16(NMI_VECTOR)
        } else {
            self.read16(IRQ_VECTOR)
        };

        self.reg.write_pc(val);
    }

    fn branch(&mut self, cond: Flag, when: bool) {
        let addr = self.imm();
        let value = self.read(addr) as i8;

        if self.reg.check_flag(cond) == when {
            self.bus.cycle();
            let pc = self.reg.read_pc();

            if CPU::cross(pc, value as u8) {
                self.bus.cycle();
            }

            let res = pc as i16 + i16::from(value);
            self.reg.write_pc(res as u16);
        }
    }

    fn nop(&mut self, m: Option<AddressingMode>) {
        if let Some(m) = m {
            let addr = self.resolve_addr(m);
            self.read(addr);
        }
        self.bus.cycle();
    }

    fn rol_a(&mut self) {
        let c = if self.reg.check_flag(Flag::Carry) {
            1
        } else {
            0
        };

        let value = self.reg.read(Register::A);
        self.reg.update_flag(Flag::Carry, value & 0x80 == 0x80);
        self.reg.write(Register::A, (value << 1) | c);
        self.bus.cycle();
    }

    fn rol(&mut self, m: AddressingMode) {
        let c = if self.reg.check_flag(Flag::Carry) {
            1
        } else {
            0
        };

        let addr = self.resolve_addr(m);
        let value = self.read(addr);

        self.reg.update_flag(Flag::Carry, value & 0x80 == 0x80);
        self.bus.cycle();

        self.reg.update_zn((value << 1) | c);
        self.write(addr, (value << 1) | c);
    }

    fn ror_a(&mut self) {
        let c = if self.reg.check_flag(Flag::Carry) {
            0x80
        } else {
            0
        };

        let value = self.reg.read(Register::A);
        self.reg.update_flag(Flag::Carry, value & 0x01 == 0x01);
        self.reg.write(Register::A, c | (value >> 1));
        self.bus.cycle();
    }

    fn ror(&mut self, m: AddressingMode) {
        let c = if self.reg.check_flag(Flag::Carry) {
            0x80
        } else {
            0
        };

        let addr = self.resolve_addr(m);
        let value = self.read(addr);

        self.reg.update_flag(Flag::Carry, value & 0x01 == 0x01);
        self.bus.cycle();

        self.reg.update_zn(c | (value >> 1));
        self.write(addr, c | (value >> 1));
    }
    // #endregion

    // #region Illegal Instructions
    fn lax(&mut self, m: AddressingMode) {
        let addr = self.resolve_addr(m);
        let value = self.read(addr);

        self.reg.write(Register::A, value);
        self.reg.write(Register::X, value);
    }

    fn sax(&mut self, m: AddressingMode) {
        let addr = self.resolve_addr(m);
        let a = self.reg.read(Register::A);
        let x = self.reg.read(Register::X);

        self.write(addr, a & x);
    }

    fn dcp(&mut self, m: AddressingMode) {
        let addr = self.resolve_addr(m);
        let value = self.read(addr).wrapping_sub(1);

        self.bus.cycle();

        let reg = self.reg.read(Register::A);
        self.reg.update_flag(Flag::Carry, reg >= value);
        self.reg.update_flag(Flag::Zero, reg == value);
        self.reg.update_flag(
            Flag::Negative,
            (i16::from(reg) - i16::from(value)) & 0x80 == 0x80,
        );

        self.write(addr, value);
    }

    fn isb(&mut self, m: AddressingMode) {
        let addr = self.resolve_addr(m);
        let value = self.read(addr).wrapping_add(1);
        self.bus.cycle();

        let a = self.reg.read(Register::A);
        let b = value ^ 0xFF;
        let c = if self.reg.check_flag(Flag::Carry) {
            1u16
        } else {
            0u16
        };

        let result = u16::from(a) + u16::from(b) + c;

        self.reg.update_cv(a, b, result);
        self.reg.write(Register::A, result as u8);
        self.write(addr, value);
    }

    fn slo(&mut self, m: AddressingMode) {
        let addr = self.resolve_addr(m);
        let value = self.read(addr);
        self.bus.cycle();

        self.reg.update_flag(Flag::Carry, value & 0x80 == 0x80);
        let a = self.reg.read(Register::A);

        self.reg.update_zn(value << 1);
        self.reg.write(Register::A, a | (value << 1));
        self.write(addr, value << 1);
    }

    fn rla(&mut self, m: AddressingMode) {
        let addr = self.resolve_addr(m);
        let value = self.read(addr);

        self.bus.cycle();

        let c = if self.reg.check_flag(Flag::Carry) {
            1
        } else {
            0
        };

        self.reg.update_flag(Flag::Carry, value & 0x80 == 0x80);

        let a = self.reg.read(Register::A);
        let result = (value << 1) | c;
        self.reg.update_zn(result);
        self.reg.write(Register::A, a & result);
        self.write(addr, result);
    }

    fn sre(&mut self, m: AddressingMode) {
        let addr = self.resolve_addr(m);
        let value = self.read(addr);

        self.bus.cycle();

        self.reg.update_flag(Flag::Carry, value & 0x01 == 0x01);

        let a = self.reg.read(Register::A);

        let result = value >> 1;
        self.reg.update_zn(result);
        self.reg.write(Register::A, a ^ result);
        self.write(addr, result);
    }

    fn rra(&mut self, m: AddressingMode) {
        let addr = self.resolve_addr(m);
        let value = self.read(addr);

        self.bus.cycle();

        let c = if self.reg.check_flag(Flag::Carry) {
            0x80
        } else {
            0
        };

        self.reg.update_flag(Flag::Carry, value & 0x01 == 0x01);

        let x = c | (value >> 1);
        self.reg.update_zn(x);

        let a = self.reg.read(Register::A);
        let c = if self.reg.check_flag(Flag::Carry) {
            1u16
        } else {
            0u16
        };

        let result = u16::from(a) + u16::from(x) + c;

        self.reg.update_cv(a, x, result);
        self.reg.write(Register::A, result as u8);
        self.write(addr, x);
    }

    fn aac(&mut self, m: AddressingMode) {
        let addr = self.resolve_addr(m);
        let value = self.read(addr);

        let a = self.reg.read(Register::A);
        self.reg.write(Register::A, a & value);

        let b = self.reg.check_flag(Flag::Negative);
        self.reg.update_flag(Flag::Carry, b);
    }

    fn asr(&mut self, m: AddressingMode) {
        let addr = self.resolve_addr(m);
        let value = self.read(addr);
        let reg = self.reg.read(Register::A);

        self.reg.write(Register::A, reg & value);
        let res = self.reg.read(Register::A);
        self.reg.update_flag(Flag::Carry, res & 0x01 == 0x01);
        self.reg.write(Register::A, res >> 1);
    }

    fn arr(&mut self, m: AddressingMode) {
        let addr = self.resolve_addr(m);
        let value = self.read(addr);

        let res = if self.reg.check_flag(Flag::Carry) {
            ((self.reg.read(Register::A) & value) >> 1) | 0x80
        } else {
            (self.reg.read(Register::A) & value) >> 1
        };

        self.reg.write(Register::A, res);

        let reg = self.reg.read(Register::A);
        self.reg.update_flag(Flag::Carry, reg & 0x40 == 0x40);

        let x = if self.reg.check_flag(Flag::Carry) {
            1
        } else {
            0
        };

        self.reg
            .update_flag(Flag::Overflow, x ^ (reg >> 5) & 0x01 != 0);
    }

    fn atx(&mut self, m: AddressingMode) {
        let addr = self.resolve_addr(m);
        let value = self.read(addr);

        self.reg.write(Register::A, value);
        self.reg.write(Register::X, value);
        self.reg.write(Register::A, value);
    }

    fn axs(&mut self, m: AddressingMode) {
        let addr = self.resolve_addr(m);
        let value = self.read(addr);
        let (a, x) = (self.reg.read(Register::A), self.reg.read(Register::X));
        self.reg.update_flag(Flag::Carry, a & x >= value);
        self.reg.write(Register::X, (a & x).wrapping_sub(value));
    }

    fn sa(&mut self, r: Register, m: AddressingMode) {
        let addr = self.resolve_addr(m);
        let hi = (addr >> 8) as u8;
        let lo = (addr & 0xFF) as u8;

        let val = self.reg.read(r) & (hi.wrapping_add(1));
        let ad = (u16::from(val) << 8) | u16::from(lo);
        self.write(ad, val);
    }
    // #endregion
}
