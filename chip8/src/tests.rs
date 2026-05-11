use super::*;

// TODO Reforce draw test it should check the collision

/// A default chip8 should be all 0 with a pc at 0x200
#[test]
fn default() -> Result<()> {
    let chip = Chip8::new().expect("cannot create chip8");

    let pc: u16 = chip.read(PC)?;
    assert!(pc == Chip8::INITIAL_PC);
    Ok(())
}

// Set Display to 0
#[test]
fn clear() -> Result<()> {
    let mut chip = Chip8::new().expect("cannot create chip8");

    chip.write(0xffffffu64, Display)?;
    println!("Mm: {:032x}", ByteArray::<u128>::read(&chip, Display)?);
    chip.execute(OpCode::Clear)?;
    let display: u128 = chip.read(Display)?;

    assert!(display == 0);

    Ok(())
}

// Pop & Set PC
// Push & Set PC NNN
#[test]
fn subroutines() -> Result<()> {
    let mut chip = Chip8::new().expect("cannot create chip8");
    chip.execute(OpCode::Call { at: 0xfb03 })?;
    chip.execute(OpCode::Call { at: 0xfb03 })?;
    chip.execute(OpCode::Call { at: 0xfb03 })?;
    let pc: u16 = chip.read(PC)?;

    assert!(pc == 0xfb03);

    chip.execute(OpCode::Return)?;
    chip.execute(OpCode::Return)?;
    chip.execute(OpCode::Return)?;
    let pc: u16 = chip.read(PC)?;

    assert!(pc == Chip8::INITIAL_PC);
    Ok(())
}

// Skip if VX == NN
// Skip if VX != NN
// Skip if VX == VY
// Skip if VX != VY
// Skip if VX == key
// Skip if VX != key
#[test]
fn skips() -> Result<()> {
    let mut chip = Chip8::new().expect("cannot create chip8");
    chip.write(7u8, Vs + 4)?;

    chip.execute(OpCode::SkipEqK { reg: 4, val: 0 })?;
    let pc: u16 = chip.read(PC)?;
    assert!(pc == Chip8::INITIAL_PC);
    chip.execute(OpCode::SkipEqK { reg: 4, val: 7 })?;
    let pc: u16 = chip.read(PC)?;
    assert!(pc == Chip8::INITIAL_PC + 2);

    chip.execute(OpCode::SkipNotEqK { reg: 4, val: 7 })?;
    let pc: u16 = chip.read(PC)?;
    assert!(pc == Chip8::INITIAL_PC + 2);
    chip.execute(OpCode::SkipNotEqK { reg: 4, val: 0 })?;
    let pc: u16 = chip.read(PC)?;
    assert!(pc == Chip8::INITIAL_PC + 4);

    chip.execute(OpCode::SkipEq { a: 4, b: 7 })?;
    let pc: u16 = chip.read(PC)?;
    assert!(pc == Chip8::INITIAL_PC + 4);
    chip.execute(OpCode::SkipEq { a: 7, b: 7 })?;
    let pc: u16 = chip.read(PC)?;
    assert!(pc == Chip8::INITIAL_PC + 6);

    chip.execute(OpCode::SkipNotEq { a: 7, b: 7 })?;
    let pc: u16 = chip.read(PC)?;
    assert!(pc == Chip8::INITIAL_PC + 6);
    chip.execute(OpCode::SkipNotEq { a: 4, b: 7 })?;
    let pc: u16 = chip.read(PC)?;
    assert!(pc == Chip8::INITIAL_PC + 8);

    chip.write(7u8, LastKey)?;

    chip.execute(OpCode::SkipPressed { reg: 0 })?;
    let pc: u16 = chip.read(PC)?;
    assert!(pc == Chip8::INITIAL_PC + 8);
    chip.execute(OpCode::SkipPressed { reg: 4 })?;
    let pc: u16 = chip.read(PC)?;
    assert!(pc == Chip8::INITIAL_PC + 10);

    chip.execute(OpCode::SkipNotPressed { reg: 4 })?;
    let pc: u16 = chip.read(PC)?;
    assert!(pc == Chip8::INITIAL_PC + 10);
    chip.execute(OpCode::SkipNotPressed { reg: 0 })?;
    let pc: u16 = chip.read(PC)?;
    assert!(pc == Chip8::INITIAL_PC + 12);
    Ok(())
}

// Set PC to NNN
// Set PC to V0 + NNN
// Set I to NNN
// Set I to the sprite location of VX
// Inc I with VX
#[test]
fn set16() -> Result<()> {
    let mut chip = Chip8::new().expect("cannot create chip8");

    chip.execute(OpCode::Jump { to: 0xfb03 })?;
    let pc: u16 = chip.read(PC)?;
    assert!(pc == 0xfb03);

    chip.write(7u8, Vs + 0)?;
    chip.execute(OpCode::JumpReg { by: 0xfb03 })?;
    let pc: u16 = chip.read(PC)?;
    assert!(pc == 0xfb0a);

    chip.execute(OpCode::SetI { to: 0xfb03 })?;
    let i: u16 = chip.read(I)?;
    assert!(i == 0xfb03);

    chip.execute(OpCode::GetFontSprite { of: 0 })?;
    let i: u16 = chip.read(I)?;
    assert!(i == 7 * 5);

    chip.execute(OpCode::IncI { with: 0 })?;
    let i: u16 = chip.read(I)?;
    assert!(i == 42);
    Ok(())
}

// Set DT to VX
// Set ST to VX
// Set VX to DT
// Set VX to NN
// Set VX to KEY (block)
// Set VX to rand & NN
// Inc VX by NN
#[test]
fn set8() -> Result<()> {
    let mut chip = Chip8::new()?;
    chip.write(7u8, Vs + 4)?;

    chip.execute(OpCode::SetDelay { with: 4 })?;
    let dt: u8 = chip.read(DT)?;
    assert!(dt == 7);

    chip.execute(OpCode::SetSound { with: 4 })?;
    let st: u8 = chip.read(ST)?;
    assert!(st == 7);

    chip.execute(OpCode::ReadDelay { to: 3 })?;
    let v3: u8 = chip.read(Vs + 3)?;
    assert!(v3 == 7);

    chip.execute(OpCode::SetV { reg: 2, to: 8 })?;
    let v2: u8 = chip.read(Vs + 2)?;
    assert!(v2 == 8);

    chip.execute(OpCode::IncV { reg: 2, by: 8 })?;
    let v2: u8 = chip.read(Vs + 2)?;
    assert!(v2 == 16);

    // chip.write(4u8 ,KEY);
    // chip.execute(OpCode::ReadKey { to: 1 })?;
    // let v1 : u8 = chip.read(Vs + 1)?;
    // assert!(v1 == 4);

    const VAL: u8 = 0b11110000;
    chip.write(VAL, Vs + 9)?;
    chip.execute(OpCode::GetRand { reg: 9, mask: !VAL })?;
    let v9: u8 = chip.read(Vs + 9)?;
    assert!(v9 != VAL);
    Ok(())
}

// TODO Draw sprite from I at VX VY with N lines; VF = some 1 got turned off
// TODO BCD of VX at I

// Save V0..VX to I
// Load V0..VX from I
#[test]
fn save_load() -> Result<()> {
    let mut chip = Chip8::new().expect("cannot create chip8");
    chip.write(0x77u8, Vs + 7)?;
    chip.write(0x44u8, Vs + 4)?;
    chip.write(Chip8::INITIAL_PC, I)?;
    chip.write(0x55u8, Memory + 5)?;

    chip.execute(OpCode::SaveRegs { upto: 4 })?;
    let i: u16 = chip.read(I)?;
    assert!(i == Chip8::INITIAL_PC + 4 + 1);

    let m04: u8 = chip.read(Memory + 4)?;
    assert!(m04 == 0x44);

    chip.write(Chip8::INITIAL_PC, I)?;
    chip.write(0x33u8, Memory + 3)?;
    chip.execute(OpCode::LoadRegs { upto: 7 })?;

    let i: u16 = chip.read(I)?;
    assert!(i == Chip8::INITIAL_PC + 7 + 1);

    let v3: u8 = chip.read(Vs + 3)?;
    assert!(v3 == 0x33);

    let v7: u8 = chip.read(Vs + 7)?;
    assert!(v7 == 0x00);
    Ok(())
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
