use super::opcode::*;
use rand::random;

pub const DSP: usize = 0x1000 - 0xf00;
pub const REG: usize = 0xed0 - 0xec0;
pub const STK: usize = (0xec0 - 0xea0) / 2;
pub const MEM: usize = 0xea0 - 0x200;
pub const DAT: usize = 0x200 - 0x000;

// TODO consider impl Index for Chip8 -> self[Memory + reg] = ...
// Better control over access, a place to add checks
#[derive(Debug, Clone)]
pub struct Chip8 {
    // 0x1000 | End
    pub display: [u8; DSP], // 0x0F00 | Display (256)
    pub key: u8,            // 0x0ED7 | Key (1)
    pub st: u8,             // 0x0ED6 | Sound timer (1)
    pub dt: u8,             // 0x0ED5 | Delay timer (1)
    pub sp: u8,             // 0x0ED4 | Stack pointer (1)
    pub pc: u16,            // 0x0ED2 | Program counter (2)
    pub i: u16,             // 0x0ED0 | Address register (2)
    pub v: [u8; REG],       // 0x0EC0 | V Registers (16)
    pub stack: [u16; STK],  // 0x0EA0 | Call stack (32)
    pub memory: [u8; MEM],  // 0x0200 | Free (3232)
    pub data: [u8; DAT],    // 0x0000 | Internal Data (200)
}

impl Default for Chip8 {
    fn default() -> Self {
        Self {
            display: [0; _],
            key: Default::default(),
            st: Default::default(),
            dt: Default::default(),
            sp: Default::default(),
            pc: 0,
            i: Default::default(),
            v: Default::default(),
            stack: Default::default(),
            memory: [0; _],
            data: [0; _],
        }
    }
}

impl Chip8 {
    fn push(&mut self, adr: Adr) {
        assert!(self.sp < 16, "sp over bounds");
        self.stack[self.sp as usize] = adr;
        self.sp += 1;
    }
    fn pop(&mut self) -> Adr {
        assert!(self.sp > 0, "sp under bounds");
        self.sp -= 1;
        self.stack[self.sp as usize]
    }
    pub fn load_program(&mut self, program: &[u8]) {
        self.memory[0..program.len()].copy_from_slice(program);
    }
    pub fn load_data(&mut self, data: &[u8]) {
        self.data[0..data.len()].copy_from_slice(data);
    }
    // TODO add guards
    pub fn step(&mut self) {
        use OpCode::*;
        assert!(self.pc < self.memory.len() as _, "pc out of bounds");

        let pc = self.pc as usize;
        let fragment = self.memory[pc..pc + 2].as_array().unwrap();
        let opcode = fragment.into();

        println!("{:?} {:x?}", opcode, OpCode::as_nibbles(fragment));

        match opcode {
            NoOp { .. } => (),
            Clear => self.display = [0; _],
            Draw { x, y, size } => {
                let (x, y) = (r!(self.v[x]) as usize, r!(self.v[y]) as usize);
                for n in 0..size {
                    // 8 * 32
                    let sprite = 8 * (y + n as usize) + x;
                    let target = r!(self.display[sprite]);
                    let result = target ^ r!(self.memory[self.i + n as u16]);
                    self.v[0xf] = ((target & !result) == 0) as u8;
                    r!(self.display[sprite]) = result;
                }
            }
            Return => self.pc = self.pop(),
            Jump { to } => self.pc = to,
            Call { at } => {
                self.push(self.pc);
                self.pc = at;
            }
            Advance { by } => {
                self.pc = self.v[0] as u16 + by;
            }
            SkipEqK { reg, val } => {
                if self.v[reg as usize] == val {
                    self.pc += 2;
                }
            }

            SkipNotEqK { reg, val } => {
                if self.v[reg as usize] != val {
                    self.pc += 2;
                }
            }
            SkipEq { a, b } => {
                if self.v[a as usize] == self.v[b as usize] {
                    self.pc += 2;
                }
            }
            SkipNotEq { a, b } => {
                if self.v[a as usize] != self.v[b as usize] {
                    self.pc += 2;
                }
            }
            SetV { reg, to } => self.v[reg as usize] = to,
            IncrementV { reg, by } => self.v[reg as usize] = self.v[reg as usize].wrapping_add(by),
            SetI { to } => self.i = to,
            GetRand { reg, mask } => r!(self.v[reg]) = random::<u8>() & mask,
            SkipPressed { key } => {
                if self.key == key {
                    self.pc += 2;
                }
            }
            SkipNotPressed { key } => {
                if self.key != key {
                    self.pc += 2;
                }
            }
            ReadKey { to } => todo!("set up blocking"),
            ReadDelay { to } => self.v[to as usize] = self.dt,
            SetDelay { with } => self.dt = self.v[with as usize],
            SetSound { with } => self.st = self.v[with as usize],
            OffsetI { with } => self.i += self.v[with as usize] as u16,
            GetFontSprite { of } => self.i = r!(self.v[of]) as u16 * 5 * 8, // TODO maybe make a table...
            SaveRegs { upto } => {
                for x in 0..upto {
                    self.memory[(self.i + x as u16) as usize] = self.v[x as usize];
                }
                self.i += upto as u16;
            }
            LoadRegs { upto } => {
                for x in 0..upto {
                    self.v[x as usize] = self.memory[(self.i + x as u16) as usize];
                }
                self.i += upto as u16;
            }
            BCD { of } => {
                let range = self.i as usize..(self.i as usize + 3);
                let of = self.v[of as usize];
                self.memory[range].copy_from_slice(&[
                    (of / 100) % 10,
                    (of / 10) % 10,
                    (of / 1) % 10,
                ]);
            }
            Assign { a, to } => r!(self.v[a]) = r!(self.v[to]),
            Or { a, b } => r!(self.v[a]) |= r!(self.v[b]),
            And { a, b } => r!(self.v[a]) &= r!(self.v[b]),
            Xor { a, b } => r!(self.v[a]) ^= r!(self.v[b]),
            Add { a, b } => {
                let res = r!(self.v[a]) as u16 + r!(self.v[b]) as u16;
                r!(self.v[a]) = res as u8;
                self.v[0xf] = (res & 0xff00 == 0) as u8;
            }
            Sub { a, b } => {
                let (a_val, b_val) = (r!(self.v[a]), r!(self.v[b]));
                r!(self.v[a]) = a_val.wrapping_sub(b_val);
                self.v[0xf] = (a_val < b_val) as u8;
            }
            ShiftR { a, b } => {
                self.v[0xf] = r!(self.v[b]) & 0x1;
                r!(self.v[a]) = r!(self.v[b]) >> 1;
            }
            SubN { a, b } => {
                let (a_val, b_val) = (r!(self.v[a]), r!(self.v[b]));
                r!(self.v[a]) = b_val.wrapping_sub(a_val);
                self.v[0xf] = (b_val < a_val) as u8;
            }
            ShiftL { a, b } => {
                self.v[0xf] = r!(self.v[b]) >> 7;
                r!(self.v[a]) = r!(self.v[b]) << 1;
            }
        }

        self.pc += 2;
    }
}
