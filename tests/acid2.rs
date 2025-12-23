use std::num::NonZeroU8;

use gb_core::{
    HEIGHT, StateMachine,
    cpu::Cpu,
    ppu::{Color, Ppu, Speeder},
    state::State,
    timer::Timer,
};

#[test]
fn dmg_acid2() {
    let rom = std::fs::read("/home/ityt/Téléchargements/dmg-acid2.gb").unwrap();
    let mut state = State::new(rom.leak());
    let mut machine = Cpu::default()
        .compose(Speeder(Ppu::default(), NonZeroU8::new(4).unwrap()))
        .compose(Timer);
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
        machine.execute(&mut state, 0);
        let ((_, Speeder(ppu, _)), _) = &mut machine;
        if let Some(scanline) = ppu.get_scanline_if_ready()
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
