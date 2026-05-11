use anyhow::{Context, Error, Result};
use std::{
    env,
    fs::read,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU16, Ordering},
    },
    time::Duration,
};

use chip8::*;
use ratatui::{
    DefaultTerminal,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    prelude::*,
    widgets::Paragraph,
};

use crate::timing::Every;

mod timing;

#[derive(Debug)]
struct App {
    chip: Chip8,
    keys: Arc<AtomicU16>,
    exit: Arc<AtomicBool>,
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let screen_bytes: &[[u8; 8]] = unsafe { self.chip[Region::Display].as_chunks_unchecked() };
        let screen = screen_bytes
            .into_iter()
            .map(|chunk| u64::from_be_bytes(*chunk))
            .map(|chunk| format!("{chunk:064b}\n"))
            .collect::<String>()
            .replace("0", "░")
            .replace("1", "█");

        // TODO dont like this
        let pc: u16 = self.chip.read(Region::PC).unwrap();
        let opcode: u16 = self.chip.read(pc).unwrap();
        let vs: u128 = self.chip.read(Region::Vs).unwrap();
        let i: u16 = self.chip.read(Region::I).unwrap();
        let dt: u8 = self.chip.read(Region::DT).unwrap();

        // TODO paint the border, and info about chip
        // println!("{}Frame{frame_number:5}", "_".repeat(54));
        // println!("{}", frame.replace("0", "░").replace("1", "█"));
        // println!(
        //     "pc:({pc:x}) | v:{} | i:{i:04x} | dt:{dt:02x}",
        //     format_registers(vs)
        // );
        // println!(" ({:02x}) : {:x?}", opcode, OpCode::try_from(opcode));

        // println!("Pressed: {key:016b}");

        Paragraph::new(screen).centered().render(area, buf);
    }
}

impl App {
    fn run(mut self, options: ChipOptions, terminal: &mut DefaultTerminal) -> Result<()> {
        // Input thread
        let _key = self.keys.clone();
        let _exit = self.exit.clone();
        let input = std::thread::Builder::new()
            .name("Input".into())
            .spawn(move || {
                let _ = input(_key, _exit);
                println!("Input exited")
            })?;

        // Harness
        let fps = options.fps;
        let frame_length = if let Some(fps) = fps {
            Duration::from_secs(1) / fps
        } else {
            Duration::ZERO
        };

        for _ in Every::new(frame_length) {
            if self.exit.load(Ordering::Acquire) {
                break;
            }

            terminal.draw(|frame| frame.render_widget(&self, frame.area()))?;

            self.chip.load_key(self.keys.load(Ordering::Acquire))?;

            self.chip.step()?;
        }

        // Cleanup
        input
            .join()
            .map_err(|_| Error::msg("couldnt join input thread"))?;

        Ok(())
    }
}
fn main() {
    ratatui().expect("Something went wrong");
    // run();
}
// TODO handle errors instead of just returning Option return Result with message
// TODO slipperyslope runs incorrectly, seems to be collision code
// Main problems to solve:
// Quirks...

fn ratatui() -> Result<()> {
    // Setup
    let options: ChipOptions = env::args().try_into()?;
    let program = read(options.program_path.clone())
        .with_context(|| format!("program '{}' not found", options.program_path))?;

    if options.only_parse {
        for opcode in OpCode::parse_program(&program).chunks(5) {
            println!("{opcode:?}");
        }
        println!("Program: {}", options.program_path);
        return Ok(());
    }

    let mut chip = Chip8::new()?;
    chip.load_program(&program)?;
    chip.load_data(FONT.as_flattened())?;

    let app = App {
        chip,
        keys: Arc::new(AtomicU16::new(0)),
        exit: Arc::new(AtomicBool::new(false)),
    };

    ratatui::run(|terminal| app.run(options, terminal))?;

    Ok(())
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq)]
struct ChipOptions {
    graphics: bool,
    only_parse: bool,
    program_path: String,
    fps: Option<u32>,
}
impl Default for ChipOptions {
    fn default() -> Self {
        Self {
            graphics: true,
            only_parse: false,
            program_path: "./roms/test.ch8".into(),
            fps: None,
        }
    }
}

impl<'a> TryFrom<env::Args> for ChipOptions {
    type Error = anyhow::Error;

    fn try_from(args: env::Args) -> Result<Self, Self::Error> {
        let mut args = args.into_iter().skip(1);
        let mut res = Self::default();

        while let Some(word) = args.next() {
            match word.as_str() {
                "-p" | "--parse" => res.only_parse = true,
                "-g" | "--no-graphics" => res.graphics = false,
                "-s" | "--fps" => {
                    let argument = args
                        .next()
                        .with_context(|| format!("{word} needs an argument"))?;

                    res.fps = Some(
                        argument
                            .as_str()
                            .parse()
                            .with_context(|| format!("{word} needs the number of fps"))?,
                    );
                }
                "-f" | "--file" => {
                    res.program_path = args
                        .next()
                        .with_context(|| format!("{word} needs the file path as argument"))?
                }
                _ => res.program_path = word,
            }
        }

        Ok(res)
    }
}

// fn harness(options: ChipOptions, exit: Arc<AtomicBool>, key: Arc<AtomicU16>) -> Result<()> {
//     let mut chip = Chip8::new()?;

//     dbg!(options.program_path.clone());
//     let program = read(options.program_path)?;
//     chip.load_program(&program)?;
//     chip.load_data(FONT.as_flattened())?;

//     chip.write(u8::MAX, Region::DT)?;

//     let fps = options.fps;
//     let frame_length = if let Some(fps) = fps {
//         Duration::from_secs(1) / fps
//     } else {
//         Duration::ZERO
//     };

//     let mut prev_output = PostExecute::Next;

//     // TODO change to ratatui
//     print!("\x1b[2J\x1b[H");
//     for (i, _) in Every::new(frame_length).enumerate() {
//         if exit.load(Ordering::SeqCst) {
//             break;
//         }
//         let key = key.load(Ordering::Acquire);

//         chip.load_key(key)?;

//         if prev_output != PostExecute::Wait || key != 0xff {
//             prev_output = chip.step()?;
//         }

//         if options.graphics {
//             let keys: u16 = chip.read(Region::Keys)?;
//             simple_display(&chip, i as u32, keys)?;
//         }
//     }
//     print!("\x1b[2J\x1b[H");

//     Ok(())
// }

fn input(key: Arc<AtomicU16>, exit: Arc<AtomicBool>) -> Result<()> {
    loop {
        if exit.load(Ordering::Acquire) {
            break;
        }

        match event::read()? {
            Event::Key(key_event) => match key_event {
                KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::CONTROL,
                    kind: KeyEventKind::Press,
                    ..
                } => {
                    exit.store(true, Ordering::Release);
                }
                KeyEvent {
                    code: KeyCode::Char(pressed_key),
                    modifiers: KeyModifiers::NONE,
                    kind: KeyEventKind::Press,
                    ..
                } => {
                    let key_state = key.load(Ordering::Acquire);
                    key.store(key_state | translate_key(pressed_key), Ordering::Release);
                }
                KeyEvent {
                    code: KeyCode::Char(pressed_key),
                    modifiers: KeyModifiers::NONE,
                    kind: KeyEventKind::Release,
                    ..
                } => {
                    let key_state = key.load(Ordering::Acquire);
                    key.store(key_state & !translate_key(pressed_key), Ordering::Release);
                }
                _ => {}
            },
            _ => {}
        }
    }

    Ok(())
}

// fn run() -> Result<()> {
//     let options: ChipOptions = env::args().try_into()?;

//     let program = read(options.program_path.clone()).expect("Program not found");

//     if options.only_parse {
//         for opcode in OpCode::parse_program(&program).chunks(5) {
//             println!("{opcode:?}");
//         }
//         println!("Program: {}", options.program_path);
//         return Ok(());
//     }

//     // Handle kill signals
//     let exit = Arc::new(AtomicBool::new(false));
//     let _exit = exit.clone();
//     ctrlc::set_handler(move || _exit.store(true, Ordering::SeqCst))?;

//     // Read keys continuously
//     let key = Arc::new(AtomicU16::new(0xff));
//     let _key = key.clone();
//     let _exit = exit.clone();
//     let input = std::thread::Builder::new()
//         .name("Veronica".into())
//         .spawn(move || {
//             input(_key, _exit);
//             println!("Input exited")
//         })?;

//     // Harness
//     let _key = key.clone();
//     let _exit = exit.clone();
//     let harness = std::thread::Builder::new()
//         .name("Staicy".into())
//         .spawn(move || {
//             harness(options, _exit, _key).unwrap(); // relay the err to stdin
//             println!("Harness exited")
//         })?;

//     input
//         .join()
//         .map_err(|_| Error::msg("couldnt join input thread"))?;
//     harness
//         .join()
//         .map_err(|_| Error::msg("couldnt join harness thread"))?;

//     Ok(())
// }

// fn simple_display(chip: &Chip8, frame_number: u32, key: u16) -> Result<()> {
//     use Region::*;
//     let mut frame = String::new();
//     for y in 0..32 {
//         let line: u64 = chip.read(Display + y * 8)?;
//         frame.push_str(format!("{:064b}\n", line).as_str());
//     }

//     let pc: u16 = chip.read(PC)?;
//     let opcode: u16 = chip.read(pc)?;
//     let vs: u128 = chip.read(Vs)?;
//     let i: u16 = chip.read(I)?;
//     let dt: u8 = chip.read(DT)?;

//     println!("{}Frame{frame_number:5}", "_".repeat(54));
//     println!("{}", frame.replace("0", "░").replace("1", "█"));
//     println!(
//         "pc:({pc:x}) | v:{} | i:{i:04x} | dt:{dt:02x}",
//         format_registers(vs)
//     );
//     println!(" ({:02x}) : {:x?}", opcode, OpCode::try_from(opcode));

//     println!("Pressed: {key:016b}");

//     print!("\x1b[64A\x1b[32D");

//     Ok(())
// }

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

pub const KEY_MAP: [(char, u16); 16] = [
    ('q', 1 << 0x0),
    ('w', 1 << 0x1),
    ('e', 1 << 0x2),
    ('r', 1 << 0x3),
    //
    ('a', 1 << 0x4),
    ('s', 1 << 0x5),
    ('d', 1 << 0x6),
    ('f', 1 << 0x7),
    //
    ('z', 1 << 0x8),
    ('x', 1 << 0x9),
    ('c', 1 << 0xa),
    ('v', 1 << 0xb),
    //
    ('j', 1 << 0xc),
    ('k', 1 << 0xd),
    ('l', 1 << 0xe),
    ('p', 1 << 0xf),
];
fn translate_key(key: char) -> u16 {
    KEY_MAP
        .into_iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v)
        .unwrap_or(0)
}
