cgb_boot.asm, sameboot.inc, hardware.inc from [SameBoy](https://github.com/LIJI32/SameBoy/tree/208ba4afabffab9edde416f2dbb8ae459e34adb8/BootROMs).
cgb_boot.asm edited to remove the logo handling.

## Build boot rom
```sh
rgbasm -o cgb_boot.o cgb_boot.asm
rgblink -x -o bootrom cgb_boot.o
```
