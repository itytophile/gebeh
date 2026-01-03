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

use std::sync::{Arc, RwLock};

use cpal::{
    FromSample, I24, SizedSample,
    traits::{DeviceTrait, StreamTrait},
};
use gebeh_core::{Emulator, apu::Apu};

pub struct Audio {
    falling_edge: bool,
    // we must avoid updating it every cycle...
    apu: Arc<RwLock<Apu>>,
    stream: cpal::Stream,
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
            falling_edge: false,
            stream,
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

        device
            .build_output_stream(
                config,
                move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                    let apu = apu.read().unwrap();
                    for frame in data.chunks_mut(channels) {
                        let left = T::from_sample(apu.sample_left(sample_rate, sample_index));
                        let right = T::from_sample(apu.sample_right(sample_rate, sample_index));
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

    pub fn update_sound(&mut self, emulator: &Emulator) {
        // 256Hz
        let has_ticked = emulator.get_timer().get_div() & (1 << 5) != 0;

        if self.falling_edge == has_ticked {
            return;
        }

        self.falling_edge = has_ticked;

        if self.falling_edge {
            return;
        }

        *self.apu.write().unwrap() = emulator.get_apu().clone();
    }
}
