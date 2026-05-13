mod bus;
mod execute_instruction;
pub mod instructions;
pub mod speed_switch;

use crate::{
    Model, Peripherals, PeripheralsRef, addresses::*, external_bus::external_bus_read,
    interrupts::Interrupts, mbc::Mbc,
};
use arrayvec::ArrayVec;
use instructions::{
    AfterReadInstruction, Instruction, InstructionsAndSetPc, NoReadInstruction, OpAfterRead,
    Prefetch, ReadAddress, Register8Bit, Register16Bit, SetPc, get_instructions, vec,
};

// https://github.com/joamag/boytacean/blob/bfb56ee4073f47f9bc1401a1d5206bfafb4ec901/src/data.rs#L177
pub const CGB_BOYTACEAN: [u8; 2304] = [
    49, 254, 255, 205, 166, 5, 38, 254, 14, 160, 34, 13, 32, 252, 14, 16, 33, 48, 255, 34, 47, 13,
    32, 251, 224, 193, 224, 128, 62, 128, 224, 38, 224, 17, 62, 243, 224, 18, 224, 37, 62, 119,
    224, 36, 62, 252, 224, 71, 17, 4, 1, 33, 16, 128, 26, 71, 205, 113, 5, 19, 123, 254, 52, 32,
    245, 205, 1, 6, 62, 1, 224, 79, 205, 166, 5, 205, 211, 5, 6, 3, 33, 194, 152, 22, 3, 62, 8, 14,
    16, 245, 62, 1, 224, 79, 54, 8, 175, 224, 79, 241, 34, 130, 13, 32, 240, 214, 47, 213, 17, 16,
    0, 25, 209, 5, 32, 227, 21, 40, 10, 21, 62, 56, 46, 167, 1, 7, 1, 24, 216, 17, 97, 5, 14, 8,
    33, 129, 255, 175, 47, 34, 34, 26, 28, 246, 32, 71, 26, 29, 246, 132, 31, 203, 24, 112, 44, 34,
    175, 34, 34, 26, 28, 34, 26, 28, 34, 175, 13, 32, 225, 205, 109, 7, 62, 145, 224, 64, 205, 14,
    6, 62, 48, 224, 194, 6, 4, 205, 149, 5, 62, 131, 205, 159, 5, 6, 5, 205, 149, 5, 62, 193, 205,
    159, 5, 205, 148, 7, 205, 137, 5, 33, 194, 255, 53, 32, 244, 205, 55, 6, 24, 34, 208, 0, 152,
    160, 18, 208, 0, 128, 0, 64, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 224, 80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 136, 22, 54, 209, 219, 242, 60, 140, 146, 61, 92, 88, 201, 62, 112, 29, 89,
    105, 25, 53, 168, 20, 170, 117, 149, 153, 52, 111, 21, 255, 151, 75, 144, 23, 16, 57, 247, 246,
    162, 73, 78, 67, 104, 224, 139, 240, 206, 12, 41, 232, 183, 134, 154, 82, 1, 157, 113, 156,
    189, 93, 109, 103, 63, 107, 179, 70, 40, 165, 198, 211, 39, 97, 24, 102, 106, 191, 13, 244,
    179, 70, 40, 165, 198, 211, 39, 97, 24, 102, 106, 191, 13, 244, 179, 0, 4, 5, 35, 34, 3, 31,
    15, 10, 5, 19, 36, 135, 37, 30, 44, 21, 32, 31, 20, 5, 33, 13, 14, 5, 29, 5, 18, 9, 3, 2, 26,
    25, 25, 41, 42, 26, 45, 42, 45, 36, 38, 154, 42, 30, 41, 34, 34, 5, 42, 6, 5, 33, 25, 42, 42,
    40, 2, 16, 25, 42, 42, 5, 0, 39, 36, 22, 25, 6, 32, 12, 36, 11, 39, 18, 39, 24, 31, 50, 17, 46,
    6, 27, 0, 47, 41, 41, 0, 0, 19, 34, 23, 18, 29, 66, 69, 70, 65, 65, 82, 66, 69, 75, 69, 75, 32,
    82, 45, 85, 82, 65, 82, 32, 73, 78, 65, 73, 76, 73, 67, 69, 32, 82, 32, 32, 232, 144, 144, 144,
    160, 160, 160, 192, 192, 192, 72, 72, 72, 0, 0, 0, 216, 216, 216, 40, 40, 40, 96, 96, 96, 208,
    208, 208, 128, 64, 64, 32, 224, 224, 32, 16, 16, 24, 32, 32, 32, 232, 232, 224, 32, 224, 16,
    136, 16, 128, 128, 64, 32, 32, 56, 32, 32, 144, 32, 32, 160, 152, 152, 72, 30, 30, 88, 136,
    136, 16, 32, 32, 16, 32, 32, 24, 224, 224, 0, 24, 24, 0, 0, 0, 8, 144, 176, 144, 160, 176, 160,
    192, 176, 192, 128, 176, 64, 136, 32, 104, 222, 0, 112, 222, 32, 120, 152, 182, 72, 128, 224,
    80, 32, 184, 224, 136, 176, 16, 32, 0, 16, 32, 224, 24, 224, 24, 0, 24, 224, 32, 168, 224, 32,
    24, 224, 0, 200, 24, 224, 0, 224, 64, 32, 24, 224, 224, 24, 48, 32, 224, 232, 240, 240, 240,
    248, 248, 248, 224, 32, 8, 0, 0, 16, 255, 127, 191, 50, 208, 0, 0, 0, 159, 99, 121, 66, 176,
    21, 203, 4, 255, 127, 49, 110, 74, 69, 0, 0, 255, 127, 239, 27, 0, 2, 0, 0, 255, 127, 31, 66,
    242, 28, 0, 0, 255, 127, 148, 82, 74, 41, 0, 0, 255, 127, 255, 3, 47, 1, 0, 0, 255, 127, 239,
    3, 214, 1, 0, 0, 255, 127, 181, 66, 200, 61, 0, 0, 116, 126, 255, 3, 128, 1, 0, 0, 255, 103,
    172, 119, 19, 26, 107, 45, 214, 126, 255, 75, 117, 33, 0, 0, 255, 83, 95, 74, 82, 126, 0, 0,
    255, 79, 210, 126, 76, 58, 224, 28, 237, 3, 255, 127, 95, 37, 0, 0, 106, 3, 31, 2, 255, 3, 255,
    127, 255, 127, 223, 1, 18, 1, 0, 0, 31, 35, 95, 3, 242, 0, 9, 0, 255, 127, 234, 3, 31, 1, 0, 0,
    159, 41, 26, 0, 12, 0, 0, 0, 255, 127, 127, 2, 31, 0, 0, 0, 255, 127, 224, 3, 6, 2, 32, 1, 255,
    127, 235, 126, 31, 0, 0, 124, 255, 127, 255, 63, 0, 126, 31, 0, 255, 127, 255, 3, 31, 0, 0, 0,
    255, 3, 31, 0, 12, 0, 0, 0, 255, 127, 63, 3, 147, 1, 0, 0, 0, 0, 0, 66, 127, 3, 255, 127, 255,
    127, 140, 126, 0, 124, 0, 0, 255, 127, 239, 27, 128, 97, 0, 0, 255, 127, 234, 127, 95, 125, 0,
    0, 120, 71, 144, 50, 135, 29, 97, 8, 3, 144, 15, 24, 0, 120, 129, 9, 18, 21, 84, 147, 153, 156,
    159, 162, 60, 66, 185, 165, 185, 165, 66, 60, 2, 0, 36, 3, 12, 0, 6, 255, 8, 199, 8, 255, 8,
    199, 6, 255, 16, 0, 6, 128, 4, 131, 2, 135, 4, 7, 6, 143, 6, 135, 4, 3, 22, 0, 2, 240, 2, 248,
    2, 252, 4, 60, 6, 62, 4, 60, 2, 252, 2, 248, 2, 240, 22, 0, 4, 120, 2, 124, 4, 60, 2, 63, 4,
    31, 4, 15, 8, 7, 2, 31, 4, 63, 2, 30, 12, 0, 4, 121, 2, 249, 6, 240, 4, 224, 4, 192, 10, 128,
    8, 0, 10, 60, 6, 255, 12, 60, 2, 63, 6, 31, 22, 0, 6, 135, 4, 0, 6, 15, 2, 14, 2, 142, 6, 143,
    22, 0, 6, 248, 4, 120, 6, 249, 4, 120, 6, 248, 22, 0, 2, 63, 2, 127, 2, 255, 6, 243, 2, 224, 6,
    243, 4, 127, 2, 63, 22, 0, 4, 129, 6, 195, 2, 199, 2, 7, 2, 199, 6, 195, 4, 129, 22, 0, 4, 254,
    2, 255, 4, 207, 6, 255, 4, 128, 6, 254, 22, 0, 6, 31, 4, 0, 6, 31, 4, 28, 6, 31, 22, 0, 26,
    227, 22, 0, 6, 255, 20, 195, 22, 0, 26, 128, 10, 0, 0, 255, 127, 79, 119, 199, 34, 159, 3, 125,
    1, 29, 36, 56, 109, 0, 85, 205, 116, 5, 62, 4, 14, 0, 203, 32, 245, 203, 17, 241, 203, 17, 61,
    32, 245, 121, 34, 35, 34, 35, 201, 229, 33, 15, 255, 203, 134, 203, 70, 40, 252, 225, 201, 205,
    148, 7, 205, 137, 5, 5, 32, 247, 201, 224, 19, 62, 135, 224, 20, 201, 33, 0, 128, 175, 34, 203,
    108, 40, 250, 201, 205, 179, 5, 26, 161, 71, 28, 28, 26, 29, 29, 161, 203, 55, 176, 203, 65,
    40, 2, 203, 55, 35, 34, 203, 49, 201, 205, 205, 5, 205, 176, 5, 28, 123, 201, 33, 150, 4, 17,
    127, 128, 70, 4, 5, 40, 10, 35, 126, 35, 19, 18, 5, 32, 251, 24, 241, 98, 46, 128, 17, 4, 1,
    14, 240, 205, 202, 5, 198, 22, 95, 205, 202, 5, 214, 22, 95, 254, 28, 32, 238, 35, 17, 142, 4,
    14, 8, 26, 19, 34, 35, 13, 32, 249, 201, 62, 1, 224, 79, 22, 26, 6, 2, 205, 149, 5, 33, 192,
    152, 14, 3, 126, 254, 15, 40, 5, 52, 230, 7, 40, 3, 35, 24, 243, 125, 246, 31, 111, 35, 13, 32,
    235, 21, 32, 222, 201, 6, 32, 14, 32, 33, 129, 255, 197, 42, 95, 58, 87, 1, 33, 4, 123, 230,
    31, 254, 31, 32, 1, 13, 123, 254, 224, 56, 9, 122, 230, 3, 254, 3, 32, 2, 203, 169, 122, 230,
    124, 254, 124, 32, 2, 203, 144, 123, 129, 34, 122, 136, 34, 193, 13, 32, 207, 205, 137, 5, 205,
    109, 7, 205, 137, 5, 5, 32, 190, 62, 2, 224, 112, 33, 0, 208, 205, 169, 5, 60, 205, 129, 7,
    205, 134, 7, 205, 129, 7, 175, 224, 112, 47, 224, 0, 87, 89, 46, 13, 250, 67, 1, 203, 127, 204,
    198, 6, 203, 127, 224, 76, 240, 128, 71, 40, 5, 240, 193, 167, 32, 6, 175, 79, 62, 17, 97, 201,
    205, 198, 6, 224, 76, 62, 1, 201, 33, 125, 4, 79, 6, 0, 9, 126, 201, 62, 1, 224, 108, 205, 241,
    6, 203, 127, 196, 52, 8, 203, 191, 71, 128, 128, 71, 240, 193, 167, 40, 5, 205, 189, 6, 24, 1,
    120, 205, 137, 5, 205, 72, 7, 62, 4, 17, 8, 0, 46, 124, 201, 33, 75, 1, 126, 254, 51, 40, 5,
    61, 32, 66, 24, 12, 46, 68, 42, 254, 48, 32, 57, 126, 254, 49, 32, 52, 46, 52, 14, 16, 175,
    134, 44, 13, 32, 251, 224, 128, 71, 33, 0, 2, 125, 214, 94, 200, 42, 184, 32, 248, 125, 214,
    66, 56, 14, 229, 125, 198, 122, 111, 126, 225, 79, 250, 55, 1, 185, 32, 229, 125, 198, 93, 111,
    120, 224, 128, 126, 201, 175, 201, 33, 217, 2, 6, 0, 79, 9, 201, 205, 64, 7, 30, 0, 42, 229,
    33, 126, 3, 79, 9, 22, 8, 14, 106, 205, 118, 7, 225, 203, 91, 32, 4, 30, 8, 24, 233, 78, 33,
    126, 3, 9, 22, 8, 24, 5, 33, 129, 255, 22, 64, 30, 0, 14, 104, 62, 128, 179, 226, 12, 42, 226,
    21, 32, 251, 201, 224, 79, 33, 220, 0, 205, 137, 5, 14, 81, 6, 5, 42, 226, 12, 5, 32, 250, 201,
    62, 32, 224, 0, 240, 0, 47, 230, 15, 200, 46, 0, 44, 31, 48, 252, 62, 16, 224, 0, 240, 0, 47,
    23, 23, 230, 12, 133, 111, 240, 193, 189, 200, 125, 224, 193, 197, 213, 205, 189, 6, 205, 64,
    7, 44, 44, 78, 33, 127, 3, 9, 58, 254, 127, 32, 2, 35, 35, 245, 42, 229, 33, 129, 255, 205, 42,
    8, 46, 131, 205, 42, 8, 225, 224, 135, 42, 229, 33, 130, 255, 205, 42, 8, 46, 132, 205, 42, 8,
    225, 224, 136, 241, 40, 2, 35, 35, 240, 187, 230, 222, 71, 42, 230, 222, 128, 71, 250, 188,
    255, 203, 151, 78, 203, 145, 137, 31, 234, 188, 255, 120, 31, 234, 187, 255, 45, 42, 224, 191,
    42, 224, 192, 42, 224, 133, 42, 224, 134, 205, 137, 5, 205, 109, 7, 62, 48, 224, 194, 209, 193,
    201, 17, 8, 0, 75, 119, 25, 13, 32, 251, 201, 245, 205, 137, 5, 62, 25, 234, 16, 153, 33, 47,
    153, 14, 12, 61, 40, 8, 50, 13, 32, 249, 46, 15, 24, 245, 241, 201, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
];

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
pub struct Cpu<M: Model> {
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
    pub boot_rom: &'static [u8],
    pub speed_switch: M::SpeedSwitch,
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

impl<M: Model> Cpu<M> {
    pub fn new(boot_rom: &'static [u8]) -> Self {
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
            boot_rom,
            speed_switch: Default::default(),
        }
    }
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

    fn read(
        &self,
        index: u16,
        peripherals: PeripheralsRef<impl Mbc + ?Sized, M>,
        cycles: u64,
    ) -> u8 {
        // https://gbdev.io/pandocs/Power_Up_Sequence.html#size
        match index {
            ..0x100 | 0x200..0x900 if !self.boot_rom_mapping_control => {
                self.boot_rom[usize::from(index)]
            }
            ..OAM => external_bus_read(
                index,
                peripherals.mbc,
                peripherals.ppu.get_vram_if_available(),
                peripherals.wram,
            ),
            index => self.internal_bus_read(index, peripherals, cycles),
        }
    }

    pub fn execute(
        &mut self,
        mut peripherals: Peripherals<impl Mbc + ?Sized, M>,
        cycle_count: u64,
    ) {
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
