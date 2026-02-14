INCLUDE "hardware.inc"


SECTION "lcdc", ROM0[INT_HANDLER_STAT]
    ; 5 m-cycles interrupt handling
    ; to discard the interrupt call push and not fill the stack with garbage
    ; 3 m-cycles
    pop hl
    ; 4 m-cycles
    jp lcdc

SECTION "Header", ROM0[$100]
	jp EntryPoint

	ds $150 - @, 0

EntryPoint:
    di
    ld c, LOW(rBGP)
    ld a, STAT_MODE_2
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
    ; 10 m-cycles
    REPT 10
    nop
    ENDR
    ; 1 m-cycle
    ; light gray
    ld a, e
    ; 2 m-cycles (write at first cycle)
    ldh [c], a
    ; 36 m-cycles
    REPT 36
    nop
    ENDR
    ; 1 m-cycle
    ; dark gray
    ld a, b
    ; 2 m-cycle (write at first cycle)
    ldh [c], a
    ei
    jp wait

; After STAT 2 irq: write to palette in the 24th m-cycle and second write to palette in the 63th cycle
; 24 * 4 = 96
; 63 * 4 = 252
; OR effect visible on the second pixel from the left
; OR effect visible on the 158th pixel 
; 3 pixels right
; on line 0 (1 m-cycle delay before the interrupt)
; 6 black pixels left
; 0 black pixels right
