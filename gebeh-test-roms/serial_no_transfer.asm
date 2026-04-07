INCLUDE "hardware.inc"

; enable master mode after joypad press
section "joypad", rom0[INT_HANDLER_JOYPAD]
    ld a, IE_SERIAL
    ldh [rIE], a
    ei
    jp master_mode

section "serial", rom0[INT_HANDLER_SERIAL]
    ei ; to halt later
    xor a
    ldh [rIE], a
    jp check

section "Header", rom0[$100]
	jp EntryPoint

	ds $150 - @, 0

EntryPoint:
    di

    ld a, JOYP_GET_BUTTONS
    ldh [rJOYP], a
    ld a, LCDC_ON | LCDC_WIN_OFF | LCDC_BG_ON | LCDC_BLOCK01
    ldh [rLCDC], a
    ld a, %11_11_11_00
    ldh [rBGP], a

    ld a, IE_SERIAL | IE_JOYPAD
    ldh [rIE], a
    ld a, 67
    ldh [rSB], a
    xor a
    ldh [rSC], a
    ei
    ; to tell the emulator that it can inject an input
    ld a, a
    ; don't reti in case the interrupt is fired before the halt (problem happening if spamming buttons irl)
    halt
master_mode:
    ; master mode
    ld a, SC_START | SC_INTERNAL
    ldh [rSC], a
    halt
check:
    ldh a, [rSB]
    and a
    jr nz, .end
    ; success
    ld b, 7
    ld a, %00_00_00_11
    ldh [rBGP], a
.end
    ; end of program
    ld h, 7
    halt
