# CD-i Bad Apple!!

This is a tech demo for the Philips CD-i, which plays the famous monochrome music video "Bad Apple!!",
without the usage of the digital video cartridge.
It is inspired by the technique used by the game "Burn:Cycle".

Features
* RL3 video mode is used for compression
  * Full frame rate of 30 FPS at the maximum PAL resolution of 768x280
* Audio via APDCM at Level B
  * 1/4th of the CD datarate for audio playback
  * 4 bit samples at 37.8 kHz in stereo
* 2 separate video streams for 50 and 60 Hz modes to always have the right pixel aspect ratio
* A custom RLE encoder, written in Rust

## Prerequisites

Clone https://github.com/TwBurn/cdi-sdk by updating the git submodules and have it mounted as D: drive in winecfg. That can be done via script.

    ln -s $(realpath cdi-sdk) ~/.wine/dosdevices/d:

Before the disc can be authored, we need to encode the video material.
Please refer to the [converter README](converter/) for further instructions.

## Building

    ./make_image.sh

## Burning

For some reason, cdrdao doesn't like upper case CUE files. We need to rename it first.

    cd disk
    cp BADAPPLE.CUE BADAPPLE.cue
    cdrdao write --speed 1 --swap -n BADAPPLE.cue

## Emulating

To keep iterations short, starting an emulator fast can be a priority

For fast testing on MiSTer, scp works well. Afterwards use the User button to reset the CD-i.

    scp disk/BADAPPLE.CUE disk/BADAPPLE.BIN root@mister:/media/fat/games/CD-i

For fast testing using cdiemu, this works and also gives the console output

    wine wcdiemu-v053b9.exe -term uart disk/BADAPPLE.BIN -start -playcdi

For fast testing using mame, this works. Keep in mind that no console output is supported.

    mame cdimono1 -cdrom disk/BADAPPLE.CUE

## Compatibility

This demo was tested on these platforms

* [CD-i Emulator (cdiemu)](https://www.cdiemu.org/)
  * At least on cdiemu-0.5.3-beta9 there is an audio video sync problem in 60 Hz mode
* [MAME](https://www.mamedev.org/) (cdimono1)
  * 60 Hz mode not supported
  * Reset back to system menu seems to be not supported
* [MiSTer FPGA CD-i core](https://github.com/MiSTer-devel/CDi_MiSTer)
* Philips CD-i 210/05 with 50/60 Hz switch

## TODO

* Test with dirty CDs on real hardware
* Improve Video buffer verifier of the encoder. Buffer fifo level assumption doesn't match the actual playback.
* Interlacing for more vertical resolution? Data rate might be a problem.

## References

Lots of init code was taken from https://github.com/TwBurn/Nobelia
