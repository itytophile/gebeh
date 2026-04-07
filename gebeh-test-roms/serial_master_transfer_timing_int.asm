; Boot screen is inverted if success.

INCLUDE "hardware.inc"

section "serial", rom0[INT_HANDLER_SERIAL]
    xor a
    jp hl

section "Header", rom0[$100]
	jp EntryPoint

	ds $150 - @, 0

EntryPoint:
    ld hl, check
    ld a, IE_SERIAL
    ldh [rIE], a
    ei
    ld a, LCDC_ON | LCDC_WIN_OFF | LCDC_BG_ON | LCDC_BLOCK01
    ldh [rLCDC], a
    ld a, %11_11_11_00
    ldh [rBGP], a

    ld b, 42
    ld c, 67

    ; completed transfer
    xor a
    ldh [rDIV], a
    ; 1 m cycle nop after write
    ld a, SC_START | SC_INTERNAL
    ; 2 m cycle
    ldh [rSC], a
    ; 3 m cycle
    REPT 1017
    nop
    ENDR
    di
    ld a, b
check:
    ; the interrupt shouldn't have happened
    cp b
    jp nz, end
    ld hl, check2
    xor a
    ldh [rIF], a
    ei
    ldh [rDIV], a
    ld a, SC_START | SC_INTERNAL
    ldh [rSC], a
    REPT 1018
    nop
    ENDR
    di
    ld a, b
check2:
    ; the interrupt should have happened
    and a
    jp nz, end
    ; catch ld b, b to know if there is a success
    ld b, b
    ld a, %00_00_00_11
    ldh [rBGP], a
end:
    ld c, c
    ei
    nop
    halt
    jr end
