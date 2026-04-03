mkdir -p pics

ffmpeg -i *.webm -s 768x280 pics/%05d.png
#ffmpeg -i *.webm -s 768x280 -vf "drawtext=text='%{n}':x=10:y=10:fontsize=60:boxborderw=20:fontcolor=white:box=1:boxcolor=black" pics/%05d.png

cargo run --release
cp MOVIE.DAT ../build/MOVIE.DAT


