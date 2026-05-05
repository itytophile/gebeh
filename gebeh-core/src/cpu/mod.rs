mod bus;
mod execute_instruction;
pub mod instructions;

use crate::{
    Peripherals, PeripheralsRef, addresses::*, external_bus::external_bus_read,
    interrupts::Interrupts, mbc::Mbc,
};
use arrayvec::ArrayVec;
use instructions::{
    AfterReadInstruction, Instruction, InstructionsAndSetPc, NoReadInstruction, OpAfterRead,
    Prefetch, ReadAddress, Register8Bit, Register16Bit, SetPc, get_instructions, vec,
};

// from https://github.com/Ashiepaws/Bootix
pub const BOOTIX_BOOT_ROM: [u8; 256] = [
    49, 254, 255, 33, 255, 159, 175, 50, 203, 124, 32, 250, 14, 17, 33, 38, 255, 62, 128, 50, 226,
    12, 62, 243, 50, 226, 12, 62, 119, 50, 226, 17, 4, 1, 33, 16, 128, 26, 205, 184, 0, 26, 203,
    55, 205, 184, 0, 19, 123, 254, 52, 32, 240, 17, 204, 0, 6, 8, 26, 19, 34, 35, 5, 32, 249, 33,
    4, 153, 1, 12, 1, 205, 177, 0, 62, 25, 119, 33, 36, 153, 14, 12, 205, 177, 0, 62, 145, 224, 64,
    6, 16, 17, 212, 0, 120, 224, 67, 5, 123, 254, 216, 40, 4, 26, 224, 71, 19, 14, 28, 205, 167, 0,
    175, 144, 224, 67, 5, 14, 28, 205, 167, 0, 175, 176, 32, 224, 224, 67, 62, 131, 205, 159, 0,
    14, 39, 205, 167, 0, 62, 193, 205, 159, 0, 17, 138, 1, 240, 68, 254, 144, 32, 250, 27, 122,
    179, 32, 245, 24, 73, 14, 19, 226, 12, 62, 135, 226, 201, 240, 68, 254, 144, 32, 250, 13, 32,
    247, 201, 120, 34, 4, 13, 32, 250, 201, 71, 14, 4, 175, 197, 203, 16, 23, 193, 203, 16, 23, 13,
    32, 245, 34, 35, 34, 35, 201, 60, 66, 185, 165, 185, 165, 66, 60, 0, 84, 168, 252, 66, 79, 79,
    84, 73, 88, 46, 68, 77, 71, 32, 118, 49, 46, 50, 0, 62, 255, 198, 1, 11, 30, 216, 33, 77, 1, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 62, 1, 224, 80,
];

#[derive(Clone)]
pub struct Cpu {
    pub sp: u16,
    pub lsb: u8,
    pub msb: u8,
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub f: Flags,
    pub is_cb_mode: bool,
    pub pc: u16,
    pub instruction_register: (ArrayVec<Instruction, 5>, Prefetch),
    pub ime: bool,
    old_ime: bool,
    pub is_halted: bool,
    pub stop_mode: bool,
    // test purposes
    pub current_opcode: u8,
    pub is_dispatching_interrupt: bool,
    pub interrupt_enable: Interrupts,
    pub hram: [u8; 0x7f],
    pub boot_rom_mapping_control: bool,
    pub boot_rom: &'static [u8; 256],
}

impl Default for Cpu {
    fn default() -> Self {
        Self {
            sp: Default::default(),
            lsb: Default::default(),
            msb: Default::default(),
            a: Default::default(),
            b: Default::default(),
            c: Default::default(),
            d: Default::default(),
            e: Default::default(),
            h: Default::default(),
            l: Default::default(),
            f: Default::default(),
            is_cb_mode: Default::default(),
            pc: Default::default(),
            // yes the cpu can fetch opcodes in parallel of the execution but for the first boost we must
            // feed a nop or the cpu will fetch + execute the fist opcode in the same cycle
            instruction_register: (vec([NoReadInstruction::Nop.into()]), Default::default()),
            ime: false,
            old_ime: false,
            is_halted: Default::default(),
            stop_mode: Default::default(),
            current_opcode: 0,
            is_dispatching_interrupt: false,
            interrupt_enable: Interrupts::empty(),
            hram: [0; 0x7f],
            boot_rom_mapping_control: false,
            boot_rom: &BOOTIX_BOOT_ROM,
        }
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Default, Copy, PartialEq, Eq)]
    pub struct Flags: u8 {
        const Z = 1 << 7;
        const N = 1 << 6;
        const H = 1 << 5;
        const C = 1 << 4;
    }
}

// Comment ça se passe avec mooneye
// le cpu drive l'ensemble
// pour une lecture d'un registre, il fait d'abord un cycle chez les périphériques, et ensuite il lit la valeur.
// Donc quand le cycle d'un périphérique donne une interruption, cela n'affecte pas
// le cpu dans le cycle actuel (puisqu'il est en train de faire l'action de lecture).
// Donc il faut traiter l'interruption dans le prochain cycle.
// Pour l'instant, il semble que les écritures/lectures du CPU sont toujours traités à la fin d'un cycle.
// Par exemple, il écrase les modif du timer pendant le cycle courant, et il a conscience des changements immédiats du ppu

impl Cpu {
    fn get_8bit_register(&self, register: Register8Bit) -> u8 {
        match register {
            Register8Bit::A => self.a,
            Register8Bit::B => self.b,
            Register8Bit::C => self.c,
            Register8Bit::D => self.d,
            Register8Bit::E => self.e,
            Register8Bit::H => self.h,
            Register8Bit::L => self.l,
            Register8Bit::F => self.f.bits(),
            Register8Bit::MsbSp => self.sp.to_be_bytes()[0],
            Register8Bit::LsbSp => self.sp.to_be_bytes()[1],
            Register8Bit::W => self.msb,
            Register8Bit::Z => self.lsb,
        }
    }

    fn set_8bit_register(&mut self, register: Register8Bit, value: u8) {
        match register {
            Register8Bit::A => self.a = value,
            Register8Bit::B => self.b = value,
            Register8Bit::C => self.c = value,
            Register8Bit::D => self.d = value,
            Register8Bit::E => self.e = value,
            Register8Bit::H => self.h = value,
            Register8Bit::L => self.l = value,
            Register8Bit::F => self.f = Flags::from_bits_truncate(value),
            Register8Bit::W => self.msb = value,
            Register8Bit::Z => self.lsb = value,
            Register8Bit::MsbSp | Register8Bit::LsbSp => unreachable!(),
        }
    }

    fn get_16bit_register(&self, register: Register16Bit) -> u16 {
        match register {
            Register16Bit::AF => u16::from(self.a) << 8 | u16::from(self.f.bits()),
            Register16Bit::BC => u16::from(self.b) << 8 | u16::from(self.c),
            Register16Bit::DE => u16::from(self.d) << 8 | u16::from(self.e),
            Register16Bit::HL => u16::from(self.h) << 8 | u16::from(self.l),
            Register16Bit::WZ => u16::from_be_bytes([self.msb, self.lsb]),
            Register16Bit::SP => self.sp,
            Register16Bit::PC => self.pc,
        }
    }

    fn set_16bit_register(&mut self, register: Register16Bit, value: u16) {
        match register {
            Register16Bit::SP => {
                self.sp = value;
                return;
            }
            Register16Bit::PC => {
                self.pc = value;
                return;
            }
            _ => {}
        }
        let [msb, lsb] = value.to_be_bytes();
        self.set_8bit_register(register.get_msb(), msb);
        self.set_8bit_register(register.get_lsb(), lsb);
    }
}

impl Cpu {
    fn read<M: Mbc + ?Sized>(&self, index: u16, peripherals: PeripheralsRef<M>, cycles: u64) -> u8 {
        match index {
            ..0x100 if !self.boot_rom_mapping_control => self.boot_rom[usize::from(index)],
            ..OAM => external_bus_read(
                index,
                peripherals.mbc,
                peripherals.ppu.get_vram_reader(),
                peripherals.wram,
            ),
            index => self.internal_bus_read(index, peripherals, cycles),
        }
    }

    pub fn execute<M: Mbc + ?Sized>(&mut self, mut peripherals: Peripherals<M>, cycle_count: u64) {
        let interrupts_to_execute =
            Interrupts::from_bits_truncate(self.interrupt_enable.bits()) & *peripherals.interrupts;
        // Peripherals interrupts are not handled the same cycle they are triggered.
        // However, the new value can be read or written over the same cycle.

        // https://gist.github.com/SonoSooS/c0055300670d678b5ae8433e20bea595#nop-and-stop
        if self.stop_mode {
            // self.stop_mode = false;
            // // quand on va sortir du stop mode on va exécuter un nop
            // // et fetch le prochain opcode en parallèle
            // self.instruction_register = (vec([NoReadInstruction::Nop.into()]), Default::default());
            todo!("stop")
        }

        // https://gbdev.io/pandocs/halt.html#halt
        if self.is_halted {
            if interrupts_to_execute.is_empty() {
                peripherals
                    .ppu
                    .execute_dma(peripherals.mbc, peripherals.wram, cycle_count);
                return;
            }
            self.is_halted = false;
            self.instruction_register = Default::default();
        }

        // petite douille. On profite que le CPU soit exécuté de manière cyclique pour changer l'ordre des étapes.
        // selon ma compréhension, OAM int est lancé un 0.5 t-cycle avant le début d'un nouveau cycle
        // peut-être que cela suffit à trigger le is_dispatching_interrupt du m-cycle d'avant (j'en sais rien, je comprends pas
        // ce que j'écris)
        if self.instruction_register.0.is_empty() {
            self.is_dispatching_interrupt = self.old_ime
                && self.instruction_register.1.check_interrupts
                && !interrupts_to_execute.is_empty();
            (self.pc, self.current_opcode) = match self.instruction_register.1.set_pc {
                SetPc::WithIncrement(register) => {
                    let address = self.get_16bit_register(register);
                    let opcode = self.read(address, peripherals.get_ref(), cycle_count);

                    (address.wrapping_add(1), opcode)
                }
                SetPc::NoIncrement => (
                    self.pc,
                    self.read(self.pc, peripherals.get_ref(), cycle_count),
                ),
            };
        }

        peripherals
            .ppu
            .execute_dma(peripherals.mbc, peripherals.wram, cycle_count);

        let inst = if let Some(inst) = self.instruction_register.0.pop() {
            inst
        } else if self.is_dispatching_interrupt {
            self.ime = false;
            // no need to set is_dispatching_interrupt to false
            use NoReadInstruction::*;
            self.instruction_register.0 = vec([
                Nop.into(),
                FinalStepInterruptDispatch.into(),
                WriteMsbOfRegisterWhereSpPointsAndDecSp(Register16Bit::PC).into(),
                DecStackPointer.into(),
            ]);
            self.instruction_register.1 = Default::default();
            DecPc.into()
        } else {
            let InstructionsAndSetPc((head, tail), set_pc) =
                get_instructions(self.current_opcode, self.is_cb_mode);
            self.is_cb_mode = false;
            self.instruction_register.0 = tail;
            self.instruction_register.1 = set_pc;
            head
        };

        // todo revoir la logique de lecture
        let inst = match inst {
            Instruction::NoRead(no_read) => AfterReadInstruction::NoRead(no_read),
            Instruction::Read(ReadAddress::Accumulator, inst) => AfterReadInstruction::Read(
                self.read(
                    0xff00 | u16::from(self.lsb),
                    peripherals.get_ref(),
                    cycle_count,
                ),
                inst,
            ),
            Instruction::Read(ReadAddress::Accumulator8Bit(register), inst) => {
                AfterReadInstruction::Read(
                    self.read(
                        0xff00 | u16::from(self.get_8bit_register(register)),
                        peripherals.get_ref(),
                        cycle_count,
                    ),
                    inst,
                )
            }
            Instruction::Read(ReadAddress::Register { register, op }, inst) => {
                let register_value = self.get_16bit_register(register);
                match op {
                    OpAfterRead::None => {}
                    OpAfterRead::Inc => {
                        self.set_16bit_register(register, register_value.wrapping_add(1))
                    }
                    OpAfterRead::Dec => {
                        self.set_16bit_register(register, register_value.wrapping_sub(1))
                    }
                }
                AfterReadInstruction::Read(
                    self.read(register_value, peripherals.get_ref(), cycle_count),
                    inst,
                )
            }
        };

        // EI must not take effect the same cycle so we copy it before executing instructions
        self.old_ime = self.ime;

        self.execute_instruction(inst, interrupts_to_execute, cycle_count, &mut peripherals);
    }
}
