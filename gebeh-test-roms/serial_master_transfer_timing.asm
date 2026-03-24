; according to mooneye and my observations with a Game Boy Pocket,
; the serial is synchronized with the system clock.
; I think we need 1024 M-cycles (possible off by one error) to transfer one byte
; The system clock (4194304 / 4) divided by byte transfer frequency (8192 / 8) = 1024

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

    ; completed transfer
    xor a
    ldh [rSB], a
    ldh [rDIV], a
    ; 1 m cycle nop after write
    ld a, SC_START | SC_INTERNAL
    ; 2 m cycle
    ldh [rSC], a
    ; 3 m cycle
    REPT 1016
    nop
    ENDR
    ; read at the third cycle
    ldh a, [rSB]
    cp $ff
    jp nz, .end

    ; transfer not completed
    xor a
    ldh [rSB], a
    ldh [rDIV], a
    ld a, SC_START | SC_INTERNAL
    ldh [rSC], a
    REPT 1015
    nop
    ENDR
    ldh a, [rSB]
    cp $ff
    jp z, .end

    ; completed transfer with nop after clock reset
    xor a
    ldh [rSB], a
    ldh [rDIV], a
    nop
    ld a, SC_START | SC_INTERNAL
    ldh [rSC], a
    REPT 1015
    nop
    ENDR
    ldh a, [rSB]
    cp $ff
    jp nz, .end

    ; transfer not completed with nop after clock reset
    xor a
    ldh [rSB], a
    ldh [rDIV], a
    nop
    ld a, SC_START | SC_INTERNAL
    ldh [rSC], a
    REPT 1014
    nop
    ENDR
    ldh a, [rSB]
    cp $ff
    jp z, .end

    ld a, %00_00_00_11
    ldh [rBGP], a
.end
    ei
    nop
    halt
    jr .end
