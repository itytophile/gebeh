use gebeh_core::{Emulator, HEIGHT, SYSTEM_CLOCK_FREQUENCY, WIDTH, apu::WaveCorrector};
use gebeh_front_helper::{CloneMbc, get_mbc, get_noise, get_title_from_rom};
use wasm_bindgen::prelude::*;
use web_sys::{console, js_sys};

use crate::rtc::NullRtc;

mod rtc;

#[wasm_bindgen]
pub struct WebEmulator {
    emulator: Emulator,
    noise: Vec<u8>,
    short_noise: Vec<u8>,
    sample_index: u32,
    mbc: Option<Box<dyn CloneMbc<'static>>>,
    // to iterate SYSTEM_CLOCK_FREQUENCY / sample_rate on average even if the division is not round
    error: u32,
    is_save_enabled: bool,
    wave_corrector_left: WaveCorrector,
    wave_corrector_right: WaveCorrector,
}

impl Default for WebEmulator {
    fn default() -> Self {
        Self {
            emulator: Default::default(),
            noise: get_noise(false),
            short_noise: get_noise(true),
            sample_index: 0,
            mbc: None,
            error: 0,
            is_save_enabled: false,
            wave_corrector_left: WaveCorrector::default(),
            wave_corrector_right: WaveCorrector::default(),
        }
    }
}

#[wasm_bindgen]
impl WebEmulator {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Default::default()
    }

    // this function is executed every 128 (RENDER_QUANTUM_SIZE) frames
    pub fn drive_and_sample(
        &mut self,
        left: &mut [f32],
        right: &mut [f32],
        sample_rate: u32,
        on_new_frame: &js_sys::Function,
        current_frame: &mut [u8],
    ) {
        let Some(mbc) = &mut self.mbc else {
            return;
        };

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
                self.emulator.execute(mbc.as_mut());
                if let Some(scanline) = self.emulator.get_ppu().get_scanline_if_ready() {
                    for (src, dst) in scanline.iter().zip(
                        current_frame[usize::from(self.emulator.state.ly) * usize::from(WIDTH)..]
                            .iter_mut(),
                    ) {
                        *dst = *src as u8;
                    }
                    if self.emulator.state.ly == HEIGHT - 1 {
                        let this = JsValue::null();
                        if let Err(err) = on_new_frame.call0(&this) {
                            console::error_1(&err);
                        }
                    }
                }
            }
            let sampler = self.emulator.get_apu().get_sampler();
            let sample = self.sample_index as f32 / sample_rate as f32;
            *left = sampler.sample_left(
                sample,
                &self.noise,
                &self.short_noise,
                &mut self.wave_corrector_left,
            );
            *right = sampler.sample_right(
                sample,
                &self.noise,
                &self.short_noise,
                &mut self.wave_corrector_right,
            );
            // 2 minutes without popping (sample_index must not be huge to prevent precision errors)
            self.sample_index = self.sample_index.wrapping_add(1) % (sample_rate * 2 * 60);
        }
    }

    pub fn load_rom(&mut self, rom: Vec<u8>, save: Option<Vec<u8>>) {
        console::log_1(&JsValue::from_str("Loading rom"));
        let Some((cartridge_type, mut mbc)) = get_mbc::<_, NullRtc>(rom) else {
            console::error_1(&JsValue::from_str("MBC type not recognized"));
            return;
        };
        if let Some(save) = save {
            console::log_1(&JsValue::from_str("Loading save"));
            mbc.load_saved_ram(&save);
        }
        console::log_1(&JsValue::from_str("Rom loaded!"));

        if cartridge_type.has_battery() {
            console::log_1(&JsValue::from_str("Saves enabled"));
        }

        *self = Self {
            mbc: Some(mbc),
            is_save_enabled: cartridge_type.has_battery(),
            ..Default::default()
        };
    }

    pub fn get_save(&self) -> Option<Save> {
        if !self.is_save_enabled {
            return None;
        }

        let mbc = self.mbc.as_deref()?;

        Some(Save {
            ram: mbc.get_ram_to_save()?.into(),
            game_title: get_title_from_rom(mbc.get_rom()).to_owned(),
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
