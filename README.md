# CD-i Bad Apple

This is a tech demo for the Philips CD-i which plays the famous monochrome music video "Bad Apple", without the usage of the digital video cartridge.

The video mode RL7 is used for compression, to allow the full frame rate of 30 FPS at the maximum PAL resolution of 384x280.
For audio, Level B is used, which allocates 1/4th of the CD datarate for playback of 4 bit ADPCM at 37.8 kHz in stereo.

A custom RLE encoder is written in Rust.

## Prerequisites

Clone https://github.com/TwBurn/cdi-sdk by updating the git submodules and have it mounted as D: drive in winecfg. That can be done via script.

	ln -s $(realpath cdi-sdk) ~/.wine/dosdevices/d:

Before the disc can be authored, we need to encode the video material.
Please refer to the [converter README](converter/) for further instructions.

## Building

    ./make_image.sh

## Compatibility

This demo was tested on these platforms

* [CD-i Emulator (cdiemu)](https://www.cdiemu.org/)
* MAME (cdimono1)
* MiSTer FPGA CD-i core

## TODO

* Fix underflows on real hardware
* Test with dirty CDs on real hardware
* Improve Video buffer verifier of the encoder. Buffer fifo level assumption doesn't match the actual playback.
* Interlacing for more vertical resolution? Data rate might be a problem.

## References

Lots of init code was taken from https://github.com/TwBurn/Nobelia
