INCLUDE "hardware.inc"

def BEGIN_BYTES equ $c000

; enable master mode after joypad press
section "joypad", rom0[INT_HANDLER_JOYPAD]
    jp master

section "serial", rom0[INT_HANDLER_SERIAL]
    reti

section "Header", rom0[$100]
	jp EntryPoint

	ds $150 - @, 0

EntryPoint:
    di
    ld b, 0
    ld hl, BEGIN_BYTES
    ld a, SC_START
    ldh [rSC], a
    ld a, JOYP_GET_BUTTONS
    ldh [rJOYP], a
    ld a, LCDC_ON | LCDC_WIN_OFF | LCDC_BG_ON | LCDC_BLOCK01
    ldh [rLCDC], a
    ld a, %11_11_11_00
    ldh [rBGP], a
    ld a, IE_SERIAL | IE_JOYPAD
    ei
    ldh [rIE], a
start:
    ; master interrupt triggered here
    ; sync serial transfer interrupt fired by master here
    halt
    ld a, IE_SERIAL
    ldh [rIE], a
exchange_byte:
    ld a, b
    ldh [rSB], a
    ldh a, [rSC]
    ; yeah we execute that as a slave too but who cares
    or SC_START
    ldh [rSC], a
    halt
    ldh a, [rSB]
    ld [hl+], a
    inc b
    jr nz, exchange_byte
; check bytes
    ld hl, BEGIN_BYTES
    ld b, 0
    REPT 255
    ld a, [hl+]
    cp b
    jp nz, end
    inc b
    ENDR
; success
    ld a, %00_00_00_11
    ldh [rBGP], a
end:
    halt
    jr end

master:
    ld a, IE_SERIAL
    ldh [rIE], a
    ; first transfer to sync the master and slave
    ld a, SC_INTERNAL | SC_START
    ldh [rSC], a
    ei
    jp start
