use std::rc::Rc;

use gebeh_core::{
    Emulator, HEIGHT, SYSTEM_CLOCK_FREQUENCY, WIDTH, apu::Mixer, joypad::JoypadInput,
};
use gebeh_front_helper::{EasyMbc, get_mbc, get_noise, get_title_from_rom};
use wasm_bindgen::prelude::*;
use web_sys::{
    console,
    js_sys::{self},
};

use gebeh_network::RollbackSerial;

use crate::rtc::NullRtc;

mod rtc;

struct WebEmulatorInner {
    emulator: Emulator,
    sample_index: u32,
    mbc: EasyMbc,
    // to iterate SYSTEM_CLOCK_FREQUENCY / sample_rate on average even if the division is not round
    error: u32,
    is_save_enabled: bool,
    mixer: Mixer<Vec<u8>>,
    current_frame: [u8; WIDTH as usize * HEIGHT as usize],
}

#[wasm_bindgen]
#[derive(Default)]
pub struct WebEmulator {
    inner: Option<WebEmulatorInner>,
    network: Option<(js_sys::Function, RollbackSerial)>,
}

impl WebEmulatorInner {
    fn set_joypad(&mut self, joypad: JoypadInput, synchro: Option<&mut RollbackSerial>) {
        if self.emulator.get_joypad() == &joypad {
            return;
        }

        self.emulator.set_joypad(joypad);

        let Some(synchro) = synchro else {
            return;
        };

        synchro.save_input(self.emulator.get_cycles(), joypad);
    }

    pub fn new(rom: Vec<u8>, save: Option<Vec<u8>>, sample_rate: f32) -> Option<Self> {
        console::log_1(&JsValue::from_str("Loading rom"));
        // rc to easily clone the mbc for the rollback netcode
        let Some((cartridge_type, mut mbc)) =
            get_mbc::<Rc<[u8]>, NullRtc>(Rc::from(rom.into_boxed_slice()))
        else {
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
        serial_mode: &mut Option<(js_sys::Function, RollbackSerial)>,
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
                if let Some((send, synchro)) = serial_mode.as_mut() {
                    if let Some(msg) =
                        synchro.execute_and_take_snapshot(&mut self.emulator, self.mbc.as_mut())
                        && let Err(err) =
                            send.call1(&JsValue::null(), &js_sys::Uint8Array::new_from_slice(&msg))
                    {
                        console::error_1(&err);
                    }
                } else {
                    self.emulator.execute(self.mbc.as_mut());
                }
                self.handle_graphics(on_new_frame);
            }

            (*left, *right) = self.handle_sound(sample_rate);
        }
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

        self.current_frame.as_chunks_mut::<40>().0[usize::from(self.emulator.get_ppu().get_ly())] =
            *scanline.raw();

        if self.emulator.get_ppu().get_ly() == HEIGHT - 1
            && let Err(err) = on_new_frame.call1(
                &JsValue::null(),
                &js_sys::Uint8Array::new_from_slice(&self.current_frame),
            )
        {
            console::error_1(&err);
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
}

#[wasm_bindgen]
impl WebEmulator {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        Default::default()
    }

    pub fn init_emulator(&mut self, rom: Vec<u8>, save: Option<Vec<u8>>, sample_rate: f32) {
        self.inner = WebEmulatorInner::new(rom, save, sample_rate)
    }

    // this function is executed every 128 (RENDER_QUANTUM_SIZE) frames
    pub fn drive_and_sample(
        &mut self,
        left: &mut [f32],
        right: &mut [f32],
        sample_rate: u32,
        on_new_frame: &js_sys::Function,
    ) {
        if let Some(inner) = &mut self.inner {
            inner.drive_and_sample(left, right, sample_rate, on_new_frame, &mut self.network);
        }
    }

    pub fn get_save(&self) -> Option<Save> {
        self.inner.as_ref().and_then(WebEmulatorInner::get_save)
    }

    pub fn set_a(&mut self, value: bool) {
        if let Some(inner) = &mut self.inner {
            inner.set_joypad(
                JoypadInput {
                    a: value,
                    ..*inner.emulator.get_joypad()
                },
                self.network.as_mut().map(|(_, synchro)| synchro),
            );
        }
    }
    pub fn set_b(&mut self, value: bool) {
        if let Some(inner) = &mut self.inner {
            inner.set_joypad(
                JoypadInput {
                    b: value,
                    ..*inner.emulator.get_joypad()
                },
                self.network.as_mut().map(|(_, synchro)| synchro),
            );
        }
    }
    pub fn set_start(&mut self, value: bool) {
        if let Some(inner) = &mut self.inner {
            inner.set_joypad(
                JoypadInput {
                    start: value,
                    ..*inner.emulator.get_joypad()
                },
                self.network.as_mut().map(|(_, synchro)| synchro),
            );
        }
    }
    pub fn set_select(&mut self, value: bool) {
        if let Some(inner) = &mut self.inner {
            inner.set_joypad(
                JoypadInput {
                    select: value,
                    ..*inner.emulator.get_joypad()
                },
                self.network.as_mut().map(|(_, synchro)| synchro),
            );
        }
    }
    pub fn set_left(&mut self, value: bool) {
        if let Some(inner) = &mut self.inner {
            inner.set_joypad(
                JoypadInput {
                    left: value,
                    ..*inner.emulator.get_joypad()
                },
                self.network.as_mut().map(|(_, synchro)| synchro),
            );
        }
    }
    pub fn set_right(&mut self, value: bool) {
        if let Some(inner) = &mut self.inner {
            inner.set_joypad(
                JoypadInput {
                    right: value,
                    ..*inner.emulator.get_joypad()
                },
                self.network.as_mut().map(|(_, synchro)| synchro),
            );
        }
    }
    pub fn set_down(&mut self, value: bool) {
        if let Some(inner) = &mut self.inner {
            inner.set_joypad(
                JoypadInput {
                    down: value,
                    ..*inner.emulator.get_joypad()
                },
                self.network.as_mut().map(|(_, synchro)| synchro),
            );
        }
    }
    pub fn set_up(&mut self, value: bool) {
        if let Some(inner) = &mut self.inner {
            inner.set_joypad(
                JoypadInput {
                    up: value,
                    ..*inner.emulator.get_joypad()
                },
                self.network.as_mut().map(|(_, synchro)| synchro),
            );
        }
    }

    #[must_use]
    pub fn set_serial_msg(&mut self, msg: &[u8]) -> Box<[js_sys::Uint8Array]> {
        let Some((_, synchro)) = &mut self.network else {
            panic!("No synchro");
        };

        if let Some(inner) = &mut self.inner {
            synchro
                .set_serial_msg(msg, &mut inner.emulator, &mut inner.mbc)
                .into_iter()
                .map(|bytes| js_sys::Uint8Array::new_from_slice(&bytes))
                .collect()
        } else {
            RollbackSerial::handle_msg_no_emulator(msg)
                .into_iter()
                .map(|bytes| js_sys::Uint8Array::new_from_slice(&bytes))
                .collect()
        }
    }

    pub fn set_is_serial_connected(&mut self, on_serial: Option<js_sys::Function>) {
        if let Some(on_serial) = on_serial {
            self.network = Some((on_serial.clone(), Default::default()));
        } else {
            self.network = None;
            if let Some(inner) = &mut self.inner {
                inner.emulator.serial.slave_byte = 0xff;
            }
        }
    }

    pub fn get_cycles(&self) -> u64 {
        self.inner
            .as_ref()
            .map_or(0, |inner| inner.emulator.get_cycles())
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
