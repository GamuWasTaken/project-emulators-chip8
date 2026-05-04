use std::{
    env, process,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, sleep},
    time::{Duration, Instant},
};

use chip8::*;
use k_board::{keyboard::Keyboard, keys::Keys};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
enum ChipArgs {
    NoGraphics,
}
impl TryFrom<String> for ChipArgs {
    type Error = ();

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "-d" | "--no-display" => Ok(ChipArgs::NoGraphics),
            _ => Err(()),
        }
    }
}
fn main() {
    let args: Vec<_> = env::args()
        .into_iter()
        .filter_map(|a| ChipArgs::try_from(a).ok())
        .collect();

    run(args);
}
// TODO 1dcell panics because of shift right overflow

fn run(args: Vec<ChipArgs>) -> Option<()> {
    let mut chip = Chip8::default();

    // Read keys continuously
    let read_key = Arc::new(Mutex::new(Keys::Null));
    let write_key = read_key.clone();
    thread::spawn(move || read_keys(write_key));

    let stop_sig = Arc::new(AtomicBool::new(false));
    {
        let stop_sig = stop_sig.clone();
        ctrlc::set_handler(move || stop_sig.store(true, Ordering::SeqCst)).unwrap();
    }

    let program = include_bytes!("./slipperyslope.ch8");

    chip.load_data(FONT.as_flattened());
    chip.load_program(program);

    let fps = 60 * 5;
    let frame_length = Duration::from_secs(1) / fps;

    let mut previous_step_output = PostExecute::Stay;

    print!("\x1b[2J\x1b[H");
    for i in 0.. {
        if stop_sig.load(Ordering::SeqCst) {
            break;
        }

        let time_start = Instant::now();

        chip.step_timers()?;

        let pressed_key;
        {
            let key = read_key.lock().ok()?;
            pressed_key = *key;
        }

        match (previous_step_output, pressed_key) {
            (PostExecute::Wait, Keys::Null) => (),
            _ => {
                let traduction = KEY_MAP
                    .into_iter()
                    .find(|(k, _)| *k == pressed_key)
                    .map(|(_, v)| v)
                    .unwrap_or(0xff);

                chip.load_key(traduction);
                previous_step_output = chip.step()?;

                if !args.contains(&ChipArgs::NoGraphics) {
                    simple_display(&chip, i);
                }
            }
        }

        let time_end = time_start.elapsed();
        if time_end < frame_length {
            sleep(frame_length - time_end);
        }
    }
    print!("\x1b[2J\x1b[H");

    Some(())
}

fn read_keys(pressed_key: Arc<Mutex<Keys>>) {
    for key in Keyboard::new() {
        let mut v = pressed_key.lock().unwrap();
        *v = key;
    }
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

pub const KEY_MAP: [(Keys, u8); 16] = [
    (Keys::Char('q'), 0x0),
    (Keys::Char('w'), 0x1),
    (Keys::Char('e'), 0x2),
    (Keys::Char('r'), 0x3),
    //
    (Keys::Char('a'), 0x4),
    (Keys::Char('s'), 0x5),
    (Keys::Char('d'), 0x6),
    (Keys::Char('f'), 0x7),
    //
    (Keys::Char('u'), 0x8),
    (Keys::Char('i'), 0x9),
    (Keys::Char('o'), 0xa),
    (Keys::Char('ñ'), 0xb),
    //
    (Keys::Char('j'), 0xc),
    (Keys::Char('k'), 0xd),
    (Keys::Char('l'), 0xe),
    (Keys::Char('p'), 0xf),
];
