fn main() {
    println!("Hello, world!");
}

// TODO maybe make it more compact?
// stitch!(4-bit [ a, b, c ] as u16)
macro_rules! stitch {
    ([
        $a: ident $b: ident $c: ident $d: ident
    ] as $target: ty) => {
        (((((($a << NIBBLE) + $b) << NIBBLE) + $c) << NIBBLE) + $d) as $target
    };
    ([
        $a: ident $b: ident $c: ident
    ] as $target: ty) => {
        (((($a << NIBBLE) + $b) << NIBBLE) + $c) as $target
    };
    ([
        $a: ident $b: ident
    ] as $target: ty) => {
        (($a << NIBBLE) + $b) as $target
    };
}
macro_rules! r {
    ($target: ident.v [$reg: expr]) => {
        $target.v[$reg as usize]
    };
}

const NIBBLE: usize = 4;
const DSP: usize = 0x1000 - 0xf00;
const REG: usize = 0xed0 - 0xec0;
const STK: usize = (0xec0 - 0xea0) / 2;
const MEM: usize = 0xea0 - 0x200;
const DAT: usize = 0x200 - 0x000;

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
            pc: Default::default(),
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
    // TODO add guards
    pub fn step(&mut self) {
        use OpCode::*;

        let opcode = self.memory[self.pc as usize..(self.pc + 1) as usize].into();
        match opcode {
            NoOp { .. } => (),
            Clear => self.display = [0; _],
            Draw { x, y, size } => {
                let (x, y) = (r!(self.v[x]), r!(self.v[y]));
                todo!();
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
                    self.pc += 1;
                }
            }

            SkipNotEqK { reg, val } => {
                if self.v[reg as usize] != val {
                    self.pc += 1;
                }
            }
            SkipEq { a, b } => {
                if self.v[a as usize] == self.v[b as usize] {
                    self.pc += 1;
                }
            }
            SkipNotEq { a, b } => {
                if self.v[a as usize] != self.v[b as usize] {
                    self.pc += 1;
                }
            }
            SetV { reg, to } => self.v[reg as usize] = to,
            IncrementV { reg, by } => self.v[reg as usize] = self.v[reg as usize].wrapping_add(by),
            SetI { to } => self.i = to,
            GetRand { reg, mask } => todo!("set up rand"),
            SkipPressed { key } => {
                if self.key == key {
                    self.pc += 1;
                }
            }
            SkipNotPressed { key } => {
                if self.key != key {
                    self.pc += 1;
                }
            }
            ReadKey { to } => todo!("set up bloking"),
            ReadDelay { to } => self.v[to as usize] = self.dt,
            SetDelay { with } => self.dt = self.v[with as usize],
            SetSound { with } => self.st = self.v[with as usize],
            OffsetI { with } => self.i += self.v[with as usize] as u16,
            GetFontSprite { of } => todo!("set font table"),
            SaveRegs { upto } => {
                for x in 0..upto {
                    self.memory[(self.i + x as u16) as usize] = self.v[x as usize];
                }
                self.i += upto as u16 + 1;
            }
            LoadRegs { upto } => {
                for x in 0..upto {
                    self.v[x as usize] = self.memory[(self.i + x as u16) as usize];
                }
                self.i += upto as u16 + 1;
            }
            BCD { of } => {
                let range = self.i as usize..(self.i as usize + 2);
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
                self.v[0xf] = if res & 0xff00 == 0 { 0 } else { 1 };
            }
            Sub { a, b } => {
                let (a, b) = (r!(self.v[a]), r!(self.v[b]));
                r!(self.v[a]) = a.wrapping_sub(b);
                self.v[0xf] = if a < b { 1 } else { 0 };
            }
            ShiftR { a, b } => {
                self.v[0xf] = r!(self.v[b]) & 0x1;
                r!(self.v[a]) = r!(self.v[b]) >> 1;
            }
            SubN { a, b } => {
                let (a, b) = (r!(self.v[a]), r!(self.v[b]));
                r!(self.v[a]) = b.wrapping_sub(a);
                self.v[0xf] = if b < a { 1 } else { 0 };
            }
            ShiftL { a, b } => {
                self.v[0xf] = r!(self.v[b]) >> 7;
                r!(self.v[a]) = r!(self.v[b]) << 1;
            }
        }
    }
}

pub const FONT: [[u8; 5]; 16] = [
    [0xF0, 0x90, 0x90, 0x90, 0xF0], // 0
    [0x20, 0x60, 0x20, 0x20, 0x70], // 1
    [0xF0, 0x10, 0xF0, 0x80, 0xF0], // 2
    [0xF0, 0x10, 0xF0, 0x10, 0xF0], // 3
    [0x90, 0x90, 0xF0, 0x10, 0x10], // 4
    [0xF0, 0x80, 0xF0, 0x10, 0xF0], // 5
    [0xF0, 0x80, 0xF0, 0x90, 0xF0], // 6
    [0xF0, 0x10, 0x20, 0x40, 0x40], // 7
    [0xF0, 0x90, 0xF0, 0x90, 0xF0], // 8
    [0xF0, 0x90, 0xF0, 0x10, 0xF0], // 9
    [0xF0, 0x90, 0xF0, 0x90, 0x90], // A
    [0xE0, 0x90, 0xE0, 0x90, 0xE0], // B
    [0xF0, 0x80, 0x80, 0x80, 0xF0], // C
    [0xE0, 0x90, 0x90, 0x90, 0xE0], // D
    [0xF0, 0x80, 0xF0, 0x80, 0xF0], // E
    [0xF0, 0x80, 0xF0, 0x80, 0x80], // F
];

// TODO is it ok to use more mem than needed? opcodes are 16-bits
// Reg should be a u4
// Adr should be a u12
type Reg = u8;
type Adr = u16;
#[derive(Debug, Clone, Copy)]
pub enum OpCode {
    NoOp { val: Adr },                 // 0NNN | Call | noop
    Clear,                             // 00E0 | Disp | clear screen
    Draw { x: Reg, y: Reg, size: u8 }, // DXYN | Disp | draw sprite at VX,VY with height N from I (wraps)
    Return,                            // 00EE | Flow | return (pop into I)
    Jump { to: Adr },                  // 1NNN | Flow | jump (set PC to NNN)
    Call { at: Adr },                  // 2NNN | Flow | call subroutine (push and set I to NNN)
    Advance { by: u16 },               // BNNN | Flow | PC = V0 + NNN
    SkipEqK { reg: Reg, val: u8 },     // 3XNN | Cond | PC + 1 if VX == NN
    SkipNotEqK { reg: Reg, val: u8 },  // 4XNN | Cond | PC + 1 if VX != NN
    SkipEq { a: Reg, b: Reg },         // 5XY0 | Cond | PC + 1 if VX == VY
    SkipNotEq { a: Reg, b: Reg },      // 9XY0 | Cond | PC + 1 if VX != VY
    SetV { reg: Reg, to: u8 },         // 6XNN | Cons | VX = NN
    IncrementV { reg: Reg, by: u8 },   // 7XNN | Cons | VX += NN (doesnt flag overflow)
    SetI { to: u16 },                  // ANNN | Load | I = NNN
    GetRand { reg: Reg, mask: u8 },    // CXNN | Rand | VX = rand() & NN
    SkipPressed { key: Reg },          // EX9E | Read | PC + 1 if key() == VX
    SkipNotPressed { key: Reg },       // EXA1 | Read | PC + 1 if key() != VX
    ReadKey { to: Reg },               // FX0A | Read | VX = key() (blocking)
    ReadDelay { to: Reg },             // FX07 | Time | VX = DT
    SetDelay { with: Reg },            // FX15 | Time | DT = VX
    SetSound { with: Reg },            // FX18 | Time | ST = VX
    OffsetI { with: Reg },             // FX1E | Memo | I += VX
    GetFontSprite { of: Reg },         // FX29 | Memo | I = sprite_addr[VX]
    SaveRegs { upto: Reg },            // FX55 | Memo | dump data from V0 to VX regs to Mem[I]
    LoadRegs { upto: Reg },            // FX65 | Memo | load data regs V0..VX from I
    BCD { of: Reg },                   // FX33 | Repr | ex: VX = 234, I[0] = 2, I[1] = 3, I[2] = 4

    // --- ALU --- flag VF on over/under flow
    Assign { a: Reg, to: Reg }, // 8XY0 | Set
    Or { a: Reg, b: Reg },      // 8XY1 | Or
    And { a: Reg, b: Reg },     // 8XY2 | And
    Xor { a: Reg, b: Reg },     // 8XY3 | Xor
    Add { a: Reg, b: Reg },     // 8XY4 | Add
    Sub { a: Reg, b: Reg },     // 8XY5 | Sub
    ShiftR { a: Reg, b: Reg },  // 8XY6 | SftR
    SubN { a: Reg, b: Reg },    // 8XY7 | Sub * -1
    ShiftL { a: Reg, b: Reg },  // 8XYE | SftL
}

impl From<&[u8]> for OpCode {
    fn from(value: &[u8]) -> Self {
        assert!(value.len() == 2, "bad opcode");

        let [a, b, c, d] = [
            value[0] << 4,
            value[0] & 0xff,
            value[1] << 4,
            value[1] & 0xff,
        ];
        use OpCode::*;
        match (a, b, c, d) {
            (0, 0, 0xe, 0) => Clear,
            (0xd, x, y, size) => Draw { x, y, size },
            (0, 0, 0xe, 0xe) => Return,
            (0, b, c, d) => NoOp {
                val: stitch!([b c d] as u16),
            },
            (0x1, b, c, d) => Jump {
                to: stitch!([b c d] as u16),
            },
            (0x2, b, c, d) => Call {
                at: stitch!([b c d] as u16),
            },
            (0xb, b, c, d) => Advance {
                by: stitch!([b c d] as u16),
            },
            (0x3, reg, c, d) => SkipEqK {
                reg,
                val: stitch!([c d] as u8),
            },
            (0x4, reg, c, d) => SkipNotEqK {
                reg,
                val: stitch!([c d] as u8),
            },
            (0x5, a, b, 0) => SkipEq { a, b },
            (0x9, a, b, 0) => SkipNotEq { a, b },
            (0x6, reg, c, d) => SetV {
                reg,
                to: stitch!([c d] as u8),
            },
            (0x7, reg, c, d) => IncrementV {
                reg,
                by: stitch!([c d] as u8),
            },
            (0xa, b, c, d) => SetI {
                to: stitch!([b c d] as u16),
            },
            (0xc, reg, c, d) => GetRand {
                reg,
                mask: stitch!([c d] as u8),
            },
            (0xe, key, 0x9, 0xe) => SkipPressed { key },
            (0xe, key, 0xa, 0x1) => SkipNotPressed { key },
            (0xf, to, 0, 0xa) => ReadKey { to },
            (0xf, to, 0, 0x7) => ReadDelay { to },
            (0xf, with, 0x1, 0x5) => SetDelay { with },
            (0xf, with, 0x1, 0x8) => SetSound { with },
            (0xf, with, 0x1, 0xe) => OffsetI { with },
            (0xf, of, 0x2, 0x9) => GetFontSprite { of },
            (0xf, upto, 0x5, 0x5) => SaveRegs { upto },
            (0xf, upto, 0x6, 0x5) => LoadRegs { upto },
            (0xf, of, 0x3, 0x3) => BCD { of },
            (0x8, a, to, 0) => Assign { a, to },
            (0x8, a, b, 1) => Or { a, b },
            (0x8, a, b, 2) => And { a, b },
            (0x8, a, b, 3) => Xor { a, b },
            (0x8, a, b, 4) => Add { a, b },
            (0x8, a, b, 5) => Sub { a, b },
            (0x8, a, b, 6) => ShiftR { a, b },
            (0x8, a, b, 7) => SubN { a, b },
            (0x8, a, b, 0xe) => ShiftL { a, b },
            unknown => {
                eprintln!("Unknown opcode {:#x?}", unknown);
                NoOp { val: 0 }
            }
        }
    }
}

/*
# Chip-8

Memory: 4k, 4096 memory locations (0x1000) * 8bits

    0x1000 | End
    0x0F00 | Display
    0x0EA0 | Call stack, internal use and others
    0x0200 | Free
    0x0000 | Data

Registers:

    8-bit  v0..vf (vf doubles as a flag for some ops)
    16-bit I

    16-bit PC
    8-bit  SP

Stack:

    store return addresses for subroutines
    16 * 16-bit

Timers: 2 60hz

    delay DT rw
    sound ST w (beep if not 0)

Input:

    hex keyboard (16 keys 0..F)

Output:

    graphics 64*32 pixels monochrome
        sprites 8*1..15 xor'd to screen
        on pixel off'd VF is set to 1 else 0 (collision detection)
    sound beep if sound timer is not 0

Opcodes: 36 2-byte big-endian opcodes

    0NNN | Call | noop
    00E0 | Disp | clear screen
    DXYN | Disp | draw sprite at VX,VY with height N from I (wraps)
    00EE | Flow | return (pop into I)
    1NNN | Flow | jump (set I to NNN)
    2NNN | Flow | call subroutine (push and set I to NNN)
    BNNN | Flow | PC = V0 + NNN
    3XNN | Cond | PC + 1 if VX == NN
    4XNN | Cond | PC + 1 if VX != NN
    5XY0 | Cond | PC + 1 if VX == VY
    9XY0 | Cond | PC + 1 if VX != VY
    6XNN | Cons | VX = NN
    7XNN | Cons | VX += NN (doesnt flag overflow)
    ANNN | Load | I = NNN
    CXNN | Rand | VX = rand() & NN
    EX9E | Read | PC + 1 if key() == VX
    EXA1 | Read | PC + 1 if key() != VX
    FX0A | Read | Vx = key() (blocking)
    FX07 | Time | VX = DT
    FX15 | Time | DT = VX
    FX18 | Time | ST = VX
    FX1E | Memo | I += VX
    FX29 | Memo | I = sprite_addr[VX]
    FX55 | Memo | dump data regs at I
    FX65 | Memo | load data regs from I
    FX33 | Repr | ex: VX = 234, I[0] = 2, I[1] = 3, I[2] = 4

    --- ALU --- flag VF on over/under flow
    8XY0 | Set
    8XY1 | Or
    8XY2 | And
    8XY3 | Xor
    8XY4 | Add
    8XY5 | Sub
    8XY6 | SftR
    8XY7 | Sub * -1
    8XYE | SftL

Font:
    4*5 bit 0..F padded to 8*5
*/
