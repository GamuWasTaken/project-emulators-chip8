fn main() {
    println!("Hello, world!");
}

const DSP: usize = 0x1000 - 0xf00;
const REG: usize = 0xed0 - 0xec0;
const STK: usize = 0xec0 - 0xea0;
const MEM: usize = 0xea0 - 0x200;
const DAT: usize = 0x200 - 0x000;

struct Chip8 {
    // 0x1000 | End
    display: [u8; DSP], // 0x0F00 | Display
    st: u8,             // 0x0ED6 | Sound timer
    dt: u8,             // 0x0ED5 | Delay timer
    sp: u8,             // 0x0ED4 | Stack pointer
    pc: u16,            // 0x0ED2 | Program counter
    i: u16,             // 0x0ED0 | Address register
    v: [u8; REG],       // 0x0EC0 | V Registers
    stack: [u8; STK],   // 0x0EA0 | Call stack
    memory: [u8; MEM],  // 0x0200 | Free
    data: [u8; DAT],    // 0x0000 | Internal Data
}

const FONT: [[u8; 5]; 16] = [
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
enum OpCodes {
    NoOp,                            // 0NNN | Call | noop
    Clear,                           // 00E0 | Disp | clear screen
    Draw { x: u8, y: u8, size: u8 }, // DXYN | Disp | draw sprite at VX,VY with height N from I (wraps)
    Return,                          // 00EE | Flow | return (pop into I)
    Jump { to: u16 },                // 1NNN | Flow | jump (set PC to NNN)
    Call { at: u16 },                // 2NNN | Flow | call subroutine (push and set I to NNN)
    Advance { by: u16 },             // BNNN | Flow | PC = V0 + NNN
    SkipEqK { reg: u8, val: u8 },    // 3XNN | Cond | PC + 1 if VX == NN
    SkipNeqK { reg: u8, val: u8 },   // 4XNN | Cond | PC + 1 if VX != NN
    SkipEq { a: u8, b: u8 },         // 5XY0 | Cond | PC + 1 if VX == VY
    SkipNeq { a: u8, b: u8 },        // 9XY0 | Cond | PC + 1 if VX != VY
    SetV { reg: u8, to: u8 },        // 6XNN | Cons | VX = NN
    Increment { reg: u8, by: u8 },   // 7XNN | Cons | VX += NN (doesnt flag overflow)
    SetI { to: u16 },                // ANNN | Load | I = NNN
    GetRand { reg: u8, mask: u8 },   // CXNN | Rand | VX = rand() & NN
    SkipKeyPressed { key: u8 },      // EX9E | Read | PC + 1 if key() == VX
    SkipKeyNotPressed { key: u8 },   // EXA1 | Read | PC + 1 if key() != VX
    ReadKey { reg: u8 },             // FX0A | Read | Vx = key() (blocking)
    ReadDelay { reg: u8 },           // FX07 | Time | VX = DT
    SetDelay { reg: u8 },            // FX15 | Time | DT = VX
    SetSound { reg: u8 },            // FX18 | Time | ST = VX
    OffsetI {},                      // FX1E | Memo | I += VX
                                     // FX29 | Memo | I = sprite_addr[VX]
                                     // FX55 | Memo | dump data regs at I
                                     // FX65 | Memo | load data regs from I
                                     // FX33 | Repr | ex: VX = 234, I[0] = 2, I[1] = 3, I[2] = 4

                                     // --- ALU --- flag VF on over/under flow
                                     // 8XY0 | Set
                                     // 8XY1 | Or
                                     // 8XY2 | And
                                     // 8XY3 | Xor
                                     // 8XY4 | Add
                                     // 8XY5 | Sub
                                     // 8XY6 | SftR
                                     // 8XY7 | Sub * -1
                                     // 8XYE | SftL
}
