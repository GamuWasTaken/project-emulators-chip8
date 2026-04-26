use std::{
    thread::sleep,
    time::{Duration, Instant},
};

pub mod chip8;
pub mod opcode;

pub mod memops;

use chip8::*;
use memops::*;
use opcode::*;

fn main() {
    run();
}

fn run() {
    let mut chip = Chip8::default();

    let program = include_bytes!("./test.ch8");

    // let test_program: [u16; _] = [
    //     //
    //     0x6001, 0xD005, 0x00EE,
    // ];
    // let program: Vec<u8> = test_program
    //     .into_iter()
    //     .map(u16::to_be_bytes)
    //     .flatten()
    //     .collect();
    // let program = program.as_slice();

    chip.load_data(FONT.as_flattened());
    chip.load_program(program);

    let fps = 60;
    let frame_length = Duration::from_secs(1) / fps;

    print!("\x1b[2J\x1b[H");
    for i in 0.. {
        let time_start = Instant::now();

        chip.step();
        simple_display(&chip, i);

        if ByteArray::<u16>::read(&chip, PC).unwrap() == 0x3dcu16 {
            break;
        }

        let time_end = time_start.elapsed();
        if time_end < frame_length {
            sleep(frame_length - time_end);
        }
    }
    print!("\x1b[2J\x1b[H");
}

fn simple_display(chip: &Chip8, frame_number: u32) -> Option<()> {
    let mut frame = String::new();
    for y in 0..32 {
        let line: u64 = chip.read(Display + y * 8)?;
        frame.push_str(format!("{:064b}\n", line).as_str());
    }

    // let pc: u16 = chip.read(PC)?;
    // let opcode: u16 = chip.read(pc)?;
    // let vs: u128 = chip.read(Vs)?;
    // let i: u16 = chip.read(I)?;

    // println!("{}Frame{frame_number:5}", "_".repeat(54));
    println!("{}", frame.replace("0", "░").replace("1", "█"));
    // println!("pc:({pc:x}) | v:{} | i:{i:04x}", format_registers(vs));
    // println!(" ({:02x}) : {:x?}", opcode, OpCode::from(opcode));

    print!("\x1b[64A\x1b[32D");
    Some(())
}

fn format_registers(regs: u128) -> String {
    let mut regs = format!("{regs:032x}");
    for i in (0..regs.len() / 2).rev() {
        regs.insert_str(
            i * 2,
            if i % 2 == 0 {
                //
                "\x1B[0m\x1B[1;39;49m"
            } else {
                "\x1B[0m\x1B[2;39;49m"
            },
        );
    }

    regs + "\x1B[0m"
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
