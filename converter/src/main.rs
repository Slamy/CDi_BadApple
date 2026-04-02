#![feature(iter_array_chunks)]

use std::{collections::VecDeque, fs::File, io::Write};

use image::{DynamicImage, ImageReader};

#[derive(Clone)]
struct Rle {
    index: u8,
    cnt: u32,
}

fn line_to_rle(line: &[u8]) -> Vec<Rle> {
    let mut rle: Option<Rle> = None;
    let mut out = Vec::<Rle>::new();

    let clut4conv = line
        .iter()
        .map(|p| if *p > 128 { 1 } else { 0 })
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

fn parse_picture(img: DynamicImage) -> (VecDeque<Vec<u8>>, usize) {
    let img = img.into_luma8();

    assert!(img.width() == 768);
    let mut enclines = VecDeque::new();

    let lines = img.chunks(img.width() as usize);
    assert!(img.height() as usize == lines.len());
    let mut framesize = 0;
    //println!("{}", lines.len());
    for line in lines {
        let mut enclineout = Vec::<u8>::new();

        let raw_rle = line_to_rle(line);

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
        self.buffer.extend_from_slice(&magic.to_be_bytes());
        self.buffer.extend_from_slice(&seqnum.to_be_bytes());
        self.buffer.extend_from_slice(&offset.to_be_bytes());
        self.buffer.extend_from_slice(&length.to_be_bytes());
        self.buffer.extend_from_slice(data);

        assert!(self.buffer.len() <= USER_BYTES_PER_MODE2_SECTOR);
    }

    fn flush(&mut self, file: &mut File) {
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
    println!("Hello, world!");

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

    // Preload by half a second before playback
    let mut frametime: f32 = 0_f32;
    //let mut sectortime: f32 = 0_f32;

    let mut outfile = File::create("MOVIE.DAT").unwrap();
    let mut mode2sec = Mode2Sector::new();

    let mut frame_sizes_in_buffer = VecDeque::new();

    for i in 1..6955 {
        //for i in 1..10 {
        let path = format!("pics/{i:05}.png");
        //println!("{}", path);
        let img = ImageReader::open(path).unwrap().decode().unwrap();

        let (mut rle, framesize) = parse_picture(img);

        let mut offset = 0;
        let mut lines = 0;
        while !rle.is_empty() {
            let remain = mode2sec.remaining_storage();

            if remain == 0 {
                mode2sec.flush(&mut outfile);
                frametime += seconds_per_sector;
            } else {
                let mut to_write: Vec<u8> = Vec::new();

                while !rle.is_empty() && to_write.len() + rle.front().unwrap().len() < remain {
                    lines += 1;
                    to_write.extend_from_slice(&mut rle.pop_front().unwrap());
                }

                // Pad to full word
                if to_write.len() & 1 == 1 {
                    to_write.push(0);
                }

                mode2sec.push_data(i, offset, &to_write, false);
                cdi_buffer_level += to_write.len();
                offset = lines;
            }
        }

        frame_sizes_in_buffer.push_back(framesize);

        let remain = mode2sec.remaining_storage();
        if remain == 0 {
            mode2sec.flush(&mut outfile);
            frametime += seconds_per_sector;
        }

        //while cdi_buffer_level > max_cdi_buffer_level || frame_sizes_in_buffer.len() > 80 {
        while frame_sizes_in_buffer.len() > 80 {
            // Add an empty sector
            mode2sec.push_data(i, 0, &[], true);
            mode2sec.flush(&mut outfile);
            frametime += seconds_per_sector;
            println!("Fill!");

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
