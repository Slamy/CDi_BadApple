# Custom RLE Encoder

This Rust application is able to convert a sequence of PNG images to a RL7 data stream for playback on a Philips CD-i

## Requirements

The [remastered upload of Masayoshi Minoshima / ALSTROEMERIA RECORDS](https://www.youtube.com/watch?v=i41KoE0iMY) is used. `yt-dlp` can be used for fetching.

    yt-dlp https://www.youtube.com/watch?v=i41KoE0iMY

## Building the Video stream

You need to have a [Rust compiler](https://rust-lang.org/) for this.
Please use the script to handle everything

    ./create_movie.sh

For debugging purposes, the frame count can be added to the video

    ffmpeg -i *.webm -s 768x280 -vf "drawtext=text='%{n}':x=10:y=10:fontsize=60:boxborderw=20:fontcolor=white:box=1:boxcolor=black" pics/%05d.png

## Building the Audio stream

For conversion, `ACU Shell` from the [CD-i SDK](../cdi-sdk/TOOLS/Master/) is used.

Since wave file don't work too well, we need to convert the audio to AIFF.

    ffmpeg -i *.webm -vn -ar 37800 audio.aiff

Now, we can use wine on Linux or directly start it on Windows

    wine ../cdi-sdk/TOOLS/Master/ACUSHELL.EXE

* Select `audio.aiff` on the left.
* Keep `Encode` as output format
* Press `Select Options`
* Select `B level`
* Press `OK` to close the options
* Press `OK` to start conversion

The tool is quite quirky, it will start conversion without giving a sign, that it actually does something. Don't close it even so it seems to have finished the task!

Keep observing `audio.ACM`. It should grow slowly in size.
As an alternative you can keep track of the ACUShell output like this

    cat ../cdi-sdk/TOOLS/Master/acushell.log

It should print some numbers and `Done!` at the end.

When the process is stopped, we can copy it to our build folder.

    cp audio.ACM ../build/LevelB.ACM 


