#![deny(clippy::all)]

use std::collections::HashSet;

use gebeh_core::Emulator;
use wasm_bindgen::prelude::*;
use web_sys::console;

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
// https://developer.mozilla.org/en-US/docs/Web/API/AudioWorkletGlobalScope/currentFrame
const RENDER_QUANTUM_SIZE: u32 = 128;

#[wasm_bindgen]
struct WebEmulator {
    emulator: Emulator,
    noise: Vec<u8>,
    short_noise: Vec<u8>,
    sample_index: u32,
    mbc: Option<Box<dyn CloneMbc<'static>>>,
}

#[wasm_bindgen]
impl WebEmulator {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            emulator: Default::default(),
            noise: get_noise(false),
            short_noise: get_noise(true),
            sample_index: 0,
            mbc: None,
        }
    }

    // this function is executed every 128 (RENDER_QUANTUM_SIZE) frames
    pub fn drive_and_sample(&mut self, left: &mut [f32], right: &mut [f32], sample_rate: u32) {
        let Some(mbc) = &mut self.mbc else {
            return;
        };
        // not perfect but whatever
        for _ in 0..(SYSTEM_CLOCK_FREQUENCY * RENDER_QUANTUM_SIZE / sample_rate) {
            self.emulator.execute(mbc.as_mut());
        }
        let sampler = self.emulator.get_apu().get_sampler();
        for (left, right) in left.iter_mut().zip(right.iter_mut()) {
            let sample = self.sample_index as f32 / sample_rate as f32;
            *left = sampler.sample_left(sample, &self.noise, &self.short_noise);
            *right = sampler.sample_right(sample, &self.noise, &self.short_noise);
            self.sample_index = self.sample_index.wrapping_add(1);
        }
    }

    pub fn load_rom(&mut self, rom: Vec<u8>) {
        console::log_1(&JsValue::from_str("Loading rom..."));
        let Some(mbc) = get_mbc(rom) else {
            log::error!("MBC type not recognized");
            return;
        };
        console::log_1(&JsValue::from_str("Rom loaded!"));
        self.mbc = Some(mbc);
    }
}
