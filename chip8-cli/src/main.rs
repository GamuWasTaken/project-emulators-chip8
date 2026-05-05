use std::{
    env,
    fs::read,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU8, Ordering},
    },
    time::Duration,
};

use chip8::*;
use k_board::{keyboard::Keyboard, keys::Keys};
mod timing;

#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq)]
enum ChipArgs {
    NoGraphics,
    Parse,
    Program(String),
}
impl TryFrom<String> for ChipArgs {
    type Error = ();

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "-d" | "--no-display" => Ok(ChipArgs::NoGraphics),
            "-p" | "--parse" => Ok(ChipArgs::Parse),
            _ => Ok(ChipArgs::Program(value)),
        }
    }
}
fn main() {
    run();
}

// TODO slipperyslope runs incorrectly, seems to be collision code

struct HarnessOptions {
    program: Vec<u8>,
    display: bool,
}

fn harness(options: HarnessOptions, exit: Arc<AtomicBool>, key: Arc<AtomicU8>) -> Option<()> {
    let mut chip = Chip8::default();

    chip.load_data(FONT.as_flattened())?;
    chip.load_program(&options.program)?;

    let fps = 60 * 10;
    let frame_length = Duration::from_secs(1) / fps;

    let mut prev_output = PostExecute::Stay;

    print!("\x1b[2J\x1b[H");
    for (i, _) in timing::Every::new(frame_length).enumerate() {
        if exit.load(Ordering::SeqCst) {
            break;
        }
        let key = key.load(Ordering::Acquire);

        chip.load_key(key)?;
        chip.step_timers()?;

        if prev_output != PostExecute::Wait || key != 0xff {
            prev_output = chip.step()?;
        }

        if options.display {
            simple_display(&chip, i as u32, key)?;
        }
    }
    print!("\x1b[2J\x1b[H");

    Some(())
}

fn input(key: Arc<AtomicU8>, exit: Arc<AtomicBool>) {
    let keyboard = Keyboard::new();

    for pressed_key in keyboard {
        if exit.load(Ordering::Acquire) {
            break;
        }

        let traduction = KEY_MAP
            .into_iter()
            .find(|(k, _)| *k == pressed_key)
            .map(|(_, v)| v)
            .unwrap_or(0xff);

        key.swap(traduction, Ordering::Release);
    }
}

fn run() -> Option<()> {
    let args: Vec<_> = env::args()
        .into_iter()
        .skip(1)
        .filter_map(|a| ChipArgs::try_from(a).ok())
        .collect();

    let path = args
        .iter()
        .find_map(|a| match a {
            ChipArgs::Program(p) => Some(p.to_owned()),
            _ => None,
        })
        .or(Some("./src/test.ch8".into()))?;

    let program = match read(path.clone()) {
        Ok(p) => p,
        Err(e) => panic!("Something happened loading the file: {e}"),
    };

    if args.contains(&ChipArgs::Parse) {
        for opcode in OpCode::parse_program(&program).chunks(5) {
            println!("{opcode:?}");
        }
        println!("Program: {path}");
        return Some(());
    }

    let options = HarnessOptions {
        program,
        display: !args.contains(&ChipArgs::NoGraphics),
    };

    // Handle kill signals
    let exit = Arc::new(AtomicBool::new(false));
    let _exit = exit.clone();
    ctrlc::set_handler(move || _exit.store(true, Ordering::SeqCst)).ok()?;

    // Read keys continuously
    let key = Arc::new(AtomicU8::new(0xff));
    let _key = key.clone();
    let _exit = exit.clone();
    let input = std::thread::Builder::new()
        .name("Veronica".into())
        .spawn(move || {
            input(_key, _exit);
            println!("Input exited")
        })
        .ok()?;

    // Harness
    let _key = key.clone();
    let _exit = exit.clone();
    let harness = std::thread::Builder::new()
        .name("Staicy".into())
        .spawn(move || {
            harness(options, _exit, _key);
            println!("Harness exited")
        })
        .ok()?;

    input.join().ok()?;
    harness.join().ok()?;

    Some(())
}

fn simple_display(chip: &Chip8, frame_number: u32, key: u8) -> Option<()> {
    let mut frame = String::new();
    for y in 0..32 {
        let line: u64 = chip.read(Display + y * 8)?;
        frame.push_str(format!("{:064b}\n", line).as_str());
    }

    let pc: u16 = chip.read(PC)?;
    let opcode: u16 = chip.read(pc)?;
    let vs: u128 = chip.read(Vs)?;
    let i: u16 = chip.read(I)?;
    let dt: u8 = chip.read(DT)?;

    println!("{}Frame{frame_number:5}", "_".repeat(54));
    println!("{}", frame.replace("0", "░").replace("1", "█"));
    println!(
        "pc:({pc:x}) | v:{} | i:{i:04x} | dt:{dt:02x}",
        format_registers(vs)
    );
    println!(" ({:02x}) : {:x?}", opcode, OpCode::try_from(opcode));

    println!("Pressed: {key:?}");

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
