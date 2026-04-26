use super::opcode::*;
use rand::random;

use super::memops::*;

#[derive(Debug, Clone)]
pub struct Chip8(pub(crate) [u8; 0x1000]);

impl Default for Chip8 {
    fn default() -> Self {
        let mut cero = Self([0; _]);
        cero.write(0x200u16, PC).unwrap();

        cero
    }
}

pub enum PostExecute {
    Next,
    Stay,
}

impl Chip8 {
    /// Add current PC to top of stack
    fn push(&mut self) -> Option<()> {
        let sp: u8 = self.read(SP)?;
        assert!(sp < 16, "sp over bounds {}", sp);

        let pc: u16 = self.read(PC)?;
        self.write(pc, Stack + sp)?;

        self.write(sp + 2, SP)?;
        Some(())
    }
    /// Pop from top of stack into PC
    fn pop(&mut self) -> Option<u16> {
        let sp: u8 = ByteArray::<u8>::read(self, SP)?;
        assert!(sp != 0, "sp under bounds");

        let sp = sp - 2;
        self.write(sp, SP)?;
        self.read(Stack + sp)
    }
    pub fn load_program<'a>(&'a mut self, program: &[u8]) -> Option<()> {
        self.0[(Memory as usize)..(Memory as usize + program.len())].copy_from_slice(program);
        Some(())
    }
    pub fn load_data(&mut self, data: &[u8]) -> Option<()> {
        self.0[(Data as usize)..(Data as usize + data.len())].copy_from_slice(data);
        Some(())
    }
    pub fn next_instruction(&mut self) -> Option<()> {
        let next = ByteArray::<u16>::read(self, PC)? + 2;
        self.write(next, PC)?;

        Some(())
    }

    pub fn execute(&mut self, opcode: OpCode) -> Option<PostExecute> {
        // TODO add guards
        use OpCode::*;
        match opcode {
            NoOp { .. } => panic!("noop"),
            Clear => self.write([0u8; Display.size()], Display)?,
            Draw { x, y, size } => {
                assert!(x <= 0x3f, "x out of range of screen");
                assert!(y <= 0x1f, "y out of range of screen");
                assert!(size > 0, "size bellow range");
                assert!(size <= 0xf, "size over range");

                let (x, y): (u8, u8) = (self.read(Vs + x)?, self.read(Vs + y)?);
                let i: u16 = self.read(I)?;

                for n in 0..size {
                    let sprite =
                        (ByteArray::<u8>::read(self, i + n as u16)? as u64) << (64 - 8) >> x;
                    let line_start = (y + n) * 8;
                    let screen: u64 = self.read(Display + line_start)?;

                    let xor = sprite ^ screen;

                    self.write(((screen & !xor) == 0) as u8, Vs + 0xf)?;
                    self.write(xor, Display + line_start)?;
                }
            }
            Return => {
                let adr = self.pop()?;
                self.write(adr, PC)?;
            }
            Jump { to } => {
                self.write(to, PC)?;
                return Some(PostExecute::Stay);
            }
            Call { at } => {
                self.push()?;
                self.write(at, PC)?;
                return Some(PostExecute::Stay);
            }
            JumpReg { by } => {
                let v0: u8 = self.read(Vs + 0)?;
                let sum = v0 as u16 + by;
                self.write(sum, PC)?;
                return Some(PostExecute::Stay);
            }
            SkipEqK { reg, val } => {
                let vx: u8 = self.read(Vs + reg)?;
                if vx == val {
                    self.next_instruction();
                }
            }
            SkipNotEqK { reg, val } => {
                let vx: u8 = self.read(Vs + reg)?;
                if vx != val {
                    self.next_instruction();
                }
            }
            SkipEq { a, b } => {
                let (va, vb): (u8, u8) = (self.read(Vs + a)?, self.read(Vs + b)?);
                if va == vb {
                    self.next_instruction();
                }
            }
            SkipNotEq { a, b } => {
                let (va, vb): (u8, u8) = (self.read(Vs + a)?, self.read(Vs + b)?);
                if va != vb {
                    self.next_instruction();
                }
            }
            SetV { reg, to } => self.write(to, Vs + reg)?,
            IncV { reg, by } => {
                let vx: u8 = self.read(Vs + reg)?;
                self.write(vx.wrapping_add(by), Vs + reg)?;
            }
            SetI { to } => self.write(to, I)?,
            GetRand { reg, mask } => self.write(random::<u8>() & mask, Vs + reg)?,
            SkipPressed { key } => {
                let reg: u8 = self.read(Vs + key)?;
                let pressed: u8 = self.read(KEY)?;
                if reg == pressed {
                    self.next_instruction();
                }
            }
            SkipNotPressed { key } => {
                let reg: u8 = self.read(Vs + key)?;
                let pressed: u8 = self.read(KEY)?;
                if reg != pressed {
                    self.next_instruction();
                }
            }
            ReadKey { to } => todo!("set up blocking"),
            ReadDelay { to } => {
                let dt: u8 = self.read(DT)?;
                self.write(dt, Vs + to)?;
            }
            SetDelay { with } => {
                let vx: u8 = self.read(Vs + with)?;
                self.write(vx, DT)?;
            }
            SetSound { with } => {
                let vx: u8 = self.read(Vs + with)?;
                self.write(vx, ST)?;
            }
            IncI { with } => {
                let i: u16 = self.read(I)?;
                let vx = ByteArray::<u8>::read(self, Vs + with)? as u16;

                self.write(vx + i, I)?;
            }
            GetFontSprite { of } => {
                let vx: u8 = self.read(Vs + of)?;
                let adr = vx as u16 * 5;

                self.write(adr, I)?;
            }
            SaveRegs { upto } => {
                let i: u16 = self.read(I)?;
                let regs: u128 = self.read(Vs)?;
                let mems: u128 = self.read(i)?;

                let mask = !0u128 << ((0xf - upto) * 8);
                let res = (mems & !mask) | (regs & mask);
                self.write(res, i)?;

                let sum = i + upto as u16 + 1;
                self.write(sum, I)?;
            }
            LoadRegs { upto } => {
                let i: u16 = self.read(I)?;
                let regs: u128 = self.read(Vs)?;
                let mems: u128 = self.read(i)?;

                let mask = !0u128 << ((0xf - upto) * 8);
                let res = (mems & mask) | (regs & !mask);
                self.write(res, Vs)?;

                let sum = i + upto as u16 + 1;
                self.write(sum, I)?;
            }
            BCD { of } => {
                let of: u8 = self.read(Vs + of)?;
                let i: u16 = self.read(I)?;

                self.write(
                    [
                        (of / 100) % 10,
                        (of / 10) % 10,
                        (of / 1) % 10, //
                    ],
                    i,
                )?;
            }
            Assign { a, to } => {
                let vb: u8 = self.read(Vs + to)?;
                self.write(vb, Vs + a)?;
            }
            Or { a, b } => {
                let (va, vb): (u8, u8) = (self.read(Vs + a)?, self.read(Vs + b)?);
                self.write(va | vb, Vs + a)?;
            }
            And { a, b } => {
                let (va, vb): (u8, u8) = (self.read(Vs + a)?, self.read(Vs + b)?);
                self.write(va & vb, Vs + a)?;
            }
            Xor { a, b } => {
                let (va, vb): (u8, u8) = (self.read(Vs + a)?, self.read(Vs + b)?);
                self.write(va ^ vb, Vs + a)?;
            }
            Add { a, b } => {
                let (va, vb) = (self.read(Vs + a)?, self.read(Vs + b)?);
                let (res, overflowed) = u8::overflowing_add(va, vb);
                self.write(res, Vs + a)?;
                self.write(overflowed as u8, Vs + 0xf)?;
            }
            Sub { a, b } => {
                let (va, vb) = (self.read(Vs + a)?, self.read(Vs + b)?);
                let (res, overflowed) = u8::overflowing_sub(va, vb);
                self.write(res, Vs + a)?;
                self.write(overflowed as u8, Vs + 0xf)?;
            }
            ShiftR { a, b } => {
                let vb: u8 = self.read(Vs + b)?;
                self.write(vb & 1, Vs + 0xf)?;
                self.write(vb >> 1, Vs + a)?;
            }
            SubN { a, b } => {
                let (va, vb) = (self.read(Vs + a)?, self.read(Vs + b)?);
                let (res, overflowed) = u8::overflowing_sub(vb, va);
                self.write(res, Vs + a)?;
                self.write(overflowed as u8, Vs + 0xf)?;
            }
            ShiftL { a, b } => {
                let vb: u8 = self.read(Vs + b)?;
                self.write(vb >> 7, Vs + 0xf)?;
                self.write(vb << 1, Vs + a)?;
            }
        }

        Some(PostExecute::Next)
    }
    pub fn step(&mut self) -> Option<()> {
        let pc: u16 = self.read(PC)?;
        assert!(pc < Memory.size() as _, "pc out of bounds");

        let fragment: u16 = self.read(pc)?;
        let opcode = fragment.into();

        match self.execute(opcode)? {
            PostExecute::Next => self.next_instruction()?,
            PostExecute::Stay => (),
        }

        Some(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)]

    use super::*;

    const INITIAL_PC: u16 = 0x200;

    /// A default chip8 should be all 0 with a pc at 0x200
    #[test]
    fn default() {
        let chip = Chip8::default();

        let pc: u16 = chip.read(PC).unwrap();
        assert!(pc == INITIAL_PC);
    }

    // Set Display to 0
    #[test]
    fn clear() {
        let mut chip = Chip8::default();

        chip.write(0xffffffu64, Display).unwrap();
        println!(
            "Mm: {:032x}",
            ByteArray::<u128>::read(&chip, Display).unwrap()
        );
        chip.execute(OpCode::Clear);
        let display: u128 = chip.read(Display).unwrap();

        assert!(display == 0);
    }

    // Pop & Set PC
    // Push & Set PC NNN
    #[test]
    fn subroutines() {
        let mut chip = Chip8::default();
        chip.execute(OpCode::Call { at: 0xfb03 });
        chip.execute(OpCode::Call { at: 0xfb03 });
        chip.execute(OpCode::Call { at: 0xfb03 });
        let pc: u16 = chip.read(PC).unwrap();

        assert!(pc == 0xfb03);

        chip.execute(OpCode::Return);
        chip.execute(OpCode::Return);
        chip.execute(OpCode::Return);
        let pc: u16 = chip.read(PC).unwrap();

        assert!(pc == INITIAL_PC);
    }

    // Skip if VX == NN
    // Skip if VX != NN
    // Skip if VX == VY
    // Skip if VX != VY
    // Skip if VX == key
    // Skip if VX != key
    #[test]
    fn skips() {
        let mut chip = Chip8::default();
        chip.write(7u8, Vs + 4).unwrap();

        chip.execute(OpCode::SkipEqK { reg: 4, val: 0 });
        let pc: u16 = chip.read(PC).unwrap();
        assert!(pc == INITIAL_PC);
        chip.execute(OpCode::SkipEqK { reg: 4, val: 7 });
        let pc: u16 = chip.read(PC).unwrap();
        assert!(pc == INITIAL_PC + 2);

        chip.execute(OpCode::SkipNotEqK { reg: 4, val: 7 });
        let pc: u16 = chip.read(PC).unwrap();
        assert!(pc == INITIAL_PC + 2);
        chip.execute(OpCode::SkipNotEqK { reg: 4, val: 0 });
        let pc: u16 = chip.read(PC).unwrap();
        assert!(pc == INITIAL_PC + 4);

        chip.execute(OpCode::SkipEq { a: 4, b: 7 });
        let pc: u16 = chip.read(PC).unwrap();
        assert!(pc == INITIAL_PC + 4);
        chip.execute(OpCode::SkipEq { a: 7, b: 7 });
        let pc: u16 = chip.read(PC).unwrap();
        assert!(pc == INITIAL_PC + 6);

        chip.execute(OpCode::SkipNotEq { a: 7, b: 7 });
        let pc: u16 = chip.read(PC).unwrap();
        assert!(pc == INITIAL_PC + 6);
        chip.execute(OpCode::SkipNotEq { a: 4, b: 7 });
        let pc: u16 = chip.read(PC).unwrap();
        assert!(pc == INITIAL_PC + 8);

        chip.write(7u8, KEY).unwrap();

        chip.execute(OpCode::SkipPressed { key: 0 });
        let pc: u16 = chip.read(PC).unwrap();
        assert!(pc == INITIAL_PC + 8);
        chip.execute(OpCode::SkipPressed { key: 4 });
        let pc: u16 = chip.read(PC).unwrap();
        assert!(pc == INITIAL_PC + 10);

        chip.execute(OpCode::SkipNotPressed { key: 4 });
        let pc: u16 = chip.read(PC).unwrap();
        assert!(pc == INITIAL_PC + 10);
        chip.execute(OpCode::SkipNotPressed { key: 0 });
        let pc: u16 = chip.read(PC).unwrap();
        assert!(pc == INITIAL_PC + 12);
    }

    // Set PC to NNN
    // Set PC to V0 + NNN
    // Set I to NNN
    // Set I to the sprite location of VX
    // Inc I with VX
    #[test]
    fn set16() {
        let mut chip = Chip8::default();

        chip.execute(OpCode::Jump { to: 0xfb03 });
        let pc: u16 = chip.read(PC).unwrap();
        assert!(pc == 0xfb03);

        chip.write(7u8, Vs + 0).unwrap();
        chip.execute(OpCode::JumpReg { by: 0xfb03 });
        let pc: u16 = chip.read(PC).unwrap();
        assert!(pc == 0xfb0a);

        chip.execute(OpCode::SetI { to: 0xfb03 });
        let i: u16 = chip.read(I).unwrap();
        assert!(i == 0xfb03);

        chip.execute(OpCode::GetFontSprite { of: 0 });
        let i: u16 = chip.read(I).unwrap();
        assert!(i == 7 * 5);

        chip.execute(OpCode::IncI { with: 0 });
        let i: u16 = chip.read(I).unwrap();
        assert!(i == 42);
    }

    // Set DT to VX
    // Set ST to VX
    // Set VX to DT
    // Set VX to NN
    // Set VX to KEY (block)
    // Set VX to rand & NN
    // Inc VX by NN
    #[test]
    fn set8() {
        let mut chip = Chip8::default();
        chip.write(7u8, Vs + 4).unwrap();

        chip.execute(OpCode::SetDelay { with: 4 });
        let dt: u8 = chip.read(DT).unwrap();
        assert!(dt == 7);

        chip.execute(OpCode::SetSound { with: 4 });
        let st: u8 = chip.read(ST).unwrap();
        assert!(st == 7);

        chip.execute(OpCode::ReadDelay { to: 3 });
        let v3: u8 = chip.read(Vs + 3).unwrap();
        assert!(v3 == 7);

        chip.execute(OpCode::SetV { reg: 2, to: 8 });
        let v2: u8 = chip.read(Vs + 2).unwrap();
        assert!(v2 == 8);

        chip.execute(OpCode::IncV { reg: 2, by: 8 });
        let v2: u8 = chip.read(Vs + 2).unwrap();
        assert!(v2 == 16);

        // chip.write(4u8 ,KEY);
        // chip.execute(OpCode::ReadKey { to: 1 });
        // let v1 : u8 = chip.read(Vs + 1).unwrap();
        // assert!(v1 == 4);

        const VAL: u8 = 0b11110000;
        chip.write(VAL, Vs + 9).unwrap();
        chip.execute(OpCode::GetRand { reg: 9, mask: !VAL });
        let v9: u8 = chip.read(Vs + 9).unwrap();
        assert!(v9 != VAL);
    }

    // TODO Draw sprite from I at VX VY with N lines; VF = some 1 got turned off
    // TODO BCD of VX at I

    // Save V0..VX to I
    // Load V0..VX from I
    #[test]
    fn save_load() {
        let mut chip = Chip8::default();
        chip.write(0x77u8, Vs + 7).unwrap();
        chip.write(0x44u8, Vs + 4).unwrap();
        chip.write(INITIAL_PC, I).unwrap();
        chip.write(0x55u8, Memory + 5).unwrap();

        chip.execute(OpCode::SaveRegs { upto: 4 });
        let i: u16 = chip.read(I).unwrap();
        assert!(i == INITIAL_PC + 4 + 1);

        let M04: u8 = chip.read(Memory + 4).unwrap();
        assert!(M04 == 0x44);

        chip.write(INITIAL_PC, I).unwrap();
        chip.write(0x33u8, Memory + 3).unwrap();
        chip.execute(OpCode::LoadRegs { upto: 7 });

        let i: u16 = chip.read(I).unwrap();
        assert!(i == INITIAL_PC + 7 + 1);

        let v3: u8 = chip.read(Vs + 3).unwrap();
        assert!(v3 == 0x33);

        let v7: u8 = chip.read(Vs + 7).unwrap();
        assert!(v7 == 0x00);
    }

    // Set VX to VY
    // VX |= VY
    // VX &= VY
    // VX ^= VY
    // VX += VY; VF = overflow
    // VX -= VY; VF = !undeflow
    // VX = VY >> 1; VF = VY[0] lsb
    // VX = VY - VX; VF = !underflow
    // VX = VY << 1; VF = VY[7] msb
}
