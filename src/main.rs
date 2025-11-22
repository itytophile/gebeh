use minifb::{Key, Scale, Window, WindowOptions};

use crate::{
    cartridge::CartridgeType,
    cpu::PipelineExecutor,
    ppu::Ppu,
    state::{State, WriteOnlyState},
};
mod cartridge;
mod cpu;
mod dma;
mod gpu;
mod hardware;
mod ic;
mod instructions;
mod ppu;
mod state;

const WIDTH: usize = 160;
const HEIGHT: usize = 144;

fn main() {
    // let rom =
    //     std::fs::read("/home/ityt/Téléchargements/pocket/pocket.gb")
    //         .unwrap();
    let rom =
        std::fs::read("/home/ityt/Documents/git/gb-test-roms/interrupt_time/interrupt_time.gb")
            .unwrap();
    let title = &rom[0x134..0x143];
    let end_zero_pos = title
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(title.len());
    let title = str::from_utf8(&title[..end_zero_pos]).unwrap();
    println!("Title: {title}");
    let cartridge_type = CartridgeType::try_from(rom[0x147]).unwrap();

    println!("Cartridge type: {cartridge_type:?}");
    let mut state = State::new(rom.leak());
    // the machine should not be affected by the composition order
    let mut machine = PipelineExecutor::default().compose(Ppu::default());

    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];

    let mut window = Window::new(
        "Test - ESC to exit",
        WIDTH,
        HEIGHT,
        WindowOptions {
            resize: false,
            scale: Scale::X4,
            ..Default::default()
        },
    )
    .unwrap();

    while window.is_open() && !window.is_key_down(Key::Escape) {
        loop {
            machine.execute(&state)(WriteOnlyState::new(&mut state));
            let (_, ppu) = &mut machine;
            if let Some(ly) = ppu.drawn_ly.take() {
                let base = usize::from(ly) * WIDTH;
                for (a, b) in buffer[base..].iter_mut().zip(&ppu.gpu.draw_line) {
                    *a = u32::from(*b);
                }
                if usize::from(ly) == HEIGHT - 1 {
                    break;
                }
            }
        }

        // We unwrap here as we want this code to exit if it fails. Real applications may want to handle this in a different way
        window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
    }
}

trait StateMachine {
    /// must take one M-cycle
    fn execute<'a>(&'a mut self, state: &State) -> impl FnOnce(WriteOnlyState) + 'a;
    fn compose<T: StateMachine>(self, other: T) -> (Self, T)
    where
        Self: Sized,
    {
        (self, other)
    }
}

impl<T: StateMachine, U: StateMachine> StateMachine for (T, U) {
    fn execute<'a>(&'a mut self, state: &State) -> impl FnOnce(WriteOnlyState) + 'a {
        let first = self.0.execute(state);
        let second = self.1.execute(state);
        move |mut state| {
            first(state.reborrow());
            second(state);
        }
    }
}
