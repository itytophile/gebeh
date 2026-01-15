use std::collections::HashSet;

use gebeh_core::{Emulator, HEIGHT, WIDTH};
use wasm_bindgen::prelude::*;
use web_sys::{
    console,
    js_sys::{self, Uint8Array},
};

use crate::mbc::{get_mbc, CloneMbc};

mod mbc;

#[derive(Default)]
struct LinearFeedbackShiftRegister(u16);

impl LinearFeedbackShiftRegister {
    fn tick(&mut self, short_mode: bool) -> u8 {
        // https://gbdev.io/pandocs/Audio_details.html#noise-channel-ch4
        let new_value = (self.0 & 1 != 0) == (self.0 & 0b10 != 0);
        self.0 = self.0 & 0x7fff | ((new_value as u16) << 15);
        if short_mode {
            self.0 = self.0 & 0xff7f | ((new_value as u16) << 7)
        }
        let shifted_out = self.0 & 1;
        self.0 >>= 1;
        shifted_out as u8
    }
}

fn get_noise(is_short: bool) -> Vec<u8> {
    let mut lfsr = LinearFeedbackShiftRegister::default();
    let mut already_seen = HashSet::new();
    let mut noise = Vec::new();
    while already_seen.insert(lfsr.0) {
        noise.push(lfsr.tick(is_short));
    }
    noise
}

// https://gbdev.io/pandocs/Specifications.html
const SYSTEM_CLOCK_FREQUENCY: u32 = 4194304 / 4;

#[wasm_bindgen]
pub struct WebEmulator {
    emulator: Emulator,
    noise: Vec<u8>,
    short_noise: Vec<u8>,
    sample_index: u32,
    mbc: Option<Box<dyn CloneMbc<'static>>>,
    current_frame: [u8; WIDTH as usize * HEIGHT as usize],
    // to iterate SYSTEM_CLOCK_FREQUENCY / sample_rate on average even if the division is not round
    error: u32,
}

#[wasm_bindgen]
impl WebEmulator {
    #[allow(clippy::new_without_default)]
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            emulator: Default::default(),
            noise: get_noise(false),
            short_noise: get_noise(true),
            sample_index: 0,
            mbc: None,
            current_frame: [0; WIDTH as usize * HEIGHT as usize],
            error: 0,
        }
    }

    // this function is executed every 128 (RENDER_QUANTUM_SIZE) frames
    pub fn drive_and_sample(
        &mut self,
        left: &mut [f32],
        right: &mut [f32],
        sample_rate: u32,
        on_new_frame: &js_sys::Function,
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
                        self.current_frame
                            [usize::from(self.emulator.state.ly) * usize::from(WIDTH)..]
                            .iter_mut(),
                    ) {
                        *dst = *src as u8;
                    }
                    if self.emulator.state.ly == HEIGHT - 1 {
                        let this = JsValue::null();
                        if let Err(err) = on_new_frame.call1(
                            &this,
                            &JsValue::from(Uint8Array::new_from_slice(&self.current_frame)),
                        ) {
                            console::error_1(&err);
                        }
                    }
                }
            }
            let sampler = self.emulator.get_apu().get_sampler();
            let sample = self.sample_index as f32 / sample_rate as f32;
            *left = sampler.sample_left(sample, &self.noise, &self.short_noise);
            *right = sampler.sample_right(sample, &self.noise, &self.short_noise);
            // 2 minutes without popping (sample_index must not be huge to prevent precision errors)
            self.sample_index = self.sample_index.wrapping_add(1) % (sample_rate * 2 * 60);
        }
    }

    pub fn load_rom(&mut self, rom: Vec<u8>) {
        console::log_1(&JsValue::from_str("Loading rom"));
        let Some(mbc) = get_mbc(rom) else {
            console::error_1(&JsValue::from_str("MBC type not recognized"));
            return;
        };
        console::log_1(&JsValue::from_str("Rom loaded!"));
        self.mbc = Some(mbc);
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
