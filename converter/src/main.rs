#![feature(iter_array_chunks)]

use std::{collections::VecDeque, env, fs::File, io::Write};

use image::{DynamicImage, ImageReader};

#[derive(Clone)]
struct Rle {
    index: u8,
    cnt: u32,
}

type ColorMapperFn = fn(&u8) -> u8;

// shadows in 03456 are luma 73
fn map3shades(p: &u8) -> u8 {
    if *p > 100 {
        1
    } else if *p > 60 {
        2
    } else {
        0
    }
}

fn map2shades(p: &u8) -> u8 {
    if *p > 100 { 1 } else { 0 }
}

fn line_to_rle(line: &[u8], colormapper: ColorMapperFn) -> Vec<Rle> {
    let mut rle: Option<Rle> = None;
    let mut out = Vec::<Rle>::new();

    let clut4conv = line
        .iter()
        .map(colormapper)
        .array_chunks()
        .map(|f: [u8; 2]| (f[0] << 4) | f[1]);

    for pixel in clut4conv {
        if let Some(somerle) = rle.as_mut() {
            if somerle.index == pixel {
                somerle.cnt += 1;
            } else {
                out.push(rle.unwrap());
                rle = Some(Rle {
                    index: pixel,
                    cnt: 1,
                });
            }
        } else {
            rle = Some(Rle {
                index: pixel,
                cnt: 1,
            });
        }
    }
    out.push(rle.unwrap());

    out
}

fn encode_append_clut4rle(line: &[Rle], out: &mut Vec<u8>) {
    // Apply some limits.
    // A single CLUT7 entry can be repeated for 2-255 pixels.
    // There is one exception, 0 is reserved for "to end of line"
    for (index, entry) in line.iter().enumerate() {
        assert!(entry.cnt > 0);
        assert!(entry.cnt <= 255 * 2); // TODO

        if entry.cnt == 1 {
            // Single Pixel must not be RLE encoded
            out.push(entry.index & 0x7f);
        } else if index == line.len() - 1 {
            out.push(entry.index | 0x80);
            out.push(0); // duplicate this pixel to the end of line
        } else if entry.cnt > 255 {
            // Half it
            let first = entry.cnt / 2;
            let second = entry.cnt - first;

            out.push(entry.index | 0x80);
            out.push(first as u8);

            out.push(entry.index | 0x80);
            out.push(second as u8);
        } else {
            // Normal output
            assert!(entry.cnt <= 255);
            out.push(entry.index | 0x80);
            out.push(entry.cnt as u8);
        }
    }
}

fn parse_picture(img: DynamicImage, colormapper: ColorMapperFn) -> (VecDeque<Vec<u8>>, usize) {
    let img = img.into_luma8();

    assert!(img.width() == 768);
    let mut enclines = VecDeque::new();

    let lines = img.chunks(img.width() as usize);
    assert!(img.height() as usize == lines.len());
    let mut framesize = 0;
    //println!("{}", lines.len());
    for line in lines {
        let mut enclineout = Vec::<u8>::new();

        let raw_rle = line_to_rle(line, colormapper);

        let x: u32 = raw_rle.iter().map(|f| f.cnt).sum();
        assert!(x == img.width() / 2);

        encode_append_clut4rle(&raw_rle, &mut enclineout);
        framesize += enclineout.len();
        enclines.push_back(enclineout);

        //println!("{} {}", line.len(), out.len());
    }
    //println!("{}", out.len());

    (enclines, framesize)
}

const USER_BYTES_PER_MODE2_SECTOR: usize = 2324;
const NUMBER_OF_PCLS: usize = 125;

struct Mode2Sector {
    buffer: Vec<u8>,
    has_end_mark: bool,
}

impl Mode2Sector {
    const HEADER_SIZE: usize = 8;

    fn new() -> Self {
        Mode2Sector {
            buffer: Vec::new(),
            has_end_mark: false,
        }
    }

    fn remaining_storage(&self) -> usize {
        let remain = USER_BYTES_PER_MODE2_SECTOR - self.buffer.len();
        if remain < 16 {
            0
        } else {
            USER_BYTES_PER_MODE2_SECTOR - self.buffer.len() - Self::HEADER_SIZE
        }
    }

    fn push_data(&mut self, seqnum: u16, offset: u16, data: &[u8], last: bool) {
        assert!(data.len() & 1 == 0, "Data not word aligned");
        assert!(self.has_end_mark == false, "Sector already closed");

        let afterremain =
            USER_BYTES_PER_MODE2_SECTOR - self.buffer.len() - Self::HEADER_SIZE - data.len();

        let last = afterremain < 16 || last;
        if last {
            self.has_end_mark = true;
        }
        assert!(self.buffer.len() + Self::HEADER_SIZE + data.len() <= USER_BYTES_PER_MODE2_SECTOR);

        let magic: u16 = if last { 0x4242 } else { 0x4243 };
        let length = data.len() as u16;

        /*
        println!(
            "Header seq:{} offset:{} len:{} {}",
            seqnum, offset, length, last
        );
        */

        self.buffer.extend_from_slice(&magic.to_be_bytes());
        self.buffer.extend_from_slice(&seqnum.to_be_bytes());
        self.buffer.extend_from_slice(&offset.to_be_bytes());
        self.buffer.extend_from_slice(&length.to_be_bytes());
        self.buffer.extend_from_slice(data);

        assert!(self.buffer.len() <= USER_BYTES_PER_MODE2_SECTOR);
    }

    fn flush(&mut self, file: &mut File) {
        //println!("Flush");
        assert!(self.has_end_mark);
        // Ensure it is padded, before writing
        assert!(self.buffer.len() <= USER_BYTES_PER_MODE2_SECTOR);
        self.buffer.resize(USER_BYTES_PER_MODE2_SECTOR, 0);

        file.write_all(&self.buffer).unwrap();
        self.buffer.clear();
        self.has_end_mark = false;
    }
}

fn main2() {
    let mut phase_accu = 0;
    let mut last_phase_accu = 0;

    for i in 1..800 {
        phase_accu += 39282;
        if phase_accu >= 0x10000 {
            phase_accu -= 0x10000;

            println!("X");
        } else {
            println!("-");
        }

        last_phase_accu = phase_accu;
    }
}
fn main() {
    let args: Vec<String> = env::args().collect();

    let folder = &args[1];
    println!("Reading from {folder}");
    /*
    let i = 239;
    let path = format!("pics/{i:05}.png");
    let img = ImageReader::open(path).unwrap().decode().unwrap();
    let rle = parse_picture(img);
    std::fs::write("foo.bin", rle).unwrap();
    */

    // Assume 75%, since Level B audio is interleaved with video data
    let sectors_per_second: f32 = 75_f32 * 3_f32 / 4_f32;
    let seconds_per_sector = 1_f32 / sectors_per_second;

    let frames_per_second = 29.97_f32;
    let seconds_per_frame: f32 = 1_f32 / frames_per_second;

    let mut cdi_buffer_level = 0;
    let max_cdi_buffer_level = NUMBER_OF_PCLS * USER_BYTES_PER_MODE2_SECTOR;

    let pal_mode = folder.contains("280");
    assert!(pal_mode || folder.contains("240"));

    let max_frames_in_buffer = if pal_mode { 80 } else { 60 };

    // Preload by half a second before playback
    let mut frametime: f32 = 0_f32;
    //let mut sectortime: f32 = 0_f32;

    let mut outpath = if pal_mode { "MOV280.DAT" } else { "MOV240.DAT" };

    let mut outfile = File::create(outpath).unwrap();
    let mut mode2sec = Mode2Sector::new();

    let mut frame_sizes_in_buffer = VecDeque::new();

    for i in 1..6955 {
        //for i in 1..10 {
        let path = format!("{folder}/{i:05}.png");
        //println!("{}", path);
        let img = ImageReader::open(path).unwrap().decode().unwrap();

        let colormapper = match (pal_mode, i) {
            (true, 1500..1910) => map2shades,
            _ => map3shades,
        };
        let (mut rle, framesize) = parse_picture(img, colormapper);

        let mut offset = 0;
        let mut lines = 0;
        while !rle.is_empty() {
            let remain = mode2sec.remaining_storage();

            // Flush the sector when no more data can be added to it
            if remain == 0 {
                mode2sec.flush(&mut outfile);
                frametime += seconds_per_sector;
            } else {
                let mut to_write: Vec<u8> = Vec::new();

                /*println!(
                    "X {} {} {} {}",
                    rle.is_empty(),
                    to_write.len(),
                    rle.front().unwrap().len(),
                    remain
                );*/

                // Push as many atomic RLE lines into the sector as possible
                while !rle.is_empty() && to_write.len() + rle.front().unwrap().len() < remain {
                    lines += 1;
                    to_write.extend_from_slice(&mut rle.pop_front().unwrap());
                }

                // Pad to full word, so the next header is again word aligned
                if to_write.len() & 1 == 1 {
                    to_write.push(0);
                }

                // There are 2 reasons why we are here. Either
                // 1. rle is empty. No more lines to push. We don't need to flush
                // 2. rle is not empty, but the next line is too big to fit
                // into the remaining space. We must flush
                /*
                if rle.is_empty() {
                    println!("E {} {}", to_write.len(), remain);
                } else {
                    println!(
                        "F {} {} {}",
                        rle.front().unwrap().len(),
                        to_write.len(),
                        remain
                    );
                }
                 */

                if !rle.is_empty() {
                    // The next line will not fit, close it and flush.
                    mode2sec.push_data(i, offset, &to_write, true);
                    cdi_buffer_level += to_write.len();
                    offset = lines;

                    mode2sec.flush(&mut outfile);
                    frametime += seconds_per_sector;
                } else {
                    mode2sec.push_data(i, offset, &to_write, false);
                    cdi_buffer_level += to_write.len();
                    offset = lines;
                }
            }
        }

        frame_sizes_in_buffer.push_back(framesize);

        let remain = mode2sec.remaining_storage();
        if remain == 0 {
            mode2sec.flush(&mut outfile);
            println!("Fill2!");

            frametime += seconds_per_sector;
        }

        //while cdi_buffer_level > max_cdi_buffer_level || frame_sizes_in_buffer.len() > 80 {
        while frame_sizes_in_buffer.len() > max_frames_in_buffer {
            // Add an empty sector
            mode2sec.push_data(i, 0, &[], true);
            mode2sec.flush(&mut outfile);
            frametime += seconds_per_sector;
            println!("Fill1!");

            // Consume frames for display
            while frametime > seconds_per_frame {
                frametime -= seconds_per_frame;
                cdi_buffer_level -= frame_sizes_in_buffer.pop_front().unwrap();
            }
        }

        // Consume frames for display
        while frametime > seconds_per_frame {
            frametime -= seconds_per_frame;
            cdi_buffer_level -= frame_sizes_in_buffer.pop_front().unwrap();
        }

        /*
        if cdi_buffer_level >= max_cdi_buffer_level as f32 {
            mode2sec.flush(&mut outfile);
            cdi_buffer_level -= USER_BYTES_PER_MODE2_SECTOR as f32;
        }
        */

        //println!("{} byte", );
        println!(
            "VBV {} {} {}",
            i,
            cdi_buffer_level,
            frame_sizes_in_buffer.len()
        );
        //assert!(cdi_buffer_level > 0_f32);

        //sum += rle.len() * 2;
    }
    //println!("{} byte", sum);
}
