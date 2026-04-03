mkdir -p pics240
mkdir -p pics280

#ffmpeg -i *.webm -s 768x280 pics280/%05d.png
#ffmpeg -i *.webm -s 768x240 pics240/%05d.png

#ffmpeg -i *.webm -s 768x280 -vf "drawtext=text='%{n}':x=10:y=10:fontsize=60:boxborderw=20:fontcolor=white:box=1:boxcolor=black" pics/%05d.png

cargo run --release pics280
mv MOV280.DAT ../build/MOV280.DAT

cargo run --release pics240
mv MOV240.DAT ../build/MOV240.DAT
