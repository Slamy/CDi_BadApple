#!/bin/bash

# Abort on error
set -e

# Clear files to be sure that we have generated them
rm -f disk/*.BIN build/*.CDI disk/*.CUE disk/*.TOC build/MASTER.LOG

# First build the application
time WINEPATH=D:/DOS/BIN wine D:/dos/bin/bmake.exe link_cd

# Then start MS-DOS to master the image
# Use a custom configuration to overclock the machine
# Use dummy video driver to hide the window
time SDL_VIDEODRIVER=dummy dosbox -conf dosbox.conf master.bat -exit

# The error codes are not available on MSDOS. We check for the log to be sure
cat build/MASTER.LOG
grep "End   generation of album" build/MASTER.LOG

# Convert the CDI/TOC files into CUE/BIN by cutting off the first 150 sectors
dd skip=150 bs=2352 if=disk/BADAPPLE.CDI of=disk/BADAPPLE.BIN
echo "FILE BADAPPLE.BIN BINARY
  TRACK 01 MODE2/2352
    INDEX 01 00:00:00" > disk/BADAPPLE.CUE

sed -i -e "s/FILE .*\\\\/FILE /" disk/*.CUE

chdman createcd -f -i disk/BADAPPLE.CUE -o disk/BADAPPLE.CHD

echo " --- Done! ---"
