use super::opcode::*;
use rand::random;

use super::memops::*;

#[derive(Debug, Clone)]
pub struct Chip8(pub(crate) [u8; 0x1000]);

impl Default for Chip8 {
    fn default() -> Self {
        Self([0; _])
    }
}

impl Chip8 {
    fn push(&mut self, adr: Adr) {
        assert!(self[SP] <= 16, "sp over bounds {}", self[SP]);

        let sp = self[SP];

        copy!(&adr => self Stack at sp as usize);

        self[SP] += 1;
    }
    fn pop(&mut self) -> [u8; 2] {
        assert!(self[SP] > 0, "sp under bounds");
        self[SP] -= 1;
        let sp = self[SP];
        [self[Stack + sp], self[Stack + sp + 1]]
    }
    pub fn load_program(&mut self, program: &[u8]) {
        copy!(program => self Memory);
    }
    pub fn load_data(&mut self, data: &[u8]) {
        copy!(data => self Data);
    }
    // TODO add guards
    pub fn step(&mut self) {
        use OpCode::*;
        assert!(self[PC] < Memory.size() as _, "pc out of bounds");

        let pc = self[PC] as usize;
        let fragment = self[Memory + (pc..pc + 2)].as_array().unwrap();
        let opcode = fragment.into();

        println!("{:?} {:x?}", opcode, OpCode::as_nibbles(fragment));

        match opcode {
            NoOp { .. } => (),
            Clear => copy!(&[0; Display.size()] => self Display),
            Draw { x, y, size } => {
                // x: 0 - 64, y: 0 - 32
                // size: 1-15

                assert!(x < 64, "x out of range of screen");
                assert!(y < 32, "y out of range of screen");
                assert!(size < 16, "size out of range");

                let (x, y) = (self[V + x], self[V + y]);
                let i = read!(u16 from self I);

                for n in 0..size {
                    let sprite = (self[i + n as u16] as u64) << (64 - 8) >> x;
                    let line_start = y + n * 8;
                    let screen = read!(u64 from self Display at line_start);

                    let xor = sprite ^ screen;
                    self[V + 0xf] = ((screen & !xor) == 0) as u8;
                    copy!(&xor.to_le_bytes() => self Display at line_start as usize);
                }
            }
            Return => {
                let adr = self.pop();
                copy!(&adr => self PC);
            }
            Jump { to } => copy!(&to => self PC),
            Call { at } => {
                self.push([self[PC], self[PC + 1]]);
                copy!(&at => self PC);
            }
            Advance { by } => {
                let sum = self[V + 0] as u16 + u16::from_be_bytes(by);
                copy!(&sum.to_be_bytes() => self PC);
            }
            SkipEqK { reg, val } => {
                if self[V + reg] == val {
                    self[PC] += 2;
                }
            }

            SkipNotEqK { reg, val } => {
                if self[V + reg] != val {
                    self[PC] += 2;
                }
            }
            SkipEq { a, b } => {
                if self[V + a] == self[V + b] {
                    self[PC] += 2;
                }
            }
            SkipNotEq { a, b } => {
                if self[V + a] != self[V + b] {
                    self[PC] += 2;
                }
            }
            SetV { reg, to } => self[V + reg] = to,
            IncrementV { reg, by } => self[V + reg] = self[V + reg].wrapping_add(by),
            SetI { to } => copy!(&to => self I),
            GetRand { reg, mask } => self[V + reg] = random::<u8>() & mask,
            SkipPressed { key } => {
                if self[KEY] == key {
                    self[PC] += 2;
                }
            }
            SkipNotPressed { key } => {
                if self[KEY] != key {
                    self[PC] += 2;
                }
            }
            ReadKey { to } => todo!("set up blocking"),
            ReadDelay { to } => self[V + to] = self[DT],
            SetDelay { with } => self[DT] = self[V + with],
            SetSound { with } => self[ST] = self[V + with],
            OffsetI { with } => {
                let sum = self[V + with] as u16 + u16::from_be_bytes([self[I], self[I + 1]]);
                copy!(&sum.to_be_bytes() => self I)
            }
            GetFontSprite { of } => {
                let adr = self[V + of] as u16 * 5 * 8;
                copy!(&adr.to_be_bytes() => self I);
            }
            SaveRegs { upto } => {
                // TODO maybe i is off by 1
                let i = u16::from_be_bytes([self[I], self[I + 1]]);
                let regs = self[V + (0..upto as usize)].to_owned();
                copy!(&regs => self Memory at i as usize);

                let sum = i + upto as u16;
                copy!(&sum.to_be_bytes() => self I);
            }
            LoadRegs { upto } => {
                let i = u16::from_be_bytes([self[I], self[I + 1]]);
                let regs = self[Memory + (0..upto as usize)].to_owned();
                copy!(&regs=> self V at i as usize);

                let sum = i + upto as u16;
                copy!(&sum.to_be_bytes() => self I);
            }
            BCD { of } => {
                let of = self[V + of];
                copy!(&[
                    (of / 100) % 10,
                    (of / 10) % 10,
                    (of / 1) % 10,
                ] => self Memory );
            }
            Assign { a, to } => self[V + a] = self[V + to],
            Or { a, b } => self[V + a] |= self[V + b],
            And { a, b } => self[V + a] &= self[V + b],
            Xor { a, b } => self[V + a] ^= self[V + b],
            Add { a, b } => {
                let res = self[V + a] as u16 + self[V + b] as u16;
                self[V + a] = res as u8;
                self[V + 0xf] = (res & 0xff00 == 0) as u8;
            }
            Sub { a, b } => {
                let (a_val, b_val) = (self[V + a], self[V + b]);
                self[V + a] = a_val.wrapping_sub(b_val);
                self[V + 0xf] = (a_val < b_val) as u8;
            }
            ShiftR { a, b } => {
                self[V + 0xf] = self[V + b] & 0x1;
                self[V + a] = self[V + b] >> 1;
            }
            SubN { a, b } => {
                let (a_val, b_val) = (self[V + a], self[V + b]);
                self[V + a] = b_val.wrapping_sub(a_val);
                self[V + 0xf] = (b_val < a_val) as u8;
            }
            ShiftL { a, b } => {
                self[V + 0xf] = self[V + b] >> 7;
                self[V + a] = self[V + b] << 1;
            }
        }

        self[PC] += 2;
    }
}
