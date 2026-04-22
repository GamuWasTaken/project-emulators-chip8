use super::opcode::*;
use rand::random;

use super::memops::*;

#[derive(Debug, Clone)]
pub struct Chip8(pub(crate) [u8; 0x1000]);

impl Default for Chip8 {
    fn default() -> Self {
        let mut cero = Self([0; _]);
        copy!(&0x200u16.to_be_bytes() => cero PC);

        cero
    }
}

impl Chip8 {
    /// Add current PC to top of stack
    fn push(&mut self) {
        let sp = self[SP] as usize;
        assert!(sp <= 16, "sp over bounds {}", self[SP]);

        let pc = read!(u16 from self PC);
        copy!(&pc.to_be_bytes() => self Stack at sp);

        self[SP] += 2;
    }
    /// Pop from top of stack into PC
    fn pop(&mut self) -> [u8; 2] {
        assert!(self[SP] > 0, "sp under bounds");
        self[SP] -= 2;
        let sp = self[SP];
        [self[Stack + sp], self[Stack + sp + 1]]
    }
    pub fn load_program(&mut self, program: &[u8]) {
        copy!(program => self Memory);
    }
    pub fn load_data(&mut self, data: &[u8]) {
        copy!(data => self Data);
    }
    pub fn next_instruction(&mut self) {
        let next = read!(u16 from self PC) + 2;
        copy!(&next.to_be_bytes() => self PC);
    }

    pub fn execute(&mut self, opcode: OpCode) {
        // TODO add guards
        use OpCode::*;
        match opcode {
            NoOp { .. } => (),
            Clear => copy!(&[0; Display.size()] => self Display),
            Draw { x, y, size } => {
                assert!(x < 64, "x out of range of screen");
                assert!(y < 32, "y out of range of screen");
                assert!(size > 0, "size bellow range");
                assert!(size < 16, "size over range");

                let (x, y) = (self[Vs + x], self[Vs + y]);
                let i = read!(u16 from self I);

                for n in 0..size {
                    let sprite = (self[i + n as u16] as u64) << (64 - 8) >> x;
                    let line_start = y + n * 8;
                    let screen = read!(u64 from self Display at line_start);

                    let xor = sprite ^ screen;
                    self[Vs + 0xf] = ((screen & !xor) == 0) as u8;
                    copy!(&xor.to_le_bytes() => self Display at line_start as usize);
                }
            }
            Return => {
                let adr = self.pop();
                copy!(&adr => self PC);
            }
            Jump { to } => copy!(&to => self PC),
            Call { at } => {
                self.push();
                copy!(&at => self PC);
            }
            JumpReg { by } => {
                let sum = self[Vs + 0] as u16 + u16::from_be_bytes(by);
                copy!(&sum.to_be_bytes() => self PC);
            }
            SkipEqK { reg, val } => {
                if self[Vs + reg] == val {
                    self.next_instruction();
                }
            }

            SkipNotEqK { reg, val } => {
                if self[Vs + reg] != val {
                    self.next_instruction();
                }
            }
            SkipEq { a, b } => {
                if self[Vs + a] == self[Vs + b] {
                    self.next_instruction();
                }
            }
            SkipNotEq { a, b } => {
                if self[Vs + a] != self[Vs + b] {
                    self.next_instruction();
                }
            }
            SetV { reg, to } => self[Vs + reg] = to,
            IncV { reg, by } => self[Vs + reg] = self[Vs + reg].wrapping_add(by),
            SetI { to } => copy!(&to => self I),
            GetRand { reg, mask } => self[Vs + reg] = random::<u8>() & mask,
            SkipPressed { key } => {
                let reg = read!(u8 from self Vs at key);
                let pressed = read!(u8 from self KEY);
                if reg == pressed {
                    self.next_instruction();
                }
            }
            SkipNotPressed { key } => {
                let reg = read!(u8 from self Vs at key);
                let pressed = read!(u8 from self KEY);
                if reg != pressed {
                    self.next_instruction();
                }
            }
            ReadKey { to } => todo!("set up blocking"),
            ReadDelay { to } => self[Vs + to] = self[DT],
            SetDelay { with } => self[DT] = self[Vs + with],
            SetSound { with } => self[ST] = self[Vs + with],
            IncI { with } => {
                let sum = self[Vs + with] as u16 + read!(u16 from self I);
                copy!(&sum.to_be_bytes() => self I)
            }
            GetFontSprite { of } => {
                let adr = self[Vs + of] as u16 * 5;
                copy!(&adr.to_be_bytes() => self I);
            }
            SaveRegs { upto } => {
                let i = read!(u16 from self I);
                let regs = read!(u128 from self Vs);
                let mems = read!(u128 from self Memory);

                let mask = !0u128 << ((0xf - upto) * 8);

                copy!(&(
                    (mems & !mask) | (regs & mask)
                ).to_be_bytes() => self at i);

                let sum = i + upto as u16 + 1;
                copy!(&sum.to_be_bytes() => self I);
            }
            LoadRegs { upto } => {
                let i = read!(u16 from self I);
                let regs = read!(u128 from self Vs);
                let mems = read!(u128 from self Memory at (i-0x200));

                let mask = !0u128 << ((0xf - upto) * 8);

                copy!(&(
                    (mems & mask) | (regs & !mask)
                ).to_be_bytes() => self Vs);

                let sum = i + upto as u16 + 1;
                copy!(&sum.to_be_bytes() => self I);
            }
            BCD { of } => {
                let of = self[Vs + of];
                copy!(&[
                    (of / 100) % 10,
                    (of / 10) % 10,
                    (of / 1) % 10,
                ] => self Memory );
            }
            Assign { a, to } => self[Vs + a] = self[Vs + to],
            Or { a, b } => self[Vs + a] |= self[Vs + b],
            And { a, b } => self[Vs + a] &= self[Vs + b],
            Xor { a, b } => self[Vs + a] ^= self[Vs + b],
            Add { a, b } => {
                let res = self[Vs + a] as u16 + self[Vs + b] as u16;
                self[Vs + a] = res as u8;
                self[Vs + 0xf] = (res & 0xff00 == 0) as u8;
            }
            Sub { a, b } => {
                let (a_val, b_val) = (self[Vs + a], self[Vs + b]);
                self[Vs + a] = a_val.wrapping_sub(b_val);
                self[Vs + 0xf] = (a_val < b_val) as u8;
            }
            ShiftR { a, b } => {
                self[Vs + 0xf] = self[Vs + b] & 0x1;
                self[Vs + a] = self[Vs + b] >> 1;
            }
            SubN { a, b } => {
                let (a_val, b_val) = (self[Vs + a], self[Vs + b]);
                self[Vs + a] = b_val.wrapping_sub(a_val);
                self[Vs + 0xf] = (b_val < a_val) as u8;
            }
            ShiftL { a, b } => {
                self[Vs + 0xf] = self[Vs + b] >> 7;
                self[Vs + a] = self[Vs + b] << 1;
            }
        }
    }
    pub fn step(&mut self) {
        let pc = read!(u16 from self PC);
        assert!(pc < Memory.size() as _, "pc out of bounds");

        let fragment = read!(u16 from self Memory at (pc - 0x200));
        let opcode = fragment.into();

        self.execute(opcode);

        self.next_instruction();
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

        let pc = read!(u16 from chip PC);
        assert!(pc == INITIAL_PC);
    }

    // Set Display to 0
    #[test]
    fn clear() {
        let mut chip = Chip8::default();

        copy!(&0xffffffu64.to_be_bytes() => chip Display);
        chip.execute(OpCode::Clear);
        let display = read!(u128 from chip Display);

        assert!(display == 0);
    }

    // Pop & Set PC
    // Push & Set PC NNN
    #[test]
    fn subroutines() {
        let mut chip = Chip8::default();
        chip.execute(OpCode::Call { at: [0xfb, 0x03] });
        let pc = read!(u16 from chip PC);

        assert!(pc == 0xfb03);

        chip.execute(OpCode::Return);
        let pc = read!(u16 from chip PC);

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
        copy!(&[7] => chip Vs at 4);

        chip.execute(OpCode::SkipEqK { reg: 4, val: 0 });
        let pc = read!(u16 from chip PC);
        assert!(pc == INITIAL_PC);
        chip.execute(OpCode::SkipEqK { reg: 4, val: 7 });
        let pc = read!(u16 from chip PC);
        assert!(pc == INITIAL_PC + 2);

        chip.execute(OpCode::SkipNotEqK { reg: 4, val: 7 });
        let pc = read!(u16 from chip PC);
        assert!(pc == INITIAL_PC + 2);
        chip.execute(OpCode::SkipNotEqK { reg: 4, val: 0 });
        let pc = read!(u16 from chip PC);
        assert!(pc == INITIAL_PC + 4);

        chip.execute(OpCode::SkipEq { a: 4, b: 7 });
        let pc = read!(u16 from chip PC);
        assert!(pc == INITIAL_PC + 4);
        chip.execute(OpCode::SkipEq { a: 7, b: 7 });
        let pc = read!(u16 from chip PC);
        assert!(pc == INITIAL_PC + 6);

        chip.execute(OpCode::SkipNotEq { a: 7, b: 7 });
        let pc = read!(u16 from chip PC);
        assert!(pc == INITIAL_PC + 6);
        chip.execute(OpCode::SkipNotEq { a: 4, b: 7 });
        let pc = read!(u16 from chip PC);
        assert!(pc == INITIAL_PC + 8);

        copy!(&[7] => chip KEY);

        chip.execute(OpCode::SkipPressed { key: 0 });
        let pc = read!(u16 from chip PC);
        assert!(pc == INITIAL_PC + 8);
        chip.execute(OpCode::SkipPressed { key: 4 });
        let pc = read!(u16 from chip PC);
        assert!(pc == INITIAL_PC + 10);

        chip.execute(OpCode::SkipNotPressed { key: 4 });
        let pc = read!(u16 from chip PC);
        assert!(pc == INITIAL_PC + 10);
        chip.execute(OpCode::SkipNotPressed { key: 0 });
        let pc = read!(u16 from chip PC);
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

        chip.execute(OpCode::Jump { to: [0xfb, 0x03] });
        let pc = read!(u16 from chip PC);
        assert!(pc == 0xfb03);

        copy!(&[7] => chip Vs at 0);
        chip.execute(OpCode::JumpReg { by: [0xfb, 0x03] });
        let pc = read!(u16 from chip PC);
        assert!(pc == 0xfb0a);

        chip.execute(OpCode::SetI { to: [0xfb, 0x03] });
        let i = read!(u16 from chip I);
        assert!(i == 0xfb03);

        chip.execute(OpCode::GetFontSprite { of: 0 });
        let i = read!(u16 from chip I);
        assert!(i == 7 * 5);

        chip.execute(OpCode::IncI { with: 0 });
        let i = read!(u16 from chip I);
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
        copy!(&[7] => chip Vs at 4);

        chip.execute(OpCode::SetDelay { with: 4 });
        let dt = read!(u8 from chip DT);
        assert!(dt == 7);

        chip.execute(OpCode::SetSound { with: 4 });
        let st = read!(u8 from chip ST);
        assert!(st == 7);

        chip.execute(OpCode::ReadDelay { to: 3 });
        let v3 = read!(u8 from chip Vs at 3);
        assert!(v3 == 7);

        chip.execute(OpCode::SetV { reg: 2, to: 8 });
        let v2 = read!(u8 from chip Vs at 2);
        assert!(v2 == 8);

        chip.execute(OpCode::IncV { reg: 2, by: 8 });
        let v2 = read!(u8 from chip Vs at 2);
        assert!(v2 == 16);

        // copy!(&[4] => chip KEY);
        // chip.execute(OpCode::ReadKey { to: 1 });
        // let v1 = read!(u8 from chip Vs at 1);
        // assert!(v1 == 4);

        const VAL: u8 = 0b11110000;
        copy!(&[VAL] => chip Vs at 9);
        chip.execute(OpCode::GetRand { reg: 9, mask: !VAL });
        let v9 = read!(u8 from chip Vs at 9);
        assert!(v9 != VAL);
    }

    // TODO Draw sprite from I at VX VY with N lines; VF = some 1 got turned off
    // TODO BCD of VX at I

    // Save V0..VX to I
    // Load V0..VX from I
    #[test]
    fn save_load() {
        let mut chip = Chip8::default();
        copy!(&[0x77] => chip Vs at 7);
        copy!(&[0x44] => chip Vs at 4);
        copy!(&(INITIAL_PC).to_be_bytes() => chip I);
        copy!(&[0x55] => chip Memory at 5);

        chip.execute(OpCode::SaveRegs { upto: 4 });
        let i = read!(u16 from chip I);
        assert!(i == INITIAL_PC + 4 + 1);

        let M04 = read!(u8 from chip Memory at 4);
        assert!(M04 == 0x44);

        copy!(&(INITIAL_PC).to_be_bytes() => chip I);
        copy!(&[0x33] => chip Memory at 3);
        chip.execute(OpCode::LoadRegs { upto: 7 });

        let i = read!(u16 from chip I);
        assert!(i == INITIAL_PC + 7 + 1);

        let v3 = read!(u8 from chip Vs at 3);
        assert!(v3 == 0x33);

        let v7 = read!(u8 from chip Vs at 7);
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
