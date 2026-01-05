//! Plays a simple 440 Hz sine wave (beep) tone.
//!
//! This example demonstrates:
//! - Selecting audio hosts (with optional JACK support on Linux)
//! - Selecting devices by ID or using the default output device
//! - Querying the default output configuration
//! - Building and running an output stream with typed samples
//! - Generating audio data in the stream callback
//!
//! Run with: `cargo run --example beep`
//! With JACK (Linux): `cargo run --example beep --features jack -- --jack`
//! With specific device: `cargo run --example beep -- --device "wasapi:device_id"`

use std::{
    collections::HashSet,
    sync::{Arc, RwLock},
};

use cpal::{
    FromSample, I24, SizedSample,
    traits::{DeviceTrait, StreamTrait},
};
use gebeh_core::apu::Apu;

// for sampling reasons we have to generate the noise values at program start
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

pub struct Audio {
    // we must avoid updating it every cycle...
    apu: Arc<RwLock<Apu>>,
    _stream: cpal::Stream,
}

impl Audio {
    pub fn new(device: &cpal::Device) -> Self {
        let apu_to_keep: Arc<RwLock<Apu>> = Default::default();
        let config = device.default_output_config().unwrap();
        let apu = apu_to_keep.clone();
        let stream = match config.sample_format() {
            cpal::SampleFormat::I8 => Self::create_stream::<i8>(device, &config.into(), apu),
            cpal::SampleFormat::I16 => Self::create_stream::<i16>(device, &config.into(), apu),
            cpal::SampleFormat::I24 => Self::create_stream::<I24>(device, &config.into(), apu),
            cpal::SampleFormat::I32 => Self::create_stream::<i32>(device, &config.into(), apu),
            // cpal::SampleFormat::I48 => run::<I48>(&device, &config.into()),
            cpal::SampleFormat::I64 => Self::create_stream::<i64>(device, &config.into(), apu),
            cpal::SampleFormat::U8 => Self::create_stream::<u8>(device, &config.into(), apu),
            cpal::SampleFormat::U16 => Self::create_stream::<u16>(device, &config.into(), apu),
            // cpal::SampleFormat::U24 => run::<U24>(&device, &config.into()),
            cpal::SampleFormat::U32 => Self::create_stream::<u32>(device, &config.into(), apu),
            // cpal::SampleFormat::U48 => run::<U48>(&device, &config.into()),
            cpal::SampleFormat::U64 => Self::create_stream::<u64>(device, &config.into(), apu),
            cpal::SampleFormat::F32 => Self::create_stream::<f32>(device, &config.into(), apu),
            cpal::SampleFormat::F64 => Self::create_stream::<f64>(device, &config.into(), apu),
            sample_format => panic!("Unsupported sample format '{sample_format}'"),
        };
        stream.play().unwrap();

        Self {
            apu: apu_to_keep,
            _stream: stream,
        }
    }

    fn create_stream<T>(
        device: &cpal::Device,
        config: &cpal::StreamConfig,
        apu: Arc<RwLock<Apu>>,
    ) -> cpal::Stream
    where
        T: SizedSample + FromSample<f32>,
    {
        let sample_rate = config.sample_rate;
        let channels = config.channels as usize;
        let mut sample_index = 0u32;

        let noise = get_noise(false);
        let short_noise = get_noise(true);

        device
            .build_output_stream(
                config,
                move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                    let apu = apu.read().unwrap();
                    for frame in data.chunks_mut(channels) {
                        let sample = sample_index as f32 / sample_rate as f32;
                        let left = T::from_sample(apu.sample_left(sample, &noise, &short_noise));
                        let right = T::from_sample(apu.sample_right(sample, &noise, &short_noise));
                        sample_index = (sample_index + 1) % sample_rate;
                        for (index, sample) in frame.iter_mut().enumerate() {
                            // even is left, odd is right
                            *sample = if index % 2 == 0 { left } else { right }
                        }
                    }
                },
                |err| eprintln!("an error occurred on stream: {err}"),
                None,
            )
            .unwrap()
    }

    pub fn update_sound(&mut self, apu: Apu) {
        *self.apu.write().unwrap() = apu;
    }
}
