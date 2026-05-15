use std::{cell::Cell, rc::Rc};

use arrayvec::ArrayVec;
use gebeh_core::{
    Cgb, Dmg, Emulator, EmulatorExt, HEIGHT, Model, SYSTEM_CLOCK_FREQUENCY, WIDTH, apu::Mixer,
    joypad::JoypadInput, ppu::scanline::Scanline, serial::Serial,
};
use gebeh_front_helper::{
    Compatibility, EasyMbc, get_compatibility, get_mbc, get_noise, get_title_from_rom,
};
use wasm_bindgen::prelude::*;
use web_sys::{
    console,
    js_sys::{self},
};

use gebeh_network::{RollbackSerial, message::SerialMessage};

use crate::rtc::AudioRtc;

mod rtc;

#[wasm_bindgen]
#[derive(Default, Clone, Copy)]
pub enum Mode {
    #[default]
    CgbWhenExplicit,
    DmgWhenPossible,
    AlwaysCgb,
}

#[derive(Default)]
#[allow(clippy::large_enum_variant)]
enum Inner {
    Dmg(WebEmulatorInner<Dmg>),
    Cgb(WebEmulatorInner<Cgb>),
    NetworkPreEnabled,
    #[default]
    None,
}

struct WebEmulatorInner<M: Model> {
    emulator: Emulator<M>,
    sample_index: u32,
    mbc: EasyMbc,
    // to iterate SYSTEM_CLOCK_FREQUENCY / sample_rate on average even if the division is not round
    error: u32,
    is_save_enabled: bool,
    mixer: Mixer<Vec<u8>>,
    current_frame: [u16; WIDTH as usize * HEIGHT as usize],
    start_time: u64,
    seconds_since_epoch: Rc<Cell<u64>>,
    network: Option<RollbackSerial<M>>,
}

#[wasm_bindgen]
#[derive(Default)]
pub struct WebEmulator {
    inner: Inner,
    mode: Mode,
}

impl<M: Model> WebEmulatorInner<M> {
    fn set_joypad(&mut self, joypad: JoypadInput) {
        if self.emulator.get_joypad() == &joypad {
            return;
        }

        self.emulator.set_joypad(joypad);
    }

    pub fn new(
        rom: Box<[u8]>,
        save: Option<Box<[u8]>>,
        extra: Option<Box<[u8]>>,
        sample_rate: f32,
        seconds_since_epoch: u32,
        audio_time: u32,
        enable_network: bool,
    ) -> Option<Self> {
        console::log_1(&JsValue::from_str("Loading rom"));
        let start_time = seconds_since_epoch - audio_time;
        let seconds_since_epoch = Rc::new(Cell::new(u64::from(seconds_since_epoch)));
        // rc to easily clone the mbc for the rollback netcode
        let Some((cartridge_type, mut mbc)) =
            get_mbc(Rc::from(rom), AudioRtc::new(seconds_since_epoch.clone()))
        else {
            console::error_1(&JsValue::from_str("MBC type not recognized"));
            return None;
        };
        if let Some(save) = save {
            console::log_1(&JsValue::from_str("Loading save"));
            mbc.load_saved_ram(&save);
        }
        if let Some(extra) = extra {
            console::log_1(&JsValue::from_str("Loading extra"));
            mbc.load_additional_data(&extra);
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
            start_time: u64::from(start_time),
            seconds_since_epoch,
            network: if enable_network {
                Some(Default::default())
            } else {
                None
            },
        })
    }

    // this function is executed every 128 (RENDER_QUANTUM_SIZE) frames
    #[must_use]
    pub fn drive_and_sample(
        &mut self,
        left: &mut [f32],
        right: &mut [f32],
        sample_rate: u32,
        audio_time: u32,
        on_new_frame: &js_sys::Function,
    ) -> Box<[u8]> {
        let mut messages = ArrayVec::<SerialMessage, 4>::new();
        let base = SYSTEM_CLOCK_FREQUENCY / sample_rate;
        let remainder = SYSTEM_CLOCK_FREQUENCY % sample_rate;

        self.seconds_since_epoch
            .set(self.start_time + u64::from(audio_time));

        for (left, right) in left.iter_mut().zip(right.iter_mut()) {
            let mut cycles = base;
            self.error += remainder;

            if let Some(error) = self.error.checked_sub(sample_rate) {
                self.error = error;
                cycles += 1;
            }

            if let Some(synchro) = self.network.as_mut() {
                synchro.rollback_if_necessary(&mut self.emulator, &mut self.mbc);
            }

            for _ in 0..cycles {
                if let Some(synchro) = self.network.as_mut() {
                    messages.extend(
                        synchro.execute_and_take_snapshot(&mut self.emulator, self.mbc.as_mut()),
                    );
                } else {
                    self.emulator.execute(self.mbc.as_mut());
                }
                self.handle_graphics(on_new_frame);
            }

            (*left, *right) = self.handle_sound(sample_rate);
        }

        SerialMessage::serialize(&messages)
    }

    fn handle_sound(&mut self, sample_rate: u32) -> (f32, f32) {
        let sample = self.sample_index as f32 / sample_rate as f32;
        let mut sampler = self
            .mixer
            .mix(self.emulator.get_apu().get_sampler(), sample);
        // 2 minutes without popping (sample_index must not be huge to prevent precision errors)
        self.sample_index = self.sample_index.wrapping_add(1) % (sample_rate * 2 * 60);
        (sampler.sample_left(), sampler.sample_right())
    }

    fn handle_graphics(&mut self, on_new_frame: &js_sys::Function) {
        let Some(scanline) = self.emulator.get_ppu().get_scanline_if_ready() else {
            return;
        };

        for (input, color) in self.current_frame.as_chunks_mut::<160>().0
            [usize::from(self.emulator.get_ppu().get_ly())]
        .iter_mut()
        .zip(scanline.iter_colors())
        {
            *input = color.into();
        }

        if self.emulator.get_ppu().get_ly() == HEIGHT - 1
            && let Err(err) = on_new_frame.call1(
                &JsValue::null(),
                &js_sys::Uint16Array::new_from_slice(&self.current_frame),
            )
        {
            console::error_1(&err);
        }
    }

    pub fn get_save(&self) -> Option<Save> {
        if !self.is_save_enabled {
            return None;
        }

        let mut extra_buffer = [0; 64];
        let count = self.mbc.get_additional_data_to_save(&mut extra_buffer);

        Some(Save {
            ram: self.mbc.get_ram_to_save()?.into(),
            extra: if count > 0 {
                Some(extra_buffer[0..count].into())
            } else {
                None
            },
            game_title: get_title_from_rom(self.mbc.get_rom()).to_owned(),
        })
    }
}

#[wasm_bindgen]
impl WebEmulator {
    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
    }

    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        wasm_logger::init(wasm_logger::Config::new(log::Level::Info));
        Default::default()
    }

    pub fn init_emulator(
        &mut self,
        rom: Box<[u8]>,
        save: Option<Box<[u8]>>,
        extra: Option<Box<[u8]>>,
        sample_rate: f32,
        seconds_since_epoch: u32,
        audio_time: u32,
    ) {
        let network_enabled = match &self.inner {
            Inner::Dmg(web_emulator_inner) => web_emulator_inner.network.is_some(),
            Inner::Cgb(web_emulator_inner) => web_emulator_inner.network.is_some(),
            Inner::NetworkPreEnabled => true,
            Inner::None => false,
        };

        let inner = match (get_compatibility(&rom), self.mode) {
            (Compatibility::Dmg, Mode::CgbWhenExplicit | Mode::DmgWhenPossible)
            | (Compatibility::Both, Mode::DmgWhenPossible) => WebEmulatorInner::new(
                rom,
                save,
                extra,
                sample_rate,
                seconds_since_epoch,
                audio_time,
                network_enabled,
            )
            .map(Inner::Dmg),
            (Compatibility::Cgb, _)
            | (_, Mode::AlwaysCgb)
            | (Compatibility::Both, Mode::CgbWhenExplicit) => WebEmulatorInner::new(
                rom,
                save,
                extra,
                sample_rate,
                seconds_since_epoch,
                audio_time,
                network_enabled,
            )
            .map(Inner::Cgb),
        };

        self.inner = inner.unwrap_or(if network_enabled {
            Inner::NetworkPreEnabled
        } else {
            Inner::None
        });
    }

    // this function is executed every 128 (RENDER_QUANTUM_SIZE) frames
    pub fn drive_and_sample(
        &mut self,
        left: &mut [f32],
        right: &mut [f32],
        sample_rate: u32,
        audio_time: u32,
        on_new_frame: &js_sys::Function,
    ) -> Option<Box<[u8]>> {
        match &mut self.inner {
            Inner::Dmg(web_emulator_inner) => Some(web_emulator_inner.drive_and_sample(
                left,
                right,
                sample_rate,
                audio_time,
                on_new_frame,
            )),
            Inner::Cgb(web_emulator_inner) => Some(web_emulator_inner.drive_and_sample(
                left,
                right,
                sample_rate,
                audio_time,
                on_new_frame,
            )),
            Inner::NetworkPreEnabled => None,
            Inner::None => None,
        }
    }

    pub fn get_save(&self) -> Option<Save> {
        match &self.inner {
            Inner::Dmg(web_emulator_inner) => web_emulator_inner.get_save(),
            Inner::Cgb(web_emulator_inner) => web_emulator_inner.get_save(),
            Inner::NetworkPreEnabled => None,
            Inner::None => None,
        }
    }

    pub fn set_a(&mut self, value: bool) {
        match &mut self.inner {
            Inner::Dmg(web_emulator_inner) => web_emulator_inner.set_joypad(JoypadInput {
                a: value,
                ..*web_emulator_inner.emulator.get_joypad()
            }),
            Inner::Cgb(web_emulator_inner) => web_emulator_inner.set_joypad(JoypadInput {
                a: value,
                ..*web_emulator_inner.emulator.get_joypad()
            }),
            Inner::NetworkPreEnabled => {}
            Inner::None => {}
        }
    }
    pub fn set_b(&mut self, value: bool) {
        match &mut self.inner {
            Inner::Dmg(web_emulator_inner) => web_emulator_inner.set_joypad(JoypadInput {
                b: value,
                ..*web_emulator_inner.emulator.get_joypad()
            }),
            Inner::Cgb(web_emulator_inner) => web_emulator_inner.set_joypad(JoypadInput {
                b: value,
                ..*web_emulator_inner.emulator.get_joypad()
            }),
            Inner::NetworkPreEnabled => {}
            Inner::None => {}
        }
    }
    pub fn set_start(&mut self, value: bool) {
        match &mut self.inner {
            Inner::Dmg(web_emulator_inner) => web_emulator_inner.set_joypad(JoypadInput {
                start: value,
                ..*web_emulator_inner.emulator.get_joypad()
            }),
            Inner::Cgb(web_emulator_inner) => web_emulator_inner.set_joypad(JoypadInput {
                start: value,
                ..*web_emulator_inner.emulator.get_joypad()
            }),
            Inner::NetworkPreEnabled => {}
            Inner::None => {}
        }
    }
    pub fn set_select(&mut self, value: bool) {
        match &mut self.inner {
            Inner::Dmg(web_emulator_inner) => web_emulator_inner.set_joypad(JoypadInput {
                select: value,
                ..*web_emulator_inner.emulator.get_joypad()
            }),
            Inner::Cgb(web_emulator_inner) => web_emulator_inner.set_joypad(JoypadInput {
                select: value,
                ..*web_emulator_inner.emulator.get_joypad()
            }),
            Inner::NetworkPreEnabled => {}
            Inner::None => {}
        }
    }
    pub fn set_left(&mut self, value: bool) {
        match &mut self.inner {
            Inner::Dmg(web_emulator_inner) => web_emulator_inner.set_joypad(JoypadInput {
                left: value,
                ..*web_emulator_inner.emulator.get_joypad()
            }),
            Inner::Cgb(web_emulator_inner) => web_emulator_inner.set_joypad(JoypadInput {
                left: value,
                ..*web_emulator_inner.emulator.get_joypad()
            }),
            Inner::NetworkPreEnabled => {}
            Inner::None => {}
        }
    }
    pub fn set_right(&mut self, value: bool) {
        match &mut self.inner {
            Inner::Dmg(web_emulator_inner) => web_emulator_inner.set_joypad(JoypadInput {
                right: value,
                ..*web_emulator_inner.emulator.get_joypad()
            }),
            Inner::Cgb(web_emulator_inner) => web_emulator_inner.set_joypad(JoypadInput {
                right: value,
                ..*web_emulator_inner.emulator.get_joypad()
            }),
            Inner::NetworkPreEnabled => {}
            Inner::None => {}
        }
    }
    pub fn set_down(&mut self, value: bool) {
        match &mut self.inner {
            Inner::Dmg(web_emulator_inner) => web_emulator_inner.set_joypad(JoypadInput {
                down: value,
                ..*web_emulator_inner.emulator.get_joypad()
            }),
            Inner::Cgb(web_emulator_inner) => web_emulator_inner.set_joypad(JoypadInput {
                down: value,
                ..*web_emulator_inner.emulator.get_joypad()
            }),
            Inner::NetworkPreEnabled => {}
            Inner::None => {}
        }
    }
    pub fn set_up(&mut self, value: bool) {
        match &mut self.inner {
            Inner::Dmg(web_emulator_inner) => web_emulator_inner.set_joypad(JoypadInput {
                up: value,
                ..*web_emulator_inner.emulator.get_joypad()
            }),
            Inner::Cgb(web_emulator_inner) => web_emulator_inner.set_joypad(JoypadInput {
                up: value,
                ..*web_emulator_inner.emulator.get_joypad()
            }),
            Inner::NetworkPreEnabled => {}
            Inner::None => {}
        }
    }

    pub fn add_serial_message(&mut self, message: Box<[u8]>) -> Option<Box<[u8]>> {
        match &mut self.inner {
            Inner::Dmg(web_emulator_inner) => {
                web_emulator_inner
                    .network
                    .as_mut()
                    .expect("No synchro")
                    .add_messages(&message);
                None
            }
            Inner::Cgb(web_emulator_inner) => {
                web_emulator_inner
                    .network
                    .as_mut()
                    .expect("No synchro")
                    .add_messages(&message);
                None
            }
            Inner::NetworkPreEnabled => {
                let msg = gebeh_network::handle_msg_no_emulator(&message)?;
                Some(SerialMessage::serialize(&[msg]))
            }
            Inner::None => panic!("No synchro"),
        }
    }

    pub fn set_is_serial_connected(&mut self, is_connected: bool) {
        if is_connected {
            match &mut self.inner {
                Inner::Dmg(web_emulator_inner) => {
                    web_emulator_inner.network = Some(Default::default())
                }
                Inner::Cgb(web_emulator_inner) => {
                    web_emulator_inner.network = Some(Default::default())
                }
                Inner::NetworkPreEnabled => {}
                Inner::None => self.inner = Inner::NetworkPreEnabled,
            }
        } else {
            match &mut self.inner {
                Inner::Dmg(web_emulator_inner) => {
                    web_emulator_inner.emulator.serial.set_slave_byte(0xff);
                    web_emulator_inner.network = None
                }
                Inner::Cgb(web_emulator_inner) => {
                    web_emulator_inner.emulator.serial.set_slave_byte(0xff);
                    web_emulator_inner.network = None
                }
                Inner::NetworkPreEnabled => self.inner = Inner::None,
                Inner::None => {}
            }
        }
    }

    pub fn get_cycles(&self) -> u64 {
        match &self.inner {
            Inner::Dmg(web_emulator_inner) => web_emulator_inner.emulator.get_cycles(),
            Inner::Cgb(web_emulator_inner) => web_emulator_inner.emulator.get_cycles(),
            Inner::NetworkPreEnabled => 0,
            Inner::None => 0,
        }
    }
}

#[wasm_bindgen]
pub struct Save {
    ram: Box<[u8]>,
    extra: Option<Box<[u8]>>,
    game_title: String,
}

#[wasm_bindgen]
impl Save {
    pub fn get_ram(&self) -> Box<[u8]> {
        self.ram.clone()
    }

    pub fn get_extra(&self) -> Option<Box<[u8]>> {
        self.extra.clone()
    }

    pub fn get_game_title(&self) -> String {
        self.game_title.clone()
    }
}
