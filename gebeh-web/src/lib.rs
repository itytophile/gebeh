use std::rc::Rc;

use arraydeque::ArrayDeque;
use gebeh_core::{
    Emulator, HEIGHT, SYSTEM_CLOCK_FREQUENCY, WIDTH,
    apu::Mixer,
    state::{Interruptions, SerialControl, State},
};
use gebeh_front_helper::{CloneMbc, EasyMbc, get_mbc, get_noise, get_title_from_rom};
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
    serial_state: SerialState,
    synchro_cycles: Option<SynchroCycles>,
    snapshots: Box<Snapshots>,
    session: bool,
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
            serial_state: SerialState::Slave {
                state: ProutSlave {},
            },
            synchro_cycles: None,
            snapshots: Default::default(),
            session: false,
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
                let (mut emulator, mbc) =
                    core::mem::take(&mut synchro_serial.previous_batch_snapshots)
                        .into_iter()
                        .find(|(emulator, _)| emulator.get_cycles() == msg.cycle)
                        .expect("desync too big");
                *emulator.get_joypad_mut() = *self.emulator.get_joypad_mut();
                self.emulator = emulator;
                self.mbc = mbc;
                synchro_serial.current_message.session = !synchro_serial.current_message.session;
                synchro_serial.current_message.messages.clear();
                console_log(&format!(
                    "Correction from slave 0x{:02x} -> 0x{:02x}",
                    synchro_serial.current_message.prediction, msg.correction
                ));
                synchro_serial.current_message.prediction = msg.correction;
                None
            }
        }
    }

    fn execute_and_take_snapshot(&mut self, serial_mode: &mut SerialMode) {
        self.emulator.execute(self.mbc.as_mut());
        let cycles = self.emulator.get_cycles();
        serial_mode.execute(
            &mut self.serial_state,
            &mut self.emulator,
            self.mbc.as_mut(),
        );
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

            let msg_from_slave = self.advance_while_consuming_messages(msg, serial_mode);

            self.add_snapshot();

            return msg_from_slave.inspect(|_| self.session = !self.session);
        };

        let current_cycle = self.emulator.get_cycles();

        let before_cycle =
            synchro_cycles.slave + msg.first_message.1.to_native() - synchro_cycles.master;

        let snapshots = core::mem::take(&mut self.snapshots);

        if let Some((emulator, mbc)) = snapshots
            .into_iter()
            .rev()
            .find(|(emulator, _)| emulator.get_cycles() <= before_cycle)
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

        let msg_from_slave = self.advance_while_consuming_messages(msg, serial_mode);

        self.add_snapshot();

        if msg_from_slave.is_none() && current_cycle > self.emulator.get_cycles() {
            // catching up
            for _ in 0..(current_cycle - self.emulator.get_cycles()) {
                self.execute_and_take_snapshot(serial_mode)
            }
        }

        msg_from_slave.inspect(|_| self.session = !self.session)
    }

    fn set_msg_from_master(&mut self, byte: u8) -> u8 {
        let SerialState::Slave {
            state: ProutSlave { .. },
        } = self.serial_state
        else {
            return 0xff;
        };

        if self
            .emulator
            .state
            .sc
            .contains(SerialControl::TRANSFER_ENABLE)
        {
            let sb = self.emulator.state.sb;
            self.serial_state
                .accept_byte(byte, &mut self.emulator.state);
            sb
        } else {
            0xff
        }
    }

    fn advance_while_consuming_messages(
        &mut self,
        msg: &ArchivedMessageFromMaster,
        serial_mode: &mut SerialMode,
    ) -> Option<MessageFromSlave> {
        for (byte, master_cycle) in std::iter::once(&msg.first_message)
            .chain(msg.messages.iter())
            .map(|a| (a.0, a.1.to_native()))
        {
            for _ in 0..master_cycle - self.synchro_cycles.as_ref().unwrap().master {
                self.execute_and_take_snapshot(serial_mode)
            }

            self.synchro_cycles = Some(SynchroCycles {
                master: master_cycle,
                slave: self.emulator.get_cycles(),
            });

            let emulator_clone = self.emulator.clone();
            let response = self.set_msg_from_master(byte);
            if response != msg.prediction {
                self.emulator = emulator_clone;
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
            inner.emulator.get_joypad_mut().a = value;
        }
    }
    pub fn set_b(&mut self, value: bool) {
        if let Some(inner) = &mut self.inner {
            inner.emulator.get_joypad_mut().b = value;
        }
    }
    pub fn set_start(&mut self, value: bool) {
        if let Some(inner) = &mut self.inner {
            inner.emulator.get_joypad_mut().start = value;
        }
    }
    pub fn set_select(&mut self, value: bool) {
        if let Some(inner) = &mut self.inner {
            inner.emulator.get_joypad_mut().select = value;
        }
    }
    pub fn set_left(&mut self, value: bool) {
        if let Some(inner) = &mut self.inner {
            inner.emulator.get_joypad_mut().left = value;
        }
    }
    pub fn set_right(&mut self, value: bool) {
        if let Some(inner) = &mut self.inner {
            inner.emulator.get_joypad_mut().right = value;
        }
    }
    pub fn set_down(&mut self, value: bool) {
        if let Some(inner) = &mut self.inner {
            inner.emulator.get_joypad_mut().down = value;
        }
    }
    pub fn set_up(&mut self, value: bool) {
        if let Some(inner) = &mut self.inner {
            inner.emulator.get_joypad_mut().up = value;
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
                    prediction: 0xff,
                    messages: Default::default(),
                    session: false,
                },
                previous_batch_snapshots: Default::default(),
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
    prediction: u8,
    messages: Vec<(u8, Emulator, EasyMbc)>,
    session: bool,
}

#[derive(Clone, Copy)]
enum ProutMaster {
    Init,
    Exchanging(u16),
}

struct ProutSlave {}

enum SerialState {
    MasterNoTransfer,
    Master(ProutMaster),
    Slave { state: ProutSlave },
}

impl SerialState {
    fn needs_message(&self) -> bool {
        matches!(self, Self::Master(ProutMaster::Exchanging(_)))
    }
}

impl SerialState {
    fn refresh(&mut self, state: &State) {
        if !state.sc.contains(SerialControl::TRANSFER_ENABLE)
            && state.sc.contains(SerialControl::CLOCK_SELECT)
        {
            *self = Self::MasterNoTransfer;
            return;
        }

        if state.sc.contains(SerialControl::CLOCK_SELECT)
            && !std::matches!(self, SerialState::Master(_))
        {
            *self = Self::Master(ProutMaster::Init);
            return;
        }

        if !state.sc.contains(SerialControl::CLOCK_SELECT)
            && !std::matches!(self, SerialState::Slave { .. })
        {
            *self = Self::Slave {
                state: ProutSlave {},
            }
        }
    }

    fn get_serial_byte(&mut self, state: &State) -> Option<u8> {
        if let SerialState::Master(ProutMaster::Init) = self {
            *self = SerialState::Master(ProutMaster::Exchanging(0));
            return Some(state.sb);
        }
        None
    }

    fn accept_byte(&mut self, byte: u8, state: &mut State) {
        state.sc.remove(SerialControl::TRANSFER_ENABLE);
        state.interrupt_flag.insert(Interruptions::SERIAL);
        state.sb = byte;
        self.refresh(state);
    }

    fn set_msg_from_slave(&mut self, byte: u8, state: &mut State) {
        if std::matches!(self, Self::Master(_)) {
            self.accept_byte(byte, state);
        }
    }
}

// 1 second
const ROLLBACK_TRESHOLD: u64 = 4194304 / 4;
// 10 ms
const BATCH_PERIOD: u64 = 4194304 / 4 / 100;
const MAX_SNAPSHOT: usize = 20;
const ROLLBACK_SNAPSHOT_PERIOD: u64 = ROLLBACK_TRESHOLD / MAX_SNAPSHOT as u64;

struct SynchroSerial {
    on_serial: js_sys::Function,
    current_message: MessageFromMasterAcc,
    previous_batch_snapshots: Vec<(Emulator, EasyMbc)>,
}

impl SynchroSerial {
    fn execute(
        &mut self,
        serial_state: &mut SerialState,
        emulator: &mut Emulator,
        mbc: &dyn CloneMbc<'static>,
    ) {
        serial_state.refresh(&emulator.state);
        // doesn't happen at the same time as get_serial_byte
        if serial_state.needs_message() {
            serial_state.set_msg_from_slave(self.current_message.prediction, &mut emulator.state);
        }
        if let Some(byte) = serial_state.get_serial_byte(&emulator.state) {
            self.current_message
                .messages
                .push((byte, emulator.clone(), mbc.clone_boxed()));
        }
        if let Some((_, first_snap, _)) = self.current_message.messages.first()
            && emulator.get_cycles() - first_snap.get_cycles() > BATCH_PERIOD
        {
            self.previous_batch_snapshots
                .retain(|(snap, _)| emulator.get_cycles() - snap.get_cycles() < ROLLBACK_TRESHOLD);
            let mut messages = core::mem::take(&mut self.current_message.messages).into_iter();
            let (first_byte, first_snap, first_mbc) = messages.next().unwrap();
            let first_cycle = first_snap.get_cycles();

            self.previous_batch_snapshots.push((first_snap, first_mbc));

            let mut messages_to_send = Vec::new();
            for (byte, emulator, mbc) in messages {
                messages_to_send.push((byte, emulator.get_cycles()));
                self.previous_batch_snapshots.push((emulator, mbc));
            }

            let msg_to_send = MessageFromMaster {
                first_message: (first_byte, first_cycle),
                messages: messages_to_send,
                prediction: self.current_message.prediction,
                session: self.current_message.session,
            };

            console_log(&format!("Sending batch {msg_to_send:?}"));

            if let Err(err) = self.on_serial.call1(
                &JsValue::null(),
                &SerialMessage::FromMaster(msg_to_send).serialize().into(),
            ) {
                console::error_1(&err);
            }
        }
        if let SerialState::Master(ProutMaster::Exchanging(delay)) = serial_state {
            *delay = delay.saturating_add(1);
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
    fn execute(
        &mut self,
        serial_state: &mut SerialState,
        emulator: &mut Emulator,
        mbc: &dyn CloneMbc<'static>,
    ) {
        match self {
            Self::Disconnected => {
                serial_state.refresh(&emulator.state);
                serial_state.set_msg_from_slave(0xff, &mut emulator.state);
            }
            Self::SynchroSerial(synchro) => synchro.execute(serial_state, emulator, mbc),
        }
    }
}
