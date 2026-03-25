INCLUDE "hardware.inc"

section "Header", rom0[$100]
	jp EntryPoint

	ds $150 - @, 0

EntryPoint:
    di
    ld a, LCDC_ON | LCDC_WIN_OFF | LCDC_BG_ON | LCDC_BLOCK01
    ldh [rLCDC], a
    ld a, %11_11_11_00
    ldh [rBGP], a

    xor a
    ldh [rSB], a
    ldh [rDIV], a
    ld a, SC_START | SC_INTERNAL
    ldh [rSC], a
    REPT 16
    ; doesn't work with 28
    REPT 29
    nop
    ENDR
    ldh [rDIV], a
    ENDR
    ldh a, [rSB]
    cp $ff
    jr nz, .end

    ld a, %00_00_00_11
    ldh [rBGP], a
.end
    ei
    nop
    halt
    jr .end
