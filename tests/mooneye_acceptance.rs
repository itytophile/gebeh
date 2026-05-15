use gebeh::InstantRtc;
use gebeh_core::{Cgb, Dmg, Emulator, EmulatorExt, Model};
use gebeh_front_helper::get_mbc;

fn test_mooneye<M: Model>(path: &str) {
    let rom = std::fs::read(format!(
        "./downloads/mts-20240926-1737-443f6e1/acceptance/{path}"
    ))
    .unwrap();
    let rom = rom.as_slice();
    let mut emulator = Emulator::<M>::default();
    let (_, mut mbc) = get_mbc(rom, InstantRtc::default()).unwrap();

    // https://github.com/Gekkio/mooneye-test-suite/tree/main?tab=readme-ov-file#passfail-reporting
    while emulator.get_cpu().current_opcode != 0x40 {
        emulator.execute(mbc.as_mut());
    }

    let cpu = emulator.get_cpu();

    assert_eq!(3, cpu.b);
    assert_eq!(5, cpu.c);
    assert_eq!(8, cpu.d);
    assert_eq!(13, cpu.e);
    assert_eq!(21, cpu.h);
    assert_eq!(34, cpu.l);
}

#[test]
fn add_sp_e_timing_dmg() {
    test_mooneye::<Dmg>("add_sp_e_timing.gb")
}

#[test]
fn mem_oam_dmg() {
    test_mooneye::<Dmg>("bits/mem_oam.gb")
}

#[test]
fn reg_f_dmg() {
    test_mooneye::<Dmg>("bits/reg_f.gb")
}

#[test]
fn unused_hwio_gs_dmg() {
    test_mooneye::<Dmg>("bits/unused_hwio-GS.gb")
}

#[test]
fn call_cc_timing_dmg() {
    test_mooneye::<Dmg>("call_cc_timing.gb");
}

#[test]
fn call_cc_timing2_dmg() {
    test_mooneye::<Dmg>("call_cc_timing2.gb");
}

#[test]
fn call_timing_dmg() {
    test_mooneye::<Dmg>("call_timing.gb");
}

#[test]
fn call_timing2_dmg() {
    test_mooneye::<Dmg>("call_timing2.gb");
}

#[test]
fn di_timing_gs_dmg() {
    test_mooneye::<Dmg>("di_timing-GS.gb");
}

#[test]
fn div_timing_dmg() {
    test_mooneye::<Dmg>("div_timing.gb");
}

#[test]
fn ei_sequence_dmg() {
    test_mooneye::<Dmg>("ei_sequence.gb");
}

#[test]
fn ei_timing_dmg() {
    test_mooneye::<Dmg>("ei_timing.gb");
}

#[test]
fn halt_ime0_ei_dmg() {
    test_mooneye::<Dmg>("halt_ime0_ei.gb");
}

#[test]
fn halt_ime0_nointr_timing_dmg() {
    test_mooneye::<Dmg>("halt_ime0_nointr_timing.gb");
}

#[test]
fn halt_ime1_timing_dmg() {
    test_mooneye::<Dmg>("halt_ime1_timing.gb");
}

#[test]
fn halt_ime1_timing2_gs_dmg() {
    test_mooneye::<Dmg>("halt_ime1_timing2-GS.gb");
}

#[test]
fn if_ie_registers_dmg() {
    test_mooneye::<Dmg>("if_ie_registers.gb");
}

#[test]
fn intr_timing_dmg() {
    test_mooneye::<Dmg>("intr_timing.gb");
}

#[test]
fn jp_cc_timing_dmg() {
    test_mooneye::<Dmg>("jp_cc_timing.gb");
}

#[test]
fn jp_timing_dmg() {
    test_mooneye::<Dmg>("jp_timing.gb");
}

#[test]
fn ld_hl_sp_e_timing_dmg() {
    test_mooneye::<Dmg>("ld_hl_sp_e_timing.gb");
}

#[test]
fn oam_dma_restart_dmg() {
    test_mooneye::<Dmg>("oam_dma_restart.gb");
}

#[test]
fn oam_dma_start_dmg() {
    test_mooneye::<Dmg>("oam_dma_start.gb");
}

#[test]
fn oam_dma_timing_dmg() {
    test_mooneye::<Dmg>("oam_dma_timing.gb");
}

#[test]
fn pop_timing_dmg() {
    test_mooneye::<Dmg>("pop_timing.gb");
}

#[test]
fn push_timing_dmg() {
    test_mooneye::<Dmg>("push_timing.gb");
}

#[test]
fn rapid_di_ei_dmg() {
    test_mooneye::<Dmg>("rapid_di_ei.gb");
}

#[test]
fn ret_cc_timing_dmg() {
    test_mooneye::<Dmg>("ret_cc_timing.gb");
}

#[test]
fn reti_intr_timing_dmg() {
    test_mooneye::<Dmg>("reti_intr_timing.gb");
}

#[test]
fn reti_timing_dmg() {
    test_mooneye::<Dmg>("reti_timing.gb");
}

#[test]
fn ret_timing_dmg() {
    test_mooneye::<Dmg>("ret_timing.gb");
}

#[test]
fn rst_timing_dmg() {
    test_mooneye::<Dmg>("rst_timing.gb");
}

// instr

#[test]
fn daa_dmg() {
    test_mooneye::<Dmg>("instr/daa.gb");
}

// interrupts

#[test]
fn ie_push_dmg() {
    test_mooneye::<Dmg>("interrupts/ie_push.gb");
}

// oam_dma

#[test]
fn oam_dma_basic_dmg() {
    test_mooneye::<Dmg>("oam_dma/basic.gb");
}

#[test]
fn oam_dma_reg_read_dmg() {
    test_mooneye::<Dmg>("oam_dma/reg_read.gb");
}

#[test]
fn oam_dma_sources_gs_dmg() {
    test_mooneye::<Dmg>("oam_dma/sources-GS.gb");
}

// ppu

#[test]
#[ignore]
fn hblank_ly_scx_timing_gs_dmg() {
    test_mooneye::<Dmg>("ppu/hblank_ly_scx_timing-GS.gb");
}

#[test]
fn intr_1_2_timing_gs_dmg() {
    test_mooneye::<Dmg>("ppu/intr_1_2_timing-GS.gb")
}

#[test]
fn intr_2_0_timing_dmg() {
    test_mooneye::<Dmg>("ppu/intr_2_0_timing.gb");
}

#[test]
fn intr_2_mode0_timing_dmg() {
    test_mooneye::<Dmg>("ppu/intr_2_mode0_timing.gb");
}

#[test]
#[ignore]
fn intr_2_mode0_timing_sprites_dmg() {
    test_mooneye::<Dmg>("ppu/intr_2_mode0_timing_sprites.gb");
}

#[test]
fn intr_2_mode3_timing_dmg() {
    test_mooneye::<Dmg>("ppu/intr_2_mode3_timing.gb");
}

#[test]
fn intr_2_oam_ok_timing_dmg() {
    test_mooneye::<Dmg>("ppu/intr_2_oam_ok_timing.gb");
}

#[test]
#[ignore]
fn lcdon_timing_gs_dmg() {
    test_mooneye::<Dmg>("ppu/lcdon_timing-GS.gb");
}

#[test]
#[ignore]
fn lcdon_write_timing_gs_dmg() {
    test_mooneye::<Dmg>("ppu/lcdon_write_timing-GS.gb");
}

#[test]
fn stat_irq_blocking_dmg() {
    test_mooneye::<Dmg>("ppu/stat_irq_blocking.gb");
}

#[test]
#[ignore]
fn stat_lyc_onoff_dmg() {
    test_mooneye::<Dmg>("ppu/stat_lyc_onoff.gb");
}

#[test]
#[ignore]
fn vblank_stat_intr_gs_dmg() {
    test_mooneye::<Dmg>("ppu/vblank_stat_intr-GS.gb");
}

// timer

#[test]
fn div_write_dmg() {
    test_mooneye::<Dmg>("timer/div_write.gb");
}

#[test]
fn rapid_toggle_dmg() {
    test_mooneye::<Dmg>("timer/rapid_toggle.gb");
}

#[test]
fn tim00_dmg() {
    test_mooneye::<Dmg>("timer/tim00.gb");
}

#[test]
fn tim00_div_trigger_dmg() {
    test_mooneye::<Dmg>("timer/tim00_div_trigger.gb");
}

#[test]
fn tim01_dmg() {
    test_mooneye::<Dmg>("timer/tim01.gb");
}

#[test]
fn tim01_div_trigger_dmg() {
    test_mooneye::<Dmg>("timer/tim01_div_trigger.gb");
}

#[test]
fn tim10_dmg() {
    test_mooneye::<Dmg>("timer/tim10.gb");
}

#[test]
fn tim10_div_trigger_dmg() {
    test_mooneye::<Dmg>("timer/tim10_div_trigger.gb");
}

#[test]
fn tim11_dmg() {
    test_mooneye::<Dmg>("timer/tim11.gb");
}

#[test]
fn tim11_div_trigger_dmg() {
    test_mooneye::<Dmg>("timer/tim11_div_trigger.gb");
}

#[test]
fn tima_reload_dmg() {
    test_mooneye::<Dmg>("timer/tima_reload.gb");
}

#[test]
fn tima_write_reloading_dmg() {
    test_mooneye::<Dmg>("timer/tima_write_reloading.gb");
}

#[test]
fn tma_write_reloading_dmg() {
    test_mooneye::<Dmg>("timer/tma_write_reloading.gb");
}

// cgb

#[test]
fn add_sp_e_timing_cgb() {
    test_mooneye::<Cgb>("add_sp_e_timing.gb")
}

#[test]
fn mem_oam_cgb() {
    test_mooneye::<Cgb>("bits/mem_oam.gb")
}

#[test]
fn reg_f_cgb() {
    test_mooneye::<Cgb>("bits/reg_f.gb")
}

#[test]
fn call_cc_timing_cgb() {
    test_mooneye::<Cgb>("call_cc_timing.gb");
}

#[test]
fn call_cc_timing2_cgb() {
    test_mooneye::<Cgb>("call_cc_timing2.gb");
}

#[test]
fn call_timing_cgb() {
    test_mooneye::<Cgb>("call_timing.gb");
}

#[test]
fn call_timing2_cgb() {
    test_mooneye::<Cgb>("call_timing2.gb");
}

#[test]
fn di_timing_gs_cgb() {
    test_mooneye::<Cgb>("di_timing-GS.gb");
}

#[test]
fn div_timing_cgb() {
    test_mooneye::<Cgb>("div_timing.gb");
}

#[test]
fn ei_sequence_cgb() {
    test_mooneye::<Cgb>("ei_sequence.gb");
}

#[test]
fn ei_timing_cgb() {
    test_mooneye::<Cgb>("ei_timing.gb");
}

#[test]
fn halt_ime0_ei_cgb() {
    test_mooneye::<Cgb>("halt_ime0_ei.gb");
}

#[test]
fn halt_ime0_nointr_timing_cgb() {
    test_mooneye::<Cgb>("halt_ime0_nointr_timing.gb");
}

#[test]
fn halt_ime1_timing_cgb() {
    test_mooneye::<Cgb>("halt_ime1_timing.gb");
}

#[test]
fn halt_ime1_timing2_gs_cgb() {
    test_mooneye::<Cgb>("halt_ime1_timing2-GS.gb");
}

#[test]
fn if_ie_registers_cgb() {
    test_mooneye::<Cgb>("if_ie_registers.gb");
}

#[test]
fn intr_timing_cgb() {
    test_mooneye::<Cgb>("intr_timing.gb");
}

#[test]
fn jp_cc_timing_cgb() {
    test_mooneye::<Cgb>("jp_cc_timing.gb");
}

#[test]
fn jp_timing_cgb() {
    test_mooneye::<Cgb>("jp_timing.gb");
}

#[test]
fn ld_hl_sp_e_timing_cgb() {
    test_mooneye::<Cgb>("ld_hl_sp_e_timing.gb");
}

#[test]
fn oam_dma_restart_cgb() {
    test_mooneye::<Cgb>("oam_dma_restart.gb");
}

#[test]
fn oam_dma_start_cgb() {
    test_mooneye::<Cgb>("oam_dma_start.gb");
}

#[test]
fn oam_dma_timing_cgb() {
    test_mooneye::<Cgb>("oam_dma_timing.gb");
}

#[test]
fn pop_timing_cgb() {
    test_mooneye::<Cgb>("pop_timing.gb");
}

#[test]
fn push_timing_cgb() {
    test_mooneye::<Cgb>("push_timing.gb");
}

#[test]
fn rapid_di_ei_cgb() {
    test_mooneye::<Cgb>("rapid_di_ei.gb");
}

#[test]
fn ret_cc_timing_cgb() {
    test_mooneye::<Cgb>("ret_cc_timing.gb");
}

#[test]
fn reti_intr_timing_cgb() {
    test_mooneye::<Cgb>("reti_intr_timing.gb");
}

#[test]
fn reti_timing_cgb() {
    test_mooneye::<Cgb>("reti_timing.gb");
}

#[test]
fn ret_timing_cgb() {
    test_mooneye::<Cgb>("ret_timing.gb");
}

#[test]
fn rst_timing_cgb() {
    test_mooneye::<Cgb>("rst_timing.gb");
}

// instr

#[test]
fn daa_cgb() {
    test_mooneye::<Cgb>("instr/daa.gb");
}

// interrupts

#[test]
fn ie_push_cgb() {
    test_mooneye::<Cgb>("interrupts/ie_push.gb");
}

// oam_dma

#[test]
fn oam_dma_basic_cgb() {
    test_mooneye::<Cgb>("oam_dma/basic.gb");
}

#[test]
fn oam_dma_reg_read_cgb() {
    test_mooneye::<Cgb>("oam_dma/reg_read.gb");
}

#[test]
fn oam_dma_sources_gs_cgb() {
    test_mooneye::<Cgb>("oam_dma/sources-GS.gb");
}

// ppu

#[test]
#[ignore]
fn hblank_ly_scx_timing_gs_cgb() {
    test_mooneye::<Cgb>("ppu/hblank_ly_scx_timing-GS.gb");
}

#[test]
fn intr_1_2_timing_gs_cgb() {
    test_mooneye::<Cgb>("ppu/intr_1_2_timing-GS.gb")
}

#[test]
fn intr_2_0_timing_cgb() {
    test_mooneye::<Cgb>("ppu/intr_2_0_timing.gb");
}

#[test]
fn intr_2_mode0_timing_cgb() {
    test_mooneye::<Cgb>("ppu/intr_2_mode0_timing.gb");
}

#[test]
#[ignore]
fn intr_2_mode0_timing_sprites_cgb() {
    test_mooneye::<Cgb>("ppu/intr_2_mode0_timing_sprites.gb");
}

#[test]
fn intr_2_mode3_timing_cgb() {
    test_mooneye::<Cgb>("ppu/intr_2_mode3_timing.gb");
}

#[test]
fn intr_2_oam_ok_timing_cgb() {
    test_mooneye::<Cgb>("ppu/intr_2_oam_ok_timing.gb");
}

#[test]
#[ignore]
fn lcdon_timing_gs_cgb() {
    test_mooneye::<Cgb>("ppu/lcdon_timing-GS.gb");
}

#[test]
#[ignore]
fn lcdon_write_timing_gs_cgb() {
    test_mooneye::<Cgb>("ppu/lcdon_write_timing-GS.gb");
}

#[test]
fn stat_irq_blocking_cgb() {
    test_mooneye::<Cgb>("ppu/stat_irq_blocking.gb");
}

#[test]
#[ignore]
fn stat_lyc_onoff_cgb() {
    test_mooneye::<Cgb>("ppu/stat_lyc_onoff.gb");
}

#[test]
#[ignore]
fn vblank_stat_intr_gs_cgb() {
    test_mooneye::<Cgb>("ppu/vblank_stat_intr-GS.gb");
}

// timer

#[test]
fn div_write_cgb() {
    test_mooneye::<Cgb>("timer/div_write.gb");
}

#[test]
fn rapid_toggle_cgb() {
    test_mooneye::<Cgb>("timer/rapid_toggle.gb");
}

#[test]
fn tim00_cgb() {
    test_mooneye::<Cgb>("timer/tim00.gb");
}

#[test]
fn tim00_div_trigger_cgb() {
    test_mooneye::<Cgb>("timer/tim00_div_trigger.gb");
}

#[test]
fn tim01_cgb() {
    test_mooneye::<Cgb>("timer/tim01.gb");
}

#[test]
fn tim01_div_trigger_cgb() {
    test_mooneye::<Cgb>("timer/tim01_div_trigger.gb");
}

#[test]
fn tim10_cgb() {
    test_mooneye::<Cgb>("timer/tim10.gb");
}

#[test]
fn tim10_div_trigger_cgb() {
    test_mooneye::<Cgb>("timer/tim10_div_trigger.gb");
}

#[test]
fn tim11_cgb() {
    test_mooneye::<Cgb>("timer/tim11.gb");
}

#[test]
fn tim11_div_trigger_cgb() {
    test_mooneye::<Cgb>("timer/tim11_div_trigger.gb");
}

#[test]
fn tima_reload_cgb() {
    test_mooneye::<Cgb>("timer/tima_reload.gb");
}

#[test]
fn tima_write_reloading_cgb() {
    test_mooneye::<Cgb>("timer/tima_write_reloading.gb");
}

#[test]
fn tma_write_reloading_cgb() {
    test_mooneye::<Cgb>("timer/tma_write_reloading.gb");
}
