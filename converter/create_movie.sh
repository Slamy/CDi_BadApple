set -e

mkdir -p pics240
mkdir -p pics280

# Sharp grin at 01654 to 01722. Should disable blur there to keep the grin as it might be important for the aesthetic
# 06530 is one of the last frames before the credits

# We use -vf "format=gray,gblur=sigma=2:enable='not(between(n,1683,1752))'" to remove noise during fast movement
# We use -vf "format=gray,gblur=sigma=2,geq=lum_expr='if(gte(lum(X,Y),100),255,0)'" to check its efficiency before using the RLE encoder
# I'm usually no friend of AI, but thx to ChatGPT for this nice suggestion

# Raw frames for manual edit
# ffmpeg -ss 1 -i *.webm pics/%05d.png

ffmpeg -ss 1 -i *.webm -vf "format=gray,gblur=sigma=1.5:enable='not(between(n,1653,1722))'" -s 768x280 pics280/%05d.png
ffmpeg -ss 1 -i *.webm -vf "format=gray,gblur=sigma=1.5:enable='not(between(n,1653,1722))'" -s 768x240 pics240/%05d.png

ffmpeg -y -i manualpic/credits0.png -s 768x280 manualpic/credits0_280.png
ffmpeg -y -i manualpic/credits0.png -s 768x240 manualpic/credits0_240.png
ffmpeg -y -i manualpic/endcard.png  -s 768x280 manualpic/credits1_280.png
ffmpeg -y -i manualpic/endcard.png  -s 768x240 manualpic/credits1_240.png

# Insert Frame number
# ffmpeg -i *.webm -vf "drawtext=text='%{n}':x=10:y=10:fontsize=60:boxborderw=20:fontcolor=white:box=1:boxcolor=black" -s 768x280 pics280/%05d.png

cargo run --release pics280
mv MOV280.DAT ../build/MOV280.DAT

cargo run --release pics240
mv MOV240.DAT ../build/MOV240.DAT

#ffmpeg -i *.webm -vf "format=gray,geq=lum_expr='if(gte(lum(X,Y),100),255,0)'" -s 768x280 pics280/%05d.png
#ffmpeg -i *.webm -vf "format=gray,gblur=sigma=2,geq=lum_expr='if(gte(lum(X,Y),100),255,0)'" -s 768x280 pics280/%05d.png
#ffmpeg -i *.webm -vf "format=gray,gblur=sigma=2,geq=lum_expr='if(gte(lum(X,Y),100),255,0)'" pics/%05d.png

#ffmpeg -i *.webm -vf "format=gray,geq=lum_expr='if(gte(lum(X,Y),100),255,0)'" pics1/%05d.png
#ffmpeg -i *.webm -vf "format=gray,gblur=sigma=1.5,geq=lum_expr='if(gte(lum(X,Y),100),255,0)'" pics2/%05d.png
#ffmpeg -i *.webm -vf "format=gray,gblur=sigma=2,geq=lum_expr='if(gte(lum(X,Y),100),255,0)'" pics3/%05d.png

# bcompare pics2/01215.png pics3/01215.png

#ffmpeg -i endcard.png -vf "format=gray,geq=lum_expr='if(gte(lum(X,Y),100),255,0)'" -s 768x280 endcard280.png
#ffmpeg -i endcard.png -vf "format=gray,geq=lum_expr='if(gte(lum(X,Y),100),255,0)'" -s 768x280 endcard280.png

