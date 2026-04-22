pub type Reg = u8;
pub type Adr = [u8; 2];
use super::memops::stitch;

// TODO is it ok to use more mem than needed? opcodes are 16-bits
// Reg should be a u4
// Adr should be a u12
#[derive(Debug, Clone, Copy)]
pub enum OpCode {
    NoOp { val: Adr },                 // 0NNN | Call | noop
    Clear,                             // 00E0 | Disp | clear screen
    Draw { x: Reg, y: Reg, size: u8 }, // DXYN | Disp | draw sprite at VX,VY with height N from I (wraps)
    Return,                            // 00EE | Flow | return (pop into I)
    Jump { to: Adr },                  // 1NNN | Flow | jump (set PC to NNN)
    Call { at: Adr },                  // 2NNN | Flow | call subroutine (push and set I to NNN)
    JumpReg { by: Adr },               // BNNN | Flow | PC = V0 + NNN
    SkipEqK { reg: Reg, val: u8 },     // 3XNN | Cond | PC + 1 if VX == NN
    SkipNotEqK { reg: Reg, val: u8 },  // 4XNN | Cond | PC + 1 if VX != NN
    SkipEq { a: Reg, b: Reg },         // 5XY0 | Cond | PC + 1 if VX == VY
    SkipNotEq { a: Reg, b: Reg },      // 9XY0 | Cond | PC + 1 if VX != VY
    SetV { reg: Reg, to: u8 },         // 6XNN | Cons | VX = NN
    IncV { reg: Reg, by: u8 },         // 7XNN | Cons | VX += NN (doesnt flag overflow)
    SetI { to: Adr },                  // ANNN | Load | I = NNN
    GetRand { reg: Reg, mask: u8 },    // CXNN | Rand | VX = rand() & NN
    SkipPressed { key: Reg },          // EX9E | Read | PC + 1 if key() == VX
    SkipNotPressed { key: Reg },       // EXA1 | Read | PC + 1 if key() != VX
    ReadKey { to: Reg },               // FX0A | Read | VX = key() (blocking)
    ReadDelay { to: Reg },             // FX07 | Time | VX = DT
    SetDelay { with: Reg },            // FX15 | Time | DT = VX
    SetSound { with: Reg },            // FX18 | Time | ST = VX
    IncI { with: Reg },                // FX1E | Memo | I += VX
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

impl OpCode {
    pub fn as_nibbles(opcode: &[u8; 2]) -> [u8; 4] {
        [
            opcode[0] >> 4,
            opcode[0] & 0xf,
            opcode[1] >> 4,
            opcode[1] & 0xf,
        ]
    }

    pub fn parse_program(program: &[u8]) -> Vec<OpCode> {
        let (chunks, []) = program.as_chunks() else {
            panic!("Program opcodes are unaligned")
        };

        chunks.into_iter().map(OpCode::from).collect()
    }
}
impl From<u16> for OpCode {
    fn from(value: u16) -> Self {
        let [hi, lo] = value.to_be_bytes();
        Into::into(&[hi, lo])
    }
}
impl From<&[u8; 2]> for OpCode {
    fn from(value: &[u8; 2]) -> Self {
        let [a, b, c, d] = OpCode::as_nibbles(value);

        use OpCode::*;
        match (a, b, c, d) {
            (0, 0, 0xe, 0) => Clear,
            (0xd, x, y, size) => Draw { x, y, size },
            (0, 0, 0xe, 0xe) => Return,
            (0, b, c, d) => {
                // panic!("NoOp - not really a panic, but sus");
                NoOp {
                    val: stitch![0, b, c, d],
                }
            }
            (0x1, b, c, d) => Jump {
                to: stitch![0, b, c, d],
            },
            (0x2, b, c, d) => Call {
                at: stitch![0, b, c, d],
            },
            (0xb, b, c, d) => JumpReg {
                by: stitch![0, b, c, d],
            },
            (0x3, reg, c, d) => SkipEqK {
                reg,
                val: stitch![c, d],
            },
            (0x4, reg, c, d) => SkipNotEqK {
                reg,
                val: stitch![c, d],
            },
            (0x5, a, b, 0) => SkipEq { a, b },
            (0x9, a, b, 0) => SkipNotEq { a, b },
            (0x6, reg, c, d) => SetV {
                reg,
                to: stitch![c, d],
            },
            (0x7, reg, c, d) => IncV {
                reg,
                by: stitch![c, d],
            },
            (0xa, b, c, d) => SetI {
                to: stitch![0, b, c, d],
            },
            (0xc, reg, c, d) => GetRand {
                reg,
                mask: stitch![c, d],
            },
            (0xe, key, 0x9, 0xe) => SkipPressed { key },
            (0xe, key, 0xa, 0x1) => SkipNotPressed { key },
            (0xf, to, 0, 0xa) => ReadKey { to },
            (0xf, to, 0, 0x7) => ReadDelay { to },
            (0xf, with, 0x1, 0x5) => SetDelay { with },
            (0xf, with, 0x1, 0x8) => SetSound { with },
            (0xf, with, 0x1, 0xe) => IncI { with },
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
            other => {
                // panic!("Unknown opcode {:x?}", other);
                NoOp {
                    val: stitch![0, 0, 0, 0],
                }
            }
        }
    }
}
