use std::{
    thread::sleep,
    time::{Duration, Instant},
};

use chip8::*;

fn main() {
    run();
}

fn run() {
    let mut chip = Chip8::default();
    let keyboard = k_board::keyboard::Keyboard::new();

    let program = include_bytes!("./test.ch8");

    chip.load_data(FONT.as_flattened());
    chip.load_program(program);

    let fps = 60;
    let frame_length = Duration::from_secs(1) / fps;

    print!("\x1b[2J\x1b[H");
    for i in 0.. {
        let time_start = Instant::now();

        let step = chip.step();
        simple_display(&chip, i);

        if let Some(StepResult::WaitKey) = step {
            // TODO make a thread that listens for keys and messages us via mpsc of the keys, once per frame take a key and put it in key, 'waitKey' can be handled by not calling step until a new key is loaded
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
