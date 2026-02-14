INCLUDE "hardware.inc"

section "lcdc", rom0[INT_HANDLER_STAT]
    ; 5 m-cycles interrupt handling
    ; to discard the interrupt call push and not fill the stack with garbage
    ; 3 m-cycles
    pop hl
    ; 4 m-cycles
    jp lcdc

section "Header", rom0[$100]
	jp EntryPoint

	ds $150 - @, 0

EntryPoint:
    di
    ld c, LOW(rBGP)
    ld a, STAT_LYC
    ldh [rSTAT], a
    ld a, IE_STAT
    ldh [rIE], a
    ld a, LCDC_ON | LCDC_WIN_OFF | LCDC_BG_ON | LCDC_BG_MAP
    ldh [rLCDC], a
    ld b, 2
    ld e, 1
    ei
wait:
    REPT 1200
    nop
    ENDR
lcdc:
    REPT 9
    nop
    ENDR
    ; 1 m-cycle
    ; light gray
    ld a, e
    ; 2 m-cycles (write at first cycle)
    ldh [c], a
    ldh a, [rLY]
    cp 143
    jr c, .in_screen
    xor a
    ldh [rLYC], a
    jp wait
.in_screen
    inc a
    ldh [rLYC], a
    REPT 24
    nop
    ENDR
    ; 1 m-cycle
    ; dark gray
    ld a, b
    ; 2 m-cycle (write at first cycle)
    ldh [c], a
    ei
    jp wait
