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

use crate::timing::Every;

mod timing;

fn main() {
    run();
}

// TODO slipperyslope runs incorrectly, seems to be collision code

#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq)]
struct ChipOptions {
    graphics: bool,
    only_parse: bool,
    program_path: String,
    fps: u32,
}
impl Default for ChipOptions {
    fn default() -> Self {
        Self {
            graphics: true,
            only_parse: false,
            program_path: "./roms/test.ch8".into(),
            fps: 60,
        }
    }
}

impl<'a> TryFrom<env::Args> for ChipOptions {
    type Error = ();

    fn try_from(value: env::Args) -> Result<Self, Self::Error> {
        let mut iter = value.into_iter().skip(1);
        let mut res = Self::default();

        while let Some(word) = iter.next() {
            match word.as_str() {
                "-p" | "--parse" => res.only_parse = true,
                "-g" | "--no-graphics" => res.graphics = false,
                "-s" | "--fps" => {
                    res.fps = iter
                        .next()
                        .ok_or(())
                        .and_then(|s| s.as_str().parse().map_err(|_| ()))?;
                }
                "-f" | "--file" => res.program_path = iter.next().ok_or(())?,
                _ => res.program_path = word,
            }
        }

        Ok(res)
    }
}

fn harness(
    options: ChipOptions,
    exit: Arc<AtomicBool>,
    key: Arc<AtomicU8>,
    delay: Arc<AtomicU8>,
    sound: Arc<AtomicU8>,
) -> Option<()> {
    let mut chip = Chip8::default();

    dbg!(options.program_path.clone());
    let program = read(options.program_path).ok()?;
    chip.load_program(&program)?;
    chip.load_data(FONT.as_flattened())?;

    let fps = options.fps;
    let frame_length = Duration::from_secs(1) / fps;

    let mut prev_output = PostExecute::Stay;

    // TODO change to ratatui
    print!("\x1b[2J\x1b[H");
    for (i, _) in timing::Every::new(frame_length).enumerate() {
        if exit.load(Ordering::SeqCst) {
            break;
        }
        let key = key.load(Ordering::Acquire);

        match prev_output {
            PostExecute::UpdateDt => delay.store(chip.read(DT)?, Ordering::Release),
            PostExecute::UpdateSt => sound.store(chip.read(ST)?, Ordering::Release),
            _ => {}
        }
        chip.write(delay.load(Ordering::Acquire), DT)?;
        chip.write(sound.load(Ordering::Acquire), ST)?;

        // TODO chip8 actually reads multiple keys at once, so... we have to change it :)
        chip.load_key(key)?;

        if prev_output != PostExecute::Wait || key != 0xff {
            prev_output = chip.step()?;
        }

        if options.graphics {
            simple_display(&chip, i as u32, key)?;
        }
    }
    print!("\x1b[2J\x1b[H");

    Some(())
}

fn timers(delay: Arc<AtomicU8>, sound: Arc<AtomicU8>, exit: Arc<AtomicBool>) {
    for _ in Every::new(Duration::from_millis(1000 / 60)) {
        if exit.load(Ordering::Acquire) {
            break;
        }
        let dt = delay.load(Ordering::Acquire);
        // Decrement only if dt hasnt been modified
        let _ = delay.compare_exchange(
            dt,
            dt.saturating_sub(1),
            Ordering::Acquire,
            Ordering::Relaxed,
        );

        let st = sound.load(Ordering::Acquire);
        // Decrement only if st hasnt been modified
        let _ = sound.compare_exchange(
            st,
            st.saturating_sub(1),
            Ordering::Acquire,
            Ordering::Relaxed,
        );
    }
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
    let options: ChipOptions = env::args().try_into().ok()?;

    let program = read(options.program_path.clone()).ok()?;

    if options.only_parse {
        for opcode in OpCode::parse_program(&program).chunks(5) {
            println!("{opcode:?}");
        }
        println!("Program: {}", options.program_path);
        return Some(());
    }

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

    // Timers
    let (delay, sound) = (Arc::new(AtomicU8::new(0)), Arc::new(AtomicU8::new(0)));
    let (_delay, _sound) = (delay.clone(), sound.clone());
    let _exit = exit.clone();
    let timers = std::thread::Builder::new()
        .name("Jessica".into())
        .spawn(move || {
            timers(_delay, _sound, _exit);
            println!("Timers exited")
        })
        .ok()?;

    // Harness
    let _key = key.clone();
    let _exit = exit.clone();
    let (_delay, _sound) = (delay.clone(), sound.clone());
    let harness = std::thread::Builder::new()
        .name("Staicy".into())
        .spawn(move || {
            harness(options, _exit, _key, _delay, _sound);
            println!("Harness exited")
        })
        .ok()?;

    input.join().ok()?;
    timers.join().ok()?;
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

    println!("Pressed: {key:02x?}");

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
    (Keys::Char('z'), 0x8),
    (Keys::Char('x'), 0x9),
    (Keys::Char('c'), 0xa),
    (Keys::Char('v'), 0xb),
    //
    (Keys::Char('j'), 0xc),
    (Keys::Char('k'), 0xd),
    (Keys::Char('l'), 0xe),
    (Keys::Char('p'), 0xf),
];
