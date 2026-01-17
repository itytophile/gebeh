use gebeh::InstantRtc;
use gebeh_core::Emulator;
use gebeh_front_helper::get_mbc;

fn test_mooneye(path: &str) {
    let rom = std::fs::read(format!(
        "./downloads/mts-20240926-1737-443f6e1/acceptance/{path}"
    ))
    .unwrap();
    let rom = rom.as_slice();
    let mut emulator = Emulator::default();
    let mut mbc = get_mbc::<_, InstantRtc>(rom).unwrap();

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
fn add_sp_e_timing() {
    test_mooneye("add_sp_e_timing.gb")
}

#[test]
fn mem_oam() {
    test_mooneye("bits/mem_oam.gb")
}

#[test]
fn reg_f() {
    test_mooneye("bits/reg_f.gb")
}

#[test]
fn unused_hwio_gs() {
    test_mooneye("bits/unused_hwio-GS.gb")
}

#[test]
fn call_cc_timing() {
    test_mooneye("call_cc_timing.gb");
}

#[test]
fn call_cc_timing2() {
    test_mooneye("call_cc_timing2.gb");
}

#[test]
fn call_timing() {
    color_eyre::install().unwrap();
    env_logger::init();
    test_mooneye("call_timing.gb");
}

#[test]
fn call_timing2() {
    test_mooneye("call_timing2.gb");
}

#[test]
fn di_timing_gs() {
    test_mooneye("di_timing-GS.gb");
}

#[test]
fn div_timing() {
    test_mooneye("div_timing.gb");
}

#[test]
fn ei_sequence() {
    test_mooneye("ei_sequence.gb");
}

#[test]
fn ei_timing() {
    test_mooneye("ei_timing.gb");
}

#[test]
fn halt_ime0_ei() {
    test_mooneye("halt_ime0_ei.gb");
}

#[test]
fn halt_ime0_nointr_timing() {
    test_mooneye("halt_ime0_nointr_timing.gb");
}

#[test]
fn halt_ime1_timing() {
    test_mooneye("halt_ime1_timing.gb");
}

#[test]
fn halt_ime1_timing2_gs() {
    test_mooneye("halt_ime1_timing2-GS.gb");
}

#[test]
fn if_ie_registers() {
    test_mooneye("if_ie_registers.gb");
}

#[test]
fn intr_timing() {
    test_mooneye("intr_timing.gb");
}

#[test]
fn jp_cc_timing() {
    test_mooneye("jp_cc_timing.gb");
}

#[test]
fn jp_timing() {
    test_mooneye("jp_timing.gb");
}

#[test]
fn ld_hl_sp_e_timing() {
    test_mooneye("ld_hl_sp_e_timing.gb");
}

#[test]
fn oam_dma_restart() {
    test_mooneye("oam_dma_restart.gb");
}

#[test]
fn oam_dma_start() {
    test_mooneye("oam_dma_start.gb");
}

#[test]
fn oam_dma_timing() {
    test_mooneye("oam_dma_timing.gb");
}

#[test]
fn pop_timing() {
    test_mooneye("pop_timing.gb");
}

#[test]
fn push_timing() {
    test_mooneye("push_timing.gb");
}

#[test]
fn rapid_di_ei() {
    test_mooneye("rapid_di_ei.gb");
}

#[test]
fn ret_cc_timing() {
    test_mooneye("ret_cc_timing.gb");
}

#[test]
fn reti_intr_timing() {
    test_mooneye("reti_intr_timing.gb");
}

#[test]
fn reti_timing() {
    test_mooneye("reti_timing.gb");
}

#[test]
fn ret_timing() {
    test_mooneye("ret_timing.gb");
}

#[test]
fn rst_timing() {
    test_mooneye("rst_timing.gb");
}

// instr

#[test]
fn daa() {
    test_mooneye("instr/daa.gb");
}

// interrupts

#[test]
fn ie_push() {
    test_mooneye("interrupts/ie_push.gb");
}

// oam_dma

#[test]
fn oam_dma_basic() {
    test_mooneye("oam_dma/basic.gb");
}

#[test]
fn oam_dma_reg_read() {
    test_mooneye("oam_dma/reg_read.gb");
}

#[test]
fn oam_dma_sources_gs() {
    test_mooneye("oam_dma/sources-GS.gb");
}

// ppu

#[test]
fn hblank_ly_scx_timing_gs() {
    test_mooneye("ppu/hblank_ly_scx_timing-GS.gb");
}

#[test]
fn intr_1_2_timing_gs() {
    test_mooneye("ppu/intr_1_2_timing-GS.gb")
}

#[test]
fn intr_2_0_timing() {
    test_mooneye("ppu/intr_2_0_timing.gb");
}

#[test]
fn intr_2_mode0_timing() {
    test_mooneye("ppu/intr_2_mode0_timing.gb");
}

#[test]
fn intr_2_mode0_timing_sprites() {
    test_mooneye("ppu/intr_2_mode0_timing_sprites.gb");
}

#[test]
fn intr_2_mode3_timing() {
    test_mooneye("ppu/intr_2_mode3_timing.gb");
}

#[test]
fn intr_2_oam_ok_timing() {
    test_mooneye("ppu/intr_2_oam_ok_timing.gb");
}

#[test]
#[ignore]
fn lcdon_timing_gs() {
    test_mooneye("ppu/lcdon_timing-GS.gb");
}

#[test]
#[ignore]
fn lcdon_write_timing_gs() {
    test_mooneye("ppu/lcdon_write_timing-GS.gb");
}

#[test]
fn stat_irq_blocking() {
    test_mooneye("ppu/stat_irq_blocking.gb");
}

#[test]
#[ignore]
fn stat_lyc_onoff() {
    test_mooneye("ppu/stat_lyc_onoff.gb");
}

#[test]
fn vblank_stat_intr_gs() {
    test_mooneye("ppu/vblank_stat_intr-GS.gb");
}

// timer

#[test]
fn div_write() {
    test_mooneye("timer/div_write.gb");
}

#[test]
fn rapid_toggle() {
    test_mooneye("timer/rapid_toggle.gb");
}

#[test]
fn tim00() {
    test_mooneye("timer/tim00.gb");
}

#[test]
fn tim00_div_trigger() {
    test_mooneye("timer/tim00_div_trigger.gb");
}

#[test]
fn tim01() {
    test_mooneye("timer/tim01.gb");
}

#[test]
fn tim01_div_trigger() {
    test_mooneye("timer/tim01_div_trigger.gb");
}

#[test]
fn tim10() {
    test_mooneye("timer/tim10.gb");
}

#[test]
fn tim10_div_trigger() {
    test_mooneye("timer/tim10_div_trigger.gb");
}

#[test]
fn tim11() {
    test_mooneye("timer/tim11.gb");
}

#[test]
fn tim11_div_trigger() {
    test_mooneye("timer/tim11_div_trigger.gb");
}

#[test]
fn tima_reload() {
    test_mooneye("timer/tima_reload.gb");
}

#[test]
fn tima_write_reloading() {
    test_mooneye("timer/tima_write_reloading.gb");
}

#[test]
fn tma_write_reloading() {
    test_mooneye("timer/tma_write_reloading.gb");
}
