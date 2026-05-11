mod memops;
mod opcode;

use std::u8;

use anyhow::{Context, Result};

use memops::*;

pub use memops::{ByteArray, Region};
pub use opcode::OpCode;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone)]
pub struct Chip8(pub(crate) [u8; 0x1000]);

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
pub enum PostExecute {
    Next,
    Wait,
    UpdateDt,
    UpdateSt,
}

impl Chip8 {
    pub fn new() -> Result<Self> {
        let mut cero = Self([0; _]);
        cero.write(0x200u16, PC).context("cannot write to mem")?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("Cannot access time")
            .as_millis() as u64;

        cero.write(now, Time).context("cannot write to mem")?;

        Ok(cero)
    }
    pub const INITIAL_PC: u16 = 0x200;
    /// Add current PC to top of stack
    fn push(&mut self) -> Result<()> {
        let sp: u8 = self.read(SP)?;
        assert!(sp < 16, "sp over bounds {}", sp);

        let pc: u16 = self.read(PC)?;
        self.write(pc, Stack + sp)?;

        self.write(sp + 2, SP)?;

        Ok(())
    }
    /// Pop from top of stack into PC
    fn pop(&mut self) -> Result<u16> {
        let sp: u8 = ByteArray::<u8>::read(self, SP)?;
        assert!(sp != 0, "sp under bounds");

        let sp = sp - 2;
        self.write(sp, SP)?;
        self.read(Stack + sp)
    }
    /// Load a program
    pub fn load_program<'a>(&'a mut self, program: &[u8]) -> Result<()> {
        self.0[(Memory as usize)..(Memory as usize + program.len())].copy_from_slice(program);

        Ok(())
    }
    /// Load to data section
    pub fn load_data(&mut self, data: &[u8]) -> Result<()> {
        self.0[(Data as usize)..(Data as usize + data.len())].copy_from_slice(data);
        Ok(())
    }
    /// Updates the keys when passing a bit map of the pressed keys
    pub fn load_key(&mut self, new_keys: u16) -> Result<()> {
        let old_keys: u16 = self.read(Keys)?;
        let newly_pressed = !old_keys & new_keys;

        let last_pressed = newly_pressed.trailing_zeros().try_into().unwrap_or(u8::MAX);

        self.write(last_pressed, LastKey)?;
        self.write(newly_pressed, Keys)?;

        Ok(())
    }

    /// Advance PC to the next instruction
    fn next_instruction(&mut self) -> Result<()> {
        let next = ByteArray::<u16>::read(self, PC)? + 2;
        self.write(next, PC)?;

        Ok(())
    }

    fn execute(&mut self, opcode: OpCode) -> Result<PostExecute> {
        use OpCode::*;
        match opcode {
            NoOp { .. } => (),
            Clear => self.write([0u8; Display.size()], Display)?,
            Draw { x, y, size } => {
                let (x, y): (u8, u8) = (self.read(Vs + x)?, self.read(Vs + y)?);
                let (x, y) = (x % 64, y % 32);
                let i: u16 = self.read(I)?;

                for n in 0..size {
                    let slice: u8 = self.read(i + n as u16)?;
                    let sprite = ((slice as u64) << (64 - 8)) >> (x as u32);

                    let (line_start, overflowed) = (y + n).overflowing_mul(8);

                    if overflowed {
                        // Sprites are cliped if out of screen
                        continue;
                    }

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
            }
            Call { at } => {
                self.push()?;
                self.write(at, PC)?;
            }
            JumpReg { by } => {
                let v0: u8 = self.read(Vs + 0)?;
                let sum = v0 as u16 + by;
                self.write(sum, PC)?;
            }
            SkipEqK { reg, val } => {
                let vx: u8 = self.read(Vs + reg)?;
                if vx == val {
                    self.next_instruction()?;
                }
            }
            SkipNotEqK { reg, val } => {
                let vx: u8 = self.read(Vs + reg)?;
                if vx != val {
                    self.next_instruction()?;
                }
            }
            SkipEq { a, b } => {
                let (va, vb): (u8, u8) = (self.read(Vs + a)?, self.read(Vs + b)?);
                if va == vb {
                    self.next_instruction()?;
                }
            }
            SkipNotEq { a, b } => {
                let (va, vb): (u8, u8) = (self.read(Vs + a)?, self.read(Vs + b)?);
                if va != vb {
                    self.next_instruction()?;
                }
            }
            SetV { reg, to } => self.write(to, Vs + reg)?,
            IncV { reg, by } => {
                let vx: u8 = self.read(Vs + reg)?;
                self.write(vx.wrapping_add(by), Vs + reg)?;
            }
            SetI { to } => self.write(to, I)?,
            GetRand { reg, mask } => self.write(rand::random::<u8>() & mask, Vs + reg)?,
            SkipPressed { reg } => {
                let key: u8 = self.read(Vs + reg)?;
                let pressed: u16 = self.read(Keys)?;
                if ((pressed >> key) & 1) == 1 {
                    self.next_instruction()?;
                }
            }
            SkipNotPressed { reg } => {
                let key: u8 = self.read(Vs + reg)?;
                let pressed: u16 = self.read(Keys)?;
                if ((pressed >> key) & 1) != 1 {
                    self.next_instruction()?;
                }
            }
            ReadKey { to } => {
                let keys: u8 = self.read(Keys)?;
                if keys == 0 {
                    return Ok(PostExecute::Wait);
                }
                let key: u8 = self.read(LastKey)?;
                self.write(key, Vs + to)?;
            }
            ReadDelay { to } => {
                let dt: u8 = self.read(DT)?;
                self.write(dt, Vs + to)?;
            }
            SetDelay { with } => {
                let vx: u8 = self.read(Vs + with)?;
                self.write(vx, DT)?;
                return Ok(PostExecute::UpdateDt);
            }
            SetSound { with } => {
                let vx: u8 = self.read(Vs + with)?;
                self.write(vx, ST)?;
                return Ok(PostExecute::UpdateSt);
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
                self.unchecked_write(res, i)?;

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

                let bcd = [
                    (of / 100) % 10,
                    (of / 10) % 10,
                    (of / 1) % 10, //
                ];

                self.unchecked_write(bcd, i)?;
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
                self.write(!overflowed as u8, Vs + 0xf)?;
            }
            ShiftR { a, b } => {
                let vb: u8 = self.read(Vs + b)?;
                self.write(vb >> 1, Vs + a)?;
                self.write(vb & 1, Vs + 0xf)?;
            }
            SubN { a, b } => {
                let (va, vb) = (self.read(Vs + a)?, self.read(Vs + b)?);
                let (res, overflowed) = u8::overflowing_sub(vb, va);
                self.write(res, Vs + a)?;
                self.write(!overflowed as u8, Vs + 0xf)?;
            }
            ShiftL { a, b } => {
                let vb: u8 = self.read(Vs + b)?;
                self.write(vb << 1, Vs + a)?;
                self.write(vb >> 7, Vs + 0xf)?;
            }
        }

        Ok(PostExecute::Next)
    }
    /// Ticks the timers
    pub fn step_timers(&mut self) -> Result<()> {
        let prev: u64 = self.read(Time)?;

        let current = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis() as u64;

        let current_duration = std::time::Duration::from_millis(current);
        let prev_duration = std::time::Duration::from_millis(prev);

        const _60HZ: u64 = 1000 / 60;

        let difference = (current_duration - prev_duration).as_millis() as u64;
        let decrements = (difference / _60HZ).try_into().unwrap_or(u8::MAX);
        let reminder = difference % _60HZ;

        // Account for step being called in between reductions
        self.write(current - reminder, Time)?;

        let dt: u8 = self.read(DT)?;
        let st: u8 = self.read(ST)?;

        self.write(dt.saturating_sub(decrements), DT)?;
        self.write(st.saturating_sub(decrements), ST)?;

        Ok(())
    }
    /// Executes the opcode at pc
    pub fn step(&mut self) -> Result<PostExecute> {
        let pc: u16 = self.read(PC)?;
        assert!(pc < 0x1000, "Invalid pc: {pc:04x} out of bounds");

        let fragment: u16 = self.read(pc)?;
        let opcode = fragment.into();

        self.next_instruction()?;
        self.step_timers()?;

        let result = self.execute(opcode)?;

        Ok(result)
    }
}
