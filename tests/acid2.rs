use gb_core::{Emulator, HEIGHT, StateMachine, ppu::Color, state::State};

#[test]
fn dmg_acid2() {
    let rom = std::fs::read("/home/ityt/Téléchargements/dmg-acid2.gb").unwrap();
    let mut state = State::new(rom.leak());
    let mut emulator = Emulator::default();
    let mut previous_ly = None;
    let expected = include_bytes!("acid2_expected.txt");
    let split = expected.split(|a| *a == b'\n').map(|slice| {
        slice.iter().map(|c| match c {
            b'.' => Color::White,
            b'-' => Color::LightGray,
            b'#' => Color::DarkGray,
            b'@' => Color::Black,
            _ => panic!(),
        })
    });
    let mut working_split = split.clone();
    let mut all_good = true;
    loop {
        emulator.execute(&mut state, 0);
        if let Some(scanline) = emulator.get_ppu().get_scanline_if_ready()
            && previous_ly != Some(state.ly)
        {
            previous_ly = Some(state.ly);
            all_good &= working_split.next().unwrap().eq(scanline.iter().copied());
            if state.ly == HEIGHT - 1 {
                if all_good {
                    return;
                }
                working_split = split.clone();
                all_good = true;
            }
        }
    }
}
