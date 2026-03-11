use std::{iter, rc::Rc};

use gebeh_core::{
    Emulator, HEIGHT, SYSTEM_CLOCK_FREQUENCY, WIDTH,
    apu::Mixer,
    state::{Interruptions, SerialControl, State},
};
use gebeh_front_helper::{EasyMbc, get_mbc, get_noise, get_title_from_rom};
use rkyv::{Archive, Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::{
    console,
    js_sys::{self},
};

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
    serial_state: SerialState,
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
                state: ProutSlave { snapshot: None },
            },
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
                if self.serial_state.is_blocking_execution() {
                    return;
                }
                self.emulator.execute(self.mbc.as_mut());
                let cycles = self.emulator.get_cycles();
                serial_mode.execute(&mut self.serial_state, &mut self.emulator.state, cycles);
                if self.serial_state.is_blocking_execution() {
                    return;
                }
                if let SerialState::Slave {
                    state:
                        ProutSlave {
                            snapshot: Some(snapshot),
                        },
                } = &mut self.serial_state
                    && snapshot.emulator.get_cycles() < cycles
                    && cycles - snapshot.emulator.get_cycles() < ROLLBACK_TRESHOLD
                    && (cycles - snapshot.emulator.get_cycles()).is_multiple_of(ROLLBACK_SNAPSHOT_PERIOD)
                {
                    console::log_1(&JsValue::from_str("snap"));
                    snapshot
                        .snapshots
                        .push((self.emulator.clone(), self.mbc.clone_boxed()));
                }
                if let Some(scanline) = self.emulator.get_ppu().get_scanline_if_ready() {
                    self.current_frame.as_chunks_mut::<40>().0
                        [usize::from(self.emulator.get_ppu().get_ly())] = *scanline.raw();

                    if self.emulator.get_ppu().get_ly() == HEIGHT - 1
                        && let Err(err) = on_new_frame.call1(
                            &JsValue::null(),
                            &js_sys::Uint8Array::new_from_slice(&self.current_frame),
                        )
                    {
                        console::error_1(&err);
                    }
                }
            }
            let sample = self.sample_index as f32 / sample_rate as f32;
            let mut sampler = self
                .mixer
                .mix(self.emulator.get_apu().get_sampler(), sample);
            *left = sampler.sample_left();
            *right = sampler.sample_right();
            // 2 minutes without popping (sample_index must not be huge to prevent precision errors)
            self.sample_index = self.sample_index.wrapping_add(1) % (sample_rate * 2 * 60);
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

    pub fn set_serial_msg(&mut self, msg: SerialMessage) -> Option<SerialMessage> {
        if let Some(cycles) = msg.master_cycles {
            Some(SerialMessage {
                byte: self.serial_state.set_msg_from_master(
                    msg.byte,
                    cycles,
                    &mut self.emulator,
                    &mut self.mbc,
                ),
                master_cycles: None,
            })
        } else {
            self.serial_state
                .set_msg_from_slave(msg.byte, &mut self.emulator.state);
            None
        }
    }
}

#[wasm_bindgen]
impl WebEmulator {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
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

    pub fn set_serial_msg(&mut self, msg: SerialMessage) -> Option<SerialMessage> {
        if let Some(inner) = &mut self.inner {
            inner.set_serial_msg(msg)
        } else if msg.master_cycles.is_some() {
            Some(SerialMessage {
                byte: 0xff,
                master_cycles: None,
            })
        } else {
            None
        }
    }

    pub fn set_is_serial_connected(&mut self, on_serial: Option<js_sys::Function>) {
        if let Some(on_serial) = on_serial {
            self.serial_mode = SerialMode::SynchroSerial(SynchroSerial { on_serial })
        } else {
            self.serial_mode = SerialMode::Disconnected;
        }
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

#[wasm_bindgen]
#[derive(Archive, Deserialize, Serialize)]
pub struct SerialMessage {
    master_cycles: Option<u64>,
    byte: u8,
}

#[wasm_bindgen]
impl SerialMessage {
    pub fn deserialize(buffer: &[u8]) -> Option<Self> {
        let archived = rkyv::access::<ArchivedSerialMessage, rkyv::rancor::Error>(buffer).ok()?;
        rkyv::deserialize::<_, rkyv::rancor::Error>(archived).ok()
    }

    pub fn serialize(&self) -> Box<[u8]> {
        rkyv::to_bytes::<rkyv::rancor::Error>(self)
            .unwrap()
            .into_boxed_slice()
    }
}

// système de rollback
// -> mode full synchro avant tout
// -> on échantillone pendant x temps
// suite à l'échantillonage on déduit différents modes
// -> Il ne se passe rien: full synchro car le master ne fait pas de transfert donc rapide
// -> le master fait du polling: on passe en mode prédiction
// -> le master envoie des données complexes: full synchro
//
// Comment détecter le polling ?
// -> le master et le slave renvoie la même chose tout le temps avec rythme "constant"
// Comment sortir du mode prédiction ?
// -> le master envoie quelque chose de différent
// -> le master active le transfert avec un rythme différent
// -> le slave envoie quelque chose de différent
// -> le slave active le transfert avec un rythme différent
// Comment synchroniser en cas de mauvaise prédiction ?
// -> Le slave et le master prennent des snapshot tous les x transferts
// -> le master et le slave s'accorde sur la snapshot valide la plus tard.
// Comment détecter un rythme différent ?
// -> en vrai ce n'est peut-être pas nécessaire

// https://gbdev.io/pandocs/Specifications.html
// https://gbdev.io/pandocs/Serial_Data_Transfer_(Link_Cable).html#internal-clock
// 4194304 / 4 (system clock) / 8192 (serial clock) * 8 (one bit per clock)
const EXCHANGE_DELAY: u16 = 1024;

#[derive(Clone, Copy)]
enum ProutMaster {
    Init,
    Exchanging(u16),
}

struct ProutSlave {
    snapshot: Option<Snapshot>,
}

struct Snapshot {
    master_cycles: u64,
    emulator: Box<Emulator>, // boxed because large
    mbc: EasyMbc,
    snapshots: Vec<(Emulator, EasyMbc)>,
}

enum SerialState {
    MasterNoTransfer,
    Master(ProutMaster),
    Slave { state: ProutSlave },
}

impl SerialState {
    fn is_blocking_execution(&self) -> bool {
        if let Self::Master(ProutMaster::Exchanging(count)) = self
            && *count >= EXCHANGE_DELAY
        {
            true
        } else {
            false
        }
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
                state: ProutSlave { snapshot: None },
            }
        }
    }

    fn get_msg(&mut self, state: &State, clock: u64) -> Option<SerialMessage> {
        if let SerialState::Master(ProutMaster::Init) = self {
            *self = SerialState::Master(ProutMaster::Exchanging(0));
            return Some(SerialMessage {
                master_cycles: Some(clock),
                byte: state.sb,
            });
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

    // When receiving a message from a master, always send a response even if we are a master
    #[must_use]
    fn set_msg_from_master(
        &mut self,
        byte: u8,
        master_cycles: u64,
        emulator: &mut Emulator,
        mbc: &mut EasyMbc,
    ) -> u8 {
        let Self::Slave {
            state: ProutSlave { snapshot },
        } = self
        else {
            return 0xff;
        };

        let mut response = 0xff;

        if let Some(snapshot) = snapshot.take()
            && master_cycles >= snapshot.master_cycles
            && master_cycles - snapshot.master_cycles < ROLLBACK_TRESHOLD
        {
            let (index, (snap_emulator, snap_mbc)) = iter::once((*snapshot.emulator, snapshot.mbc))
                .chain(snapshot.snapshots)
                .enumerate()
                .filter(|(index, _)| {
                    u64::try_from(*index).unwrap() * ROLLBACK_SNAPSHOT_PERIOD
                        < (master_cycles - snapshot.master_cycles)
                })
                .last()
                .unwrap();

            *emulator = snap_emulator;
            *mbc = snap_mbc;
            let offset = u64::try_from(index).unwrap() * ROLLBACK_SNAPSHOT_PERIOD;
            console::log_1(&JsValue::from_str(&format!(
                "Rollback {} cycles ({} ms) with offset {}",
                master_cycles - snapshot.master_cycles - offset,
                (master_cycles - snapshot.master_cycles - offset) * 1000 * 4 / 4194304,
                offset
            )));
            for _ in 0..(master_cycles - snapshot.master_cycles - offset) {
                emulator.execute(mbc.as_mut());
            }
        }

        if emulator.state.sc.contains(SerialControl::TRANSFER_ENABLE) {
            response = emulator.state.sb;
            self.accept_byte(byte, &mut emulator.state);
        }

        if let Self::Slave {
            state: ProutSlave { snapshot },
        } = self
        {
            *snapshot = Some(Snapshot {
                master_cycles,
                emulator: Box::new(emulator.clone()),
                mbc: mbc.clone_boxed(),
                snapshots: Default::default(),
            });
        }

        response
    }
}

// 200 ms
const ROLLBACK_TRESHOLD: u64 = 4194304 / 4 / 5;
const ROLLBACK_SNAPSHOT_PERIOD: u64 = ROLLBACK_TRESHOLD / 20;

struct SynchroSerial {
    on_serial: js_sys::Function,
}

impl SynchroSerial {
    fn execute(&mut self, serial_state: &mut SerialState, state: &mut State, clock: u64) {
        serial_state.refresh(state);
        if let Some(msg) = serial_state.get_msg(state, clock)
            && let Err(err) = self.on_serial.call1(&JsValue::null(), &msg.into())
        {
            console::error_1(&err);
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
    fn execute(&mut self, serial_state: &mut SerialState, state: &mut State, clock: u64) {
        match self {
            Self::Disconnected => {
                serial_state.refresh(state);
                serial_state.set_msg_from_slave(0xff, state);
            }
            Self::SynchroSerial(synchro) => synchro.execute(serial_state, state, clock),
        }
    }
}
