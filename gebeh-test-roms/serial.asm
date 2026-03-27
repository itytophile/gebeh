INCLUDE "hardware.inc"

; enable master mode after joypad press
section "joypad", rom0[INT_HANDLER_JOYPAD]
    ld a, IE_SERIAL
    ld [rIE], a
    reti

section "serial", rom0[INT_HANDLER_SERIAL]
    ei
    jp check

section "Header", rom0[$100]
	jp EntryPoint

	ds $150 - @, 0

EntryPoint:
    di

    ld a, LCDC_ON | LCDC_WIN_OFF | LCDC_BG_ON | LCDC_BLOCK01
    ldh [rLCDC], a
    ld a, %11_11_11_00
    ldh [rBGP], a

    ; byte sent by master
    ld c, 42
    ; byte sent by slave
    ld d, 67
    ld a, IE_SERIAL | IE_JOYPAD
    ld [rIE], a
    ei
    ld a, d
    ldh [rSB], a
    ld a, SC_START
    ldh [rSC], a
    halt
    ; master mode
    ld a, c
    ldh [rSB], a
    ld a, SC_START | SC_INTERNAL
    ldh [rSC], a
    halt
check:
    ldh a, [rSC]
    cp SC_INTERNAL
    jr z, .is_master
    cp c
.is_master:
    cp d
    jr nz, .end
    ld b, b
    ld a, %00_00_00_11
.end
    ld c, c
    halt
