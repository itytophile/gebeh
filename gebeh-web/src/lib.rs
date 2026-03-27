use std::rc::Rc;

use arraydeque::ArrayDeque;
use gebeh_core::{
    Emulator, HEIGHT, SYSTEM_CLOCK_FREQUENCY, WIDTH, apu::Mixer, joypad::JoypadInput,
};
use gebeh_front_helper::{EasyMbc, get_mbc, get_noise, get_title_from_rom};
use wasm_bindgen::prelude::*;
use web_sys::{
    console,
    js_sys::{self},
};

use crate::{
    message::{
        ArchivedMessageFromMaster, ArchivedSerialMessage, MessageFromMaster, MessageFromSlave,
        SerialMessage,
    },
    rtc::NullRtc,
};

mod message;
mod rtc;

type Snapshots = ArrayDeque<Snapshot, MAX_SNAPSHOT>;

struct WebEmulatorInner {
    emulator: Emulator,
    sample_index: u32,
    mbc: EasyMbc,
    // to iterate SYSTEM_CLOCK_FREQUENCY / sample_rate on average even if the division is not round
    error: u32,
    is_save_enabled: bool,
    mixer: Mixer<Vec<u8>>,
    current_frame: [u8; WIDTH as usize * HEIGHT as usize],
    synchro_cycles: Option<SynchroCycles>,
    snapshots: Box<Snapshots>,
    session: bool,
    // it's not the actual input value at a given cycle, but WHEN the input changes
    // to avoid saving inputs every cycle
    inputs_history: Box<ArrayDeque<(u64, JoypadInput), INPUTS_HISTORY_SIZE>>,
}

struct SynchroCycles {
    master: u64,
    slave: u64,
}

#[wasm_bindgen]
#[derive(Default)]
pub struct WebEmulator {
    inner: Option<WebEmulatorInner>,
    serial_mode: SerialMode,
}

impl WebEmulatorInner {
    fn set_joypad(&mut self, joypad: JoypadInput) {
        if self.emulator.get_joypad() == &joypad {
            return;
        }

        *self.emulator.get_joypad_mut() = joypad;

        let Err(arraydeque::CapacityError { element }) = self
            .inputs_history
            .push_back((self.emulator.get_cycles(), joypad))
        else {
            return;
        };

        self.inputs_history.pop_front();
        self.inputs_history.push_back(element).unwrap();
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
            synchro_cycles: None,
            snapshots: Default::default(),
            session: false,
            inputs_history: Default::default(),
        })
    }

    // this function is executed every 128 (RENDER_QUANTUM_SIZE) frames
    pub fn drive_and_sample(
        &mut self,
        left: &mut [f32],
        right: &mut [f32],
        sample_rate: u32,
        on_new_frame: &js_sys::Function,
        serial_mode: &mut SerialMode,
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
                self.execute_and_take_snapshot(serial_mode);
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

    pub fn set_serial_msg(
        &mut self,
        msg: &ArchivedSerialMessage,
        serial_mode: &mut SerialMode,
    ) -> Option<MessageFromSlave> {
        let SerialMode::SynchroSerial(synchro_serial) = serial_mode else {
            panic!("no serial set despite receiving a message");
        };

        match msg {
            ArchivedSerialMessage::FromMaster(msg) => {
                let current_joypad = *self.emulator.get_joypad_mut();
                let response = self.handle_message_from_master(serial_mode, msg);
                *self.emulator.get_joypad_mut() = current_joypad;
                response
            }
            ArchivedSerialMessage::FromSlave(msg) => {
                let (mut emulator, mbc) = core::mem::take(&mut synchro_serial.snapshots)
                    .into_iter()
                    .find(|(emulator, _)| emulator.get_cycles() == msg.cycle)
                    .expect("desync too big");
                *emulator.get_joypad_mut() = *self.emulator.get_joypad_mut();
                self.emulator = emulator;
                self.mbc = mbc;
                console_log(&format!(
                    "Correction from slave 0x{:02x} -> 0x{:02x}",
                    self.emulator.serial.slave_byte, msg.correction
                ));
                console_log(&format!(
                    "Will emit serial {}",
                    self.emulator.will_serial_emit_byte()
                ));

                self.emulator.serial.slave_byte = msg.correction;
                synchro_serial.current_message.session = !synchro_serial.current_message.session;
                synchro_serial.current_message.messages.clear();
                self.emulator.execute(self.mbc.as_mut());

                // TODO catchup, however the master will send a lot of messages without being able
                // to receive the slave ones

                None
            }
        }
    }

    fn execute_and_take_snapshot(&mut self, serial_mode: &mut SerialMode) {
        serial_mode.execute(self.emulator.serial.slave_byte, self.emulator.get_cycles());

        if let SerialMode::SynchroSerial(synchro) = serial_mode
            && self.emulator.will_serial_emit_byte()
        {
            let emulator_clone = self.emulator.clone();
            let mbc = self.mbc.clone_boxed();
            let byte = self.emulator.execute(self.mbc.as_mut()).unwrap();
            synchro
                .current_message
                .messages
                .push((byte, emulator_clone, mbc));
        } else {
            self.emulator.execute(self.mbc.as_mut());
        }

        let cycles = self.emulator.get_cycles();
        if cycles.is_multiple_of(ROLLBACK_SNAPSHOT_PERIOD) {
            self.add_snapshot()
        }
    }

    fn handle_message_from_master(
        &mut self,
        serial_mode: &mut SerialMode,
        msg: &ArchivedMessageFromMaster,
    ) -> Option<MessageFromSlave> {
        if msg.session != self.session {
            console::log_1(&JsValue::from_str("Bad session"));
            return None;
        }

        let Some(synchro_cycles) = self.synchro_cycles.as_mut() else {
            console_log("first batch");
            let slave_cycles = self.emulator.get_cycles();
            self.synchro_cycles = Some(SynchroCycles {
                master: msg.first_message.1.to_native(),
                slave: slave_cycles,
            });

            let msg_from_slave =
                self.advance_while_consuming_messages(msg, serial_mode, &mut [].as_slice());

            self.add_snapshot();

            return msg_from_slave.inspect(|_| self.session = !self.session);
        };

        let current_cycle = self.emulator.get_cycles();

        let restore_cycle =
            synchro_cycles.slave + msg.first_message.1.to_native() - synchro_cycles.master;

        let snapshots = core::mem::take(&mut self.snapshots);

        if let Some((emulator, mbc)) = snapshots
            .into_iter()
            .rev()
            .find(|(emulator, _)| emulator.get_cycles() <= restore_cycle)
        {
            self.emulator = emulator;
            self.mbc = mbc;
            self.add_snapshot();
        } else {
            panic!("big delay");
        };

        self.synchro_cycles = Some(SynchroCycles {
            master: self.synchro_cycles.as_ref().unwrap().master + self.emulator.get_cycles()
                - self.synchro_cycles.as_ref().unwrap().slave,
            slave: self.emulator.get_cycles(),
        });

        let inputs_history: Vec<_> = self
            .inputs_history
            .iter()
            .filter(|(cycle, _)| *cycle > self.emulator.get_cycles())
            .copied()
            .collect();
        let mut inputs_history = inputs_history.as_slice();

        let msg_from_slave =
            self.advance_while_consuming_messages(msg, serial_mode, &mut inputs_history);

        self.add_snapshot();

        if msg_from_slave.is_none() && current_cycle > self.emulator.get_cycles() {
            // catching up
            for _ in 0..(current_cycle - self.emulator.get_cycles()) {
                self.execute_and_take_snapshot(serial_mode);
                if let Some((cycle, input)) = inputs_history.first()
                    && *cycle == self.emulator.get_cycles()
                {
                    *self.emulator.get_joypad_mut() = *input;
                    inputs_history = &inputs_history[1..];
                }
            }
        }

        msg_from_slave.inspect(|_| self.session = !self.session)
    }

    fn advance_while_consuming_messages(
        &mut self,
        msg: &ArchivedMessageFromMaster,
        serial_mode: &mut SerialMode,
        inputs_history: &mut &[(u64, JoypadInput)],
    ) -> Option<MessageFromSlave> {
        for (byte, master_cycle) in std::iter::once(&msg.first_message)
            .chain(msg.messages.iter())
            .map(|a| (a.0, a.1.to_native()))
        {
            for _ in 0..master_cycle - self.synchro_cycles.as_ref().unwrap().master {
                self.execute_and_take_snapshot(serial_mode);
                if let Some((cycle, input)) = inputs_history.first()
                    && *cycle == self.emulator.get_cycles()
                {
                    *self.emulator.get_joypad_mut() = *input;
                    *inputs_history = &inputs_history[1..];
                }
            }

            self.synchro_cycles = Some(SynchroCycles {
                master: master_cycle,
                slave: self.emulator.get_cycles(),
            });

            let response = self
                .emulator
                .serial
                .set_msg_from_master(byte, &mut self.emulator.state);
            if response != msg.prediction {
                return Some(MessageFromSlave {
                    correction: response,
                    cycle: master_cycle,
                });
            }
        }
        None
    }

    fn add_snapshot(&mut self) {
        if let Err(arraydeque::CapacityError { element }) = self
            .snapshots
            .push_back((self.emulator.clone(), self.mbc.clone_boxed()))
        {
            self.snapshots.pop_front();
            self.snapshots.push_back(element).unwrap();
        }
    }
}

fn console_log(text: &str) {
    console::log_1(&JsValue::from_str(text));
}

type Snapshot = (Emulator, EasyMbc);

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
            inner.drive_and_sample(
                left,
                right,
                sample_rate,
                on_new_frame,
                &mut self.serial_mode,
            );
        }
    }

    pub fn get_save(&self) -> Option<Save> {
        self.inner.as_ref().and_then(WebEmulatorInner::get_save)
    }

    pub fn set_a(&mut self, value: bool) {
        if let Some(inner) = &mut self.inner {
            inner.set_joypad(JoypadInput {
                a: value,
                ..*inner.emulator.get_joypad()
            });
        }
    }
    pub fn set_b(&mut self, value: bool) {
        if let Some(inner) = &mut self.inner {
            inner.set_joypad(JoypadInput {
                b: value,
                ..*inner.emulator.get_joypad()
            });
        }
    }
    pub fn set_start(&mut self, value: bool) {
        if let Some(inner) = &mut self.inner {
            inner.set_joypad(JoypadInput {
                start: value,
                ..*inner.emulator.get_joypad()
            });
        }
    }
    pub fn set_select(&mut self, value: bool) {
        if let Some(inner) = &mut self.inner {
            inner.set_joypad(JoypadInput {
                select: value,
                ..*inner.emulator.get_joypad()
            });
        }
    }
    pub fn set_left(&mut self, value: bool) {
        if let Some(inner) = &mut self.inner {
            inner.set_joypad(JoypadInput {
                left: value,
                ..*inner.emulator.get_joypad()
            });
        }
    }
    pub fn set_right(&mut self, value: bool) {
        if let Some(inner) = &mut self.inner {
            inner.set_joypad(JoypadInput {
                right: value,
                ..*inner.emulator.get_joypad()
            });
        }
    }
    pub fn set_down(&mut self, value: bool) {
        if let Some(inner) = &mut self.inner {
            inner.set_joypad(JoypadInput {
                down: value,
                ..*inner.emulator.get_joypad()
            });
        }
    }
    pub fn set_up(&mut self, value: bool) {
        if let Some(inner) = &mut self.inner {
            inner.set_joypad(JoypadInput {
                up: value,
                ..*inner.emulator.get_joypad()
            });
        }
    }

    pub fn set_serial_msg(&mut self, msg: &[u8]) -> Option<Box<[u8]>> {
        let msg = SerialMessage::deserialize(msg);
        if let Some(inner) = &mut self.inner {
            inner
                .set_serial_msg(msg.get(), &mut self.serial_mode)
                .map(|msg| {
                    console_log(&format!("DURE CORREKUSHOOON 0x{:02x}", msg.correction));
                    SerialMessage::FromSlave(msg).serialize()
                })
        } else if let ArchivedSerialMessage::FromMaster(msg) = msg.get()
            && msg.prediction != 0xff
        {
            Some(
                SerialMessage::FromSlave(MessageFromSlave {
                    correction: 0xff,
                    cycle: msg.first_message.1.to_native(),
                })
                .serialize(),
            )
        } else {
            None
        }
    }

    pub fn set_is_serial_connected(&mut self, on_serial: Option<js_sys::Function>) {
        if let Some(on_serial) = on_serial {
            self.serial_mode = SerialMode::SynchroSerial(SynchroSerial {
                on_serial,
                current_message: MessageFromMasterAcc {
                    messages: Default::default(),
                    session: false,
                },
                snapshots: Default::default(),
            })
        } else {
            self.serial_mode = SerialMode::Disconnected;
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

struct MessageFromMasterAcc {
    messages: Vec<(u8, Emulator, EasyMbc)>,
    session: bool,
}

// 1 second
const ROLLBACK_TRESHOLD: u64 = 4194304 / 4;
// 10 ms
const BATCH_PERIOD: u64 = 4194304 / 4 / 100;
const MAX_SNAPSHOT: usize = 20;
const ROLLBACK_SNAPSHOT_PERIOD: u64 = ROLLBACK_TRESHOLD / MAX_SNAPSHOT as u64;
const INPUTS_HISTORY_SIZE: usize = 50;

struct SynchroSerial {
    on_serial: js_sys::Function,
    current_message: MessageFromMasterAcc,
    snapshots: Vec<(Emulator, EasyMbc)>,
}

impl SynchroSerial {
    fn execute(&mut self, prediction: u8, cycles: u64) {
        let Some((_, first_snap, _)) = self.current_message.messages.first() else {
            return;
        };

        if cycles - first_snap.get_cycles() <= BATCH_PERIOD {
            return;
        }

        self.snapshots
            .retain(|(snap, _)| cycles - snap.get_cycles() < ROLLBACK_TRESHOLD);
        let mut messages = core::mem::take(&mut self.current_message.messages).into_iter();
        let (first_byte, first_snap, first_mbc) = messages.next().unwrap();
        let first_cycle = first_snap.get_cycles();

        self.snapshots.push((first_snap, first_mbc));

        let mut messages_to_send = Vec::new();
        for (byte, emulator, mbc) in messages {
            messages_to_send.push((byte, emulator.get_cycles()));
            self.snapshots.push((emulator, mbc));
        }

        let msg_to_send = MessageFromMaster {
            first_message: (first_byte, first_cycle),
            messages: messages_to_send,
            prediction,
            session: self.current_message.session,
        };

        if let Err(err) = self.on_serial.call1(
            &JsValue::null(),
            &SerialMessage::FromMaster(msg_to_send).serialize().into(),
        ) {
            console::error_1(&err);
        }
    }
}

#[derive(Default)]
enum SerialMode {
    #[default]
    Disconnected,
    SynchroSerial(SynchroSerial),
}

impl SerialMode {
    fn execute(&mut self, prediction: u8, cycles: u64) {
        match self {
            Self::Disconnected => {}
            Self::SynchroSerial(synchro) => synchro.execute(prediction, cycles),
        }
    }
}
