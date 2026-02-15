use gebeh_core::{
    Emulator, HEIGHT, SYSTEM_CLOCK_FREQUENCY, WIDTH,
    apu::Mixer,
    state::{Interruptions, SerialControl, State},
};
use gebeh_front_helper::{CloneMbc, get_mbc, get_noise, get_title_from_rom};
use wasm_bindgen::prelude::*;
use web_sys::{console, js_sys};

use crate::rtc::NullRtc;

mod rtc;

#[wasm_bindgen]
pub struct WebEmulator {
    emulator: Emulator,
    sample_index: u32,
    mbc: Box<dyn CloneMbc<'static>>,
    // to iterate SYSTEM_CLOCK_FREQUENCY / sample_rate on average even if the division is not round
    error: u32,
    is_save_enabled: bool,
    mixer: Mixer<Vec<u8>>,
    current_frame: [u8; WIDTH as usize * HEIGHT as usize],
}

#[wasm_bindgen]
impl WebEmulator {
    pub fn new(rom: Vec<u8>, save: Option<Vec<u8>>, sample_rate: f32) -> Option<Self> {
        console::log_1(&JsValue::from_str("Loading rom"));
        let Some((cartridge_type, mut mbc)) = get_mbc::<_, NullRtc>(rom) else {
            console::error_1(&JsValue::from_str("MBC type not recognized"));
            return None;
        };
        if let Some(save) = save {
            console::log_1(&JsValue::from_str("Loading save"));
            mbc.load_saved_ram(&save);
        }
        console::log_1(&JsValue::from_str("Rom loaded!"));

        if cartridge_type.has_battery() {
            console::log_1(&JsValue::from_str("Saves enabled"));
        }
        Some(Self {
            mbc,
            is_save_enabled: cartridge_type.has_battery(),
            emulator: Default::default(),
            sample_index: 0,
            error: 0,
            mixer: Mixer::new(sample_rate, get_noise(false), get_noise(true)),
            current_frame: [0; _],
        })
    }

    // this function is executed every 128 (RENDER_QUANTUM_SIZE) frames
    pub fn drive_and_sample(
        &mut self,
        left: &mut [f32],
        right: &mut [f32],
        sample_rate: u32,
        on_new_frame: &js_sys::Function,
    ) {
        let base = SYSTEM_CLOCK_FREQUENCY / sample_rate;
        let remainder = SYSTEM_CLOCK_FREQUENCY % sample_rate;

        for (left, right) in left.iter_mut().zip(right.iter_mut()) {
            let mut cycles = base;
            self.error += remainder;

            if let Some(error) = self.error.checked_sub(sample_rate) {
                self.error = error;
                cycles += 1;
            }

            for _ in 0..cycles {
                self.emulator.execute(self.mbc.as_mut());
                execute_serial(&mut self.emulator.state);
                if let Some(scanline) = self.emulator.get_ppu().get_scanline_if_ready() {
                    self.current_frame.as_chunks_mut::<40>().0
                        [usize::from(self.emulator.get_ppu().get_ly())] = *scanline.raw();

                    if self.emulator.get_ppu().get_ly() == HEIGHT - 1 {
                        let this = JsValue::null();
                        if let Err(err) = on_new_frame.call1(
                            &this,
                            &js_sys::Uint8Array::new_from_slice(&self.current_frame),
                        ) {
                            console::error_1(&err);
                        }
                    }
                }
            }
            let sample = self.sample_index as f32 / sample_rate as f32;
            let mut sampler = self
                .mixer
                .mix(self.emulator.get_apu().get_sampler(), sample);
            *left = sampler.sample_left();
            *right = sampler.sample_right();
            // 2 minutes without popping (sample_index must not be huge to prevent precision errors)
            self.sample_index = self.sample_index.wrapping_add(1) % (sample_rate * 2 * 60);
        }
    }

    pub fn get_save(&self) -> Option<Save> {
        if !self.is_save_enabled {
            return None;
        }

        Some(Save {
            ram: self.mbc.get_ram_to_save()?.into(),
            game_title: get_title_from_rom(self.mbc.get_rom()).to_owned(),
        })
    }

    pub fn set_a(&mut self, value: bool) {
        self.emulator.get_joypad_mut().a = value;
    }
    pub fn set_b(&mut self, value: bool) {
        self.emulator.get_joypad_mut().b = value;
    }
    pub fn set_start(&mut self, value: bool) {
        self.emulator.get_joypad_mut().start = value;
    }
    pub fn set_select(&mut self, value: bool) {
        self.emulator.get_joypad_mut().select = value;
    }
    pub fn set_left(&mut self, value: bool) {
        self.emulator.get_joypad_mut().left = value;
    }
    pub fn set_right(&mut self, value: bool) {
        self.emulator.get_joypad_mut().right = value;
    }
    pub fn set_down(&mut self, value: bool) {
        self.emulator.get_joypad_mut().down = value;
    }
    pub fn set_up(&mut self, value: bool) {
        self.emulator.get_joypad_mut().up = value;
    }
}

#[wasm_bindgen]
pub struct Save {
    ram: Box<[u8]>,
    game_title: String,
}

#[wasm_bindgen]
impl Save {
    pub fn get_ram(&self) -> Box<[u8]> {
        self.ram.clone()
    }

    pub fn get_game_title(&self) -> String {
        self.game_title.clone()
    }
}

fn execute_serial(state: &mut State) {
    if state
        .sc
        .contains(SerialControl::TRANSFER_ENABLE | SerialControl::CLOCK_SELECT)
    {
        // https://gbdev.io/pandocs/Serial_Data_Transfer_(Link_Cable).html#disconnects
        // Citation: On a disconnected link cable, the input bit on a master will start to read 1.
        // This means a master will start to receive $FF bytes.
        state.sb = 0xff;
        state.sc.remove(SerialControl::TRANSFER_ENABLE);
        state.interrupt_flag.insert(Interruptions::SERIAL);
    }
}
