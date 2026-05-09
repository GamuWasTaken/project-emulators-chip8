use super::*;

// TODO Reforce draw test it should check the collision
const INITIAL_PC: u16 = 0x200;

/// A default chip8 should be all 0 with a pc at 0x200
#[test]
fn default() {
    let chip = Chip8::default();

    let pc: u16 = chip.read(PC).unwrap();
    assert!(pc == INITIAL_PC);
}

// Set Display to 0
#[test]
fn clear() {
    let mut chip = Chip8::default();

    chip.write(0xffffffu64, Display).unwrap();
    println!(
        "Mm: {:032x}",
        ByteArray::<u128>::read(&chip, Display).unwrap()
    );
    chip.execute(OpCode::Clear);
    let display: u128 = chip.read(Display).unwrap();

    assert!(display == 0);
}

// Pop & Set PC
// Push & Set PC NNN
#[test]
fn subroutines() {
    let mut chip = Chip8::default();
    chip.execute(OpCode::Call { at: 0xfb03 });
    chip.execute(OpCode::Call { at: 0xfb03 });
    chip.execute(OpCode::Call { at: 0xfb03 });
    let pc: u16 = chip.read(PC).unwrap();

    assert!(pc == 0xfb03);

    chip.execute(OpCode::Return);
    chip.execute(OpCode::Return);
    chip.execute(OpCode::Return);
    let pc: u16 = chip.read(PC).unwrap();

    assert!(pc == INITIAL_PC);
}

// Skip if VX == NN
// Skip if VX != NN
// Skip if VX == VY
// Skip if VX != VY
// Skip if VX == key
// Skip if VX != key
#[test]
fn skips() {
    let mut chip = Chip8::default();
    chip.write(7u8, Vs + 4).unwrap();

    chip.execute(OpCode::SkipEqK { reg: 4, val: 0 });
    let pc: u16 = chip.read(PC).unwrap();
    assert!(pc == INITIAL_PC);
    chip.execute(OpCode::SkipEqK { reg: 4, val: 7 });
    let pc: u16 = chip.read(PC).unwrap();
    assert!(pc == INITIAL_PC + 2);

    chip.execute(OpCode::SkipNotEqK { reg: 4, val: 7 });
    let pc: u16 = chip.read(PC).unwrap();
    assert!(pc == INITIAL_PC + 2);
    chip.execute(OpCode::SkipNotEqK { reg: 4, val: 0 });
    let pc: u16 = chip.read(PC).unwrap();
    assert!(pc == INITIAL_PC + 4);

    chip.execute(OpCode::SkipEq { a: 4, b: 7 });
    let pc: u16 = chip.read(PC).unwrap();
    assert!(pc == INITIAL_PC + 4);
    chip.execute(OpCode::SkipEq { a: 7, b: 7 });
    let pc: u16 = chip.read(PC).unwrap();
    assert!(pc == INITIAL_PC + 6);

    chip.execute(OpCode::SkipNotEq { a: 7, b: 7 });
    let pc: u16 = chip.read(PC).unwrap();
    assert!(pc == INITIAL_PC + 6);
    chip.execute(OpCode::SkipNotEq { a: 4, b: 7 });
    let pc: u16 = chip.read(PC).unwrap();
    assert!(pc == INITIAL_PC + 8);

    chip.write(7u8, LastKey).unwrap();

    chip.execute(OpCode::SkipPressed { reg: 0 });
    let pc: u16 = chip.read(PC).unwrap();
    assert!(pc == INITIAL_PC + 8);
    chip.execute(OpCode::SkipPressed { reg: 4 });
    let pc: u16 = chip.read(PC).unwrap();
    assert!(pc == INITIAL_PC + 10);

    chip.execute(OpCode::SkipNotPressed { reg: 4 });
    let pc: u16 = chip.read(PC).unwrap();
    assert!(pc == INITIAL_PC + 10);
    chip.execute(OpCode::SkipNotPressed { reg: 0 });
    let pc: u16 = chip.read(PC).unwrap();
    assert!(pc == INITIAL_PC + 12);
}

// Set PC to NNN
// Set PC to V0 + NNN
// Set I to NNN
// Set I to the sprite location of VX
// Inc I with VX
#[test]
fn set16() {
    let mut chip = Chip8::default();

    chip.execute(OpCode::Jump { to: 0xfb03 });
    let pc: u16 = chip.read(PC).unwrap();
    assert!(pc == 0xfb03);

    chip.write(7u8, Vs + 0).unwrap();
    chip.execute(OpCode::JumpReg { by: 0xfb03 });
    let pc: u16 = chip.read(PC).unwrap();
    assert!(pc == 0xfb0a);

    chip.execute(OpCode::SetI { to: 0xfb03 });
    let i: u16 = chip.read(I).unwrap();
    assert!(i == 0xfb03);

    chip.execute(OpCode::GetFontSprite { of: 0 });
    let i: u16 = chip.read(I).unwrap();
    assert!(i == 7 * 5);

    chip.execute(OpCode::IncI { with: 0 });
    let i: u16 = chip.read(I).unwrap();
    assert!(i == 42);
}

// Set DT to VX
// Set ST to VX
// Set VX to DT
// Set VX to NN
// Set VX to KEY (block)
// Set VX to rand & NN
// Inc VX by NN
#[test]
fn set8() {
    let mut chip = Chip8::default();
    chip.write(7u8, Vs + 4).unwrap();

    chip.execute(OpCode::SetDelay { with: 4 });
    let dt: u8 = chip.read(DT).unwrap();
    assert!(dt == 7);

    chip.execute(OpCode::SetSound { with: 4 });
    let st: u8 = chip.read(ST).unwrap();
    assert!(st == 7);

    chip.execute(OpCode::ReadDelay { to: 3 });
    let v3: u8 = chip.read(Vs + 3).unwrap();
    assert!(v3 == 7);

    chip.execute(OpCode::SetV { reg: 2, to: 8 });
    let v2: u8 = chip.read(Vs + 2).unwrap();
    assert!(v2 == 8);

    chip.execute(OpCode::IncV { reg: 2, by: 8 });
    let v2: u8 = chip.read(Vs + 2).unwrap();
    assert!(v2 == 16);

    // chip.write(4u8 ,KEY);
    // chip.execute(OpCode::ReadKey { to: 1 });
    // let v1 : u8 = chip.read(Vs + 1).unwrap();
    // assert!(v1 == 4);

    const VAL: u8 = 0b11110000;
    chip.write(VAL, Vs + 9).unwrap();
    chip.execute(OpCode::GetRand { reg: 9, mask: !VAL });
    let v9: u8 = chip.read(Vs + 9).unwrap();
    assert!(v9 != VAL);
}

// TODO Draw sprite from I at VX VY with N lines; VF = some 1 got turned off
// TODO BCD of VX at I

// Save V0..VX to I
// Load V0..VX from I
#[test]
fn save_load() {
    let mut chip = Chip8::default();
    chip.write(0x77u8, Vs + 7).unwrap();
    chip.write(0x44u8, Vs + 4).unwrap();
    chip.write(INITIAL_PC, I).unwrap();
    chip.write(0x55u8, Memory + 5).unwrap();

    chip.execute(OpCode::SaveRegs { upto: 4 });
    let i: u16 = chip.read(I).unwrap();
    assert!(i == INITIAL_PC + 4 + 1);

    let m04: u8 = chip.read(Memory + 4).unwrap();
    assert!(m04 == 0x44);

    chip.write(INITIAL_PC, I).unwrap();
    chip.write(0x33u8, Memory + 3).unwrap();
    chip.execute(OpCode::LoadRegs { upto: 7 });

    let i: u16 = chip.read(I).unwrap();
    assert!(i == INITIAL_PC + 7 + 1);

    let v3: u8 = chip.read(Vs + 3).unwrap();
    assert!(v3 == 0x33);

    let v7: u8 = chip.read(Vs + 7).unwrap();
    assert!(v7 == 0x00);
}

// Set VX to VY
// VX |= VY
// VX &= VY
// VX ^= VY
// VX += VY; VF = overflow
// VX -= VY; VF = !undeflow
// VX = VY >> 1; VF = VY[0] lsb
// VX = VY - VX; VF = !underflow
// VX = VY << 1; VF = VY[7] msb
