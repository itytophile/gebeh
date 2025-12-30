# gebeh

Pronounced gebeh.

## Bibliography

### General resources

- [Pan Docs](https://gbdev.io/pandocs/)
- [Game Boy: Complete Technical Reference](https://gekkio.fi/files/gb-docs/gbctr.pdf)
- [gbz80(7) — Game Boy CPU instruction reference](https://rgbds.gbdev.io/docs/v1.0.0/gbz80.7)

### CPU important details

- [Game Boy CPU internals](https://gist.github.com/SonoSooS/c0055300670d678b5ae8433e20bea595) - Very helpful document to understand HALT and interrupt handling (and other things).

### About PPU timings

- [The Timing of LYC STAT Handlers](https://gbdev.io/guides/lyc_timing.html)
- [The Cycle-Accurate Game Boy Docs](https://raw.githubusercontent.com/geaz/emu-gameboy/master/docs/The%20Cycle-Accurate%20Game%20Boy%20Docs.pdf) - Useful information about STAT register/interrupts behavior.
- [Nitty Gritty Gameboy Cycle Timing](http://blog.kevtris.org/blogfiles/Nitty%20Gritty%20Gameboy%20VRAM%20Timing.txt) - To know exactly what the PPU is doing during Mode 3 (Drawing). I still can't explain the difference between the "classic" 172 dots duration and the 173.5 (or 174 dots in the current implementation) duration described by the document.
- [Trying to understand Sprite FIFO behavior in the PPU (Reddit)](https://www.reddit.com/r/EmuDev/comments/s6cpis/gameboy_trying_to_understand_sprite_fifo_behavior/) - About sprite fetching timing and FIFOs.

### Test roms

- [Mooneye Test Suite](https://github.com/Gekkio/mooneye-test-suite)
- [Blargg's Gameboy hardware test ROMs](https://github.com/retrio/gb-test-roms)
- [Acid2](https://github.com/mattcurrie/dmg-acid2)

### Reference emulators

- [Mooneye GB](https://github.com/Gekkio/mooneye-gb) - Used for comparison to know why gebeh was failing some tests.
- [SameBoy](https://github.com/LIJI32/SameBoy) - Used to see how roms are supposed to run. Still can't understand the code, but there are interesting comments about STAT.
- [Boytacean](https://github.com/joamag/boytacean) - Can be used in the browser directly. MBC and some instructions implementation stolen from here.
- [RGY](https://github.com/YushiOMOTE/rgy) no-std emulator - Used its PPU implementation to test the CPU at first.

### Debugger & Disassembler

- [Gameroy](https://github.com/Rodrigodd/gameroy). It's an emulator too! Used it to know what Snorpung demos were doing.

### Stress test roms

- [Beautiful demos](https://files.scene.org/browse/demos/groups/snorpung/) by Snorpung.
