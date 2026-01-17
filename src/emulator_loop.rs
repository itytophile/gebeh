use std::{
    collections::HashSet,
    sync::{Arc, RwLock, mpsc::SyncSender},
};

use cpal::{
    BufferSize, FromSample, I24, SizedSample, StreamConfig,
    traits::{DeviceTrait, StreamTrait},
};
use gebeh::{Frame, InstantRtc};
use gebeh_core::{
    Emulator, HEIGHT, SYSTEM_CLOCK_FREQUENCY, WIDTH,
    joypad::JoypadInput,
    mbc::{CartridgeType, get_factor_8_kib_ram, get_factor_32_kib_rom},
    ppu::Color,
};
use gebeh_front_helper::get_mbc;

pub fn spawn_emulator(
    device: &cpal::Device,
    shared_frame: SyncSender<Frame>,
    shared_joypad: Arc<RwLock<JoypadInput>>,
) -> cpal::Stream {
    let config = device.default_output_config().unwrap();
    let stream = match config.sample_format() {
        cpal::SampleFormat::I8 => {
            create_stream::<i8>(device, config.into(), shared_frame, shared_joypad)
        }
        cpal::SampleFormat::I16 => {
            create_stream::<i16>(device, config.into(), shared_frame, shared_joypad)
        }
        cpal::SampleFormat::I24 => {
            create_stream::<I24>(device, config.into(), shared_frame, shared_joypad)
        }
        cpal::SampleFormat::I32 => {
            create_stream::<i32>(device, config.into(), shared_frame, shared_joypad)
        }
        // cpal::SampleFormat::I48 => run::<I48>(&device, &config.into(),shared_frame),
        cpal::SampleFormat::I64 => {
            create_stream::<i64>(device, config.into(), shared_frame, shared_joypad)
        }
        cpal::SampleFormat::U8 => {
            create_stream::<u8>(device, config.into(), shared_frame, shared_joypad)
        }
        cpal::SampleFormat::U16 => {
            create_stream::<u16>(device, config.into(), shared_frame, shared_joypad)
        }
        // cpal::SampleFormat::U24 => run::<U24>(&device, &config.into(),shared_frame),
        cpal::SampleFormat::U32 => {
            create_stream::<u32>(device, config.into(), shared_frame, shared_joypad)
        }
        // cpal::SampleFormat::U48 => run::<U48>(&device, &config.into(),shared_frame),
        cpal::SampleFormat::U64 => {
            create_stream::<u64>(device, config.into(), shared_frame, shared_joypad)
        }
        cpal::SampleFormat::F32 => {
            create_stream::<f32>(device, config.into(), shared_frame, shared_joypad)
        }
        cpal::SampleFormat::F64 => {
            create_stream::<f64>(device, config.into(), shared_frame, shared_joypad)
        }
        sample_format => panic!("Unsupported sample format '{sample_format}'"),
    };
    stream.play().unwrap();

    stream
}

fn create_stream<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    shared_frame: SyncSender<Frame>,
    shared_joypad: Arc<RwLock<JoypadInput>>,
) -> cpal::Stream
where
    T: SizedSample + FromSample<f32>,
{
    // let rom = std::fs::read("/home/ityt/Téléchargements/dmg-acid2.gb").unwrap();
    // let rom = std::fs::read("/home/ityt/Téléchargements/pocket/pocket.gb").unwrap();
    // let rom = std::fs::read("/home/ityt/Téléchargements/gejmboj/gejmboj.gb").unwrap();
    // let rom = std::fs::read("/home/ityt/Téléchargements/oh_2/oh.gb").unwrap();
    // let rom = std::fs::read("/home/ityt/Téléchargements/20y/20y.gb").unwrap();
    let rom = std::fs::read("/home/ityt/Téléchargements/mallard/mallard.gb").unwrap();
    // https://gbdev.io/pandocs/The_Cartridge_Header.html#0134-0143--title
    let title = &rom[0x134..0x143];
    let end_zero_pos = title
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(title.len());
    let title = str::from_utf8(&title[..end_zero_pos]).unwrap();
    println!("Title: {title}");

    // https://gbdev.io/pandocs/The_Cartridge_Header.html#0147--cartridge-type
    let cartridge_type = CartridgeType::try_from(rom[0x147]).unwrap();
    println!("Cartridge type: {cartridge_type:?}");
    // https://gbdev.io/pandocs/The_Cartridge_Header.html#0148--rom-size
    println!("ROM size: {} KiB", get_factor_32_kib_rom(&rom) * 32);
    println!("RAM size: {} KiB", get_factor_8_kib_ram(&rom) * 8);

    // don't forget to use arc or you will clone the rom for each save state
    let mut mbc = get_mbc::<_, InstantRtc>(Arc::from(rom.into_boxed_slice())).unwrap();
    let mut emulator = Emulator::default();

    let config = StreamConfig {
        channels: 2,
        // same as web
        buffer_size: BufferSize::Fixed(128),
        ..config
    };

    let sample_rate = config.sample_rate;
    let mut sample_index = 0u32;

    let noise = get_noise(false);
    let short_noise = get_noise(true);

    let base = SYSTEM_CLOCK_FREQUENCY / sample_rate;
    let remainder = SYSTEM_CLOCK_FREQUENCY % sample_rate;
    let mut error = 0;
    let mut current_frame = [Color::Black; WIDTH as usize * HEIGHT as usize];

    device
        .build_output_stream(
            &config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                if let Ok(input) = shared_joypad.try_read() {
                    *emulator.get_joypad_mut() = *input;
                }
                for frame in data.as_chunks_mut::<2>().0 {
                    let mut cycles = base;
                    error += remainder;

                    if let Some(new_error) = error.checked_sub(sample_rate) {
                        error = new_error;
                        cycles += 1;
                    }

                    for _ in 0..cycles {
                        emulator.execute(mbc.as_mut());
                        if let Some(scanline) = emulator.get_ppu().get_scanline_if_ready() {
                            for (src, dst) in scanline.iter().zip(
                                current_frame
                                    [usize::from(emulator.state.ly) * usize::from(WIDTH)..]
                                    .iter_mut(),
                            ) {
                                *dst = *src;
                            }
                            if emulator.state.ly == HEIGHT - 1
                                && let Err(std::sync::mpsc::TrySendError::Disconnected(_)) =
                                    shared_frame.try_send(current_frame)
                            {
                                panic!()
                            }
                        }
                    }

                    let sampler = emulator.get_apu().get_sampler();

                    let sample = sample_index as f32 / sample_rate as f32;
                    frame[0] = T::from_sample(sampler.sample_left(sample, &noise, &short_noise));
                    frame[1] = T::from_sample(sampler.sample_right(sample, &noise, &short_noise));
                    // 2 minutes without popping (sample_index must not be huge to prevent precision errors)
                    sample_index = sample_index.wrapping_add(1) % (sample_rate * 2 * 60);
                }
            },
            |err| eprintln!("an error occurred on stream: {err}"),
            None,
        )
        .unwrap()
}

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
