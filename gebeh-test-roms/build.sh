#!/bin/bash

for file in *.asm; do
    base="${file%.asm}"

    rgbasm -o "$base.o" "$file"
    rgblink -o "$base.gb" "$base.o"
    rgbfix -v -p 0xFF "$base.gb"
done
