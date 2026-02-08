use std::sync::{Arc, RwLock, mpsc::SyncSender};

use cpal::{
    BufferSize, FromSample, I24, SizedSample, StreamConfig,
    traits::{DeviceTrait, StreamTrait},
};
use gebeh::{Frame, InstantRtc};
use gebeh_core::{
    Emulator, HEIGHT, SYSTEM_CLOCK_FREQUENCY,
    apu::Mixer,
    joypad::JoypadInput,
    mbc::{CartridgeType, get_factor_8_kib_ram, get_factor_32_kib_rom},
    ppu::Scanline,
};
use gebeh_front_helper::{get_mbc, get_noise, get_title_from_rom};

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
    let rom = std::fs::read(
        std::env::args()
            .nth(1)
            .expect("Please provide a path as first argument"),
    )
    .unwrap();

    println!("Title: {}", get_title_from_rom(&rom));

    // https://gbdev.io/pandocs/The_Cartridge_Header.html#0147--cartridge-type
    let cartridge_type = CartridgeType::try_from(rom[0x147]).unwrap();
    println!("Cartridge type: {cartridge_type:?}");
    // https://gbdev.io/pandocs/The_Cartridge_Header.html#0148--rom-size
    println!("ROM size: {} KiB", get_factor_32_kib_rom(&rom) * 32);
    println!("RAM size: {} KiB", get_factor_8_kib_ram(&rom) * 8);

    // don't forget to use arc or you will clone the rom for each save state
    let (_, mut mbc) = get_mbc::<_, InstantRtc>(Arc::from(rom.into_boxed_slice())).unwrap();
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
    let mut current_frame = [Scanline::default(); HEIGHT as usize];
    let mut mixer = Mixer::new(sample_rate as f32, noise, short_noise);

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
                            current_frame[usize::from(emulator.get_ppu().get_ly())] = *scanline;
                            if emulator.get_ppu().get_ly() == HEIGHT - 1
                                && let Err(std::sync::mpsc::TrySendError::Disconnected(_)) =
                                    shared_frame.try_send(current_frame)
                            {
                                panic!()
                            }
                        }
                    }

                    let sample = sample_index as f32 / sample_rate as f32;
                    let mut sampler = mixer.mix(emulator.get_apu().get_sampler(), sample);

                    frame[0] = T::from_sample(sampler.sample_left());
                    frame[1] = T::from_sample(sampler.sample_right());
                    // 2 minutes without popping (sample_index must not be huge to prevent precision errors)
                    sample_index = sample_index.wrapping_add(1) % (sample_rate * 2 * 60);
                }
            },
            |err| eprintln!("an error occurred on stream: {err}"),
            None,
        )
        .unwrap()
}
