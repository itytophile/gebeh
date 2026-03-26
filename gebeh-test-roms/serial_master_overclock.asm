; Speed up the serial transfer by writing to the DIV register.
; Boot screen is inverted if success.

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

    ; overclock works
    xor a
    ldh [rSB], a
    ldh [rDIV], a
    ld a, SC_START | SC_INTERNAL
    ldh [rSC], a
    REPT 16
    REPT 29
    nop
    ENDR
    ldh [rDIV], a
    ENDR
    ldh a, [rSB]
    cp $ff
    jp nz, .end

    ; div writes too fast, overclock fails
    xor a
    ldh [rSB], a
    ldh [rDIV], a
    ld a, SC_START | SC_INTERNAL
    ldh [rSC], a
    REPT 16
    REPT 28
    nop
    ENDR
    ldh [rDIV], a
    ENDR
    ldh a, [rSB]
    cp $ff
    jr z, .end
    ; catch ld b, b to know if there is a success
    ld b, b
    ld a, %00_00_00_11
    ldh [rBGP], a
.end
    ; catch ld c, c to know if there is a failure
    ld c, c
    ei
    nop
    halt
    jr .end
