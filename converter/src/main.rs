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
        7
    } else if *p > 60 {
        2
    } else {
        0
    }
}

fn map7shades(p: &u8) -> u8 {
    if *p > 236 {
        7
    } else if *p > 200 {
        6
    } else if *p > 163 {
        5
    } else if *p > 127 {
        4
    } else if *p > 91 {
        3
    } else if *p > 54 {
        2
    } else if *p > 18 {
        1
    } else {
        0
    }
}

fn map2shades(p: &u8) -> u8 {
    if *p > 100 { 7 } else { 0 }
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

    fn push_data(
        &mut self,
        seqnum: u16,
        offset: u16,
        data: &[u8],
        last_header_of_cd_sector: bool,
        frame_complete: bool,
    ) {
        //println!("{seqnum} {offset} {last_header_of_cd_sector} {frame_complete}");
        let seqnum = seqnum | if frame_complete { 0x8000 } else { 0 };

        assert!(data.len() & 1 == 0, "Data not word aligned");
        assert!(!self.has_end_mark, "Sector already closed");

        let afterremain =
            USER_BYTES_PER_MODE2_SECTOR - self.buffer.len() - Self::HEADER_SIZE - data.len();

        let last_header_of_cd_sector = afterremain < 16 || last_header_of_cd_sector;
        if last_header_of_cd_sector {
            self.has_end_mark = true;
        }
        assert!(self.buffer.len() + Self::HEADER_SIZE + data.len() <= USER_BYTES_PER_MODE2_SECTOR);

        let magic: u16 = if last_header_of_cd_sector {
            0x4242
        } else {
            0x4243
        };
        let length = data.len() as u16;

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

struct RleStream {
    outfile: File,
    mode2sec: Mode2Sector,
    frame_sizes_in_buffer: VecDeque<usize>,
    frametime: f32,
    max_frames_in_buffer: usize,
}

// Assume 75%, since Level B audio is interleaved with video data
const SECTORS_PER_SECOND: f32 = 75_f32 * 3_f32 / 4_f32;
const SECONDS_PER_SECTOR: f32 = 1_f32 / SECTORS_PER_SECOND;

const FRAMES_PER_SECOND: f32 = 29.97_f32;
const SECONDS_PER_FRAME: f32 = 1_f32 / FRAMES_PER_SECOND;

impl RleStream {
    fn new(pal_mode: bool, outpath: &str) -> Self {
        let outfile = File::create(outpath).unwrap();
        let mode2sec = Mode2Sector::new();
        let frame_sizes_in_buffer = VecDeque::new();
        // Preload by half a second before playback
        let frametime: f32 = 0_f32;

        // PCL fullness during playback on CD-i
        // 70 frames at 50 Hz -> 20 -> 133
        // 50 frames at 60 Hz -> 21 -> 100
        // 55 frames at 60 Hz -> 24 -> 104
        // 75 frames at 50 Hz -> 21 -> 134
        let max_frames_in_buffer = if pal_mode { 75 } else { 55 };

        Self {
            outfile,
            mode2sec,
            frame_sizes_in_buffer,
            frametime,
            max_frames_in_buffer,
        }
    }

    // Never more than 75 sectors. We will destroy the previous PCL buffers that are still in use!
    fn push_empty_sectors(&mut self, num: usize) {
        for _ in 0..num {
            self.mode2sec.push_data(0, 0, &[], true, false);
            self.mode2sec.flush(&mut self.outfile);
        }
    }
    fn encode_frame(&mut self, path: &str, seqnum: u16, colormapper: ColorMapperFn) {
        let img = ImageReader::open(path).unwrap().decode().unwrap();

        let (mut rle, framesize) = parse_picture(img, colormapper);

        let mut offset = 0;
        let mut lines = 0;
        while !rle.is_empty() {
            let remain = self.mode2sec.remaining_storage();

            // Flush the sector when no more data can be added to it
            if remain == 0 {
                self.mode2sec.flush(&mut self.outfile);
                self.frametime += SECONDS_PER_SECTOR;
            } else {
                let mut to_write: Vec<u8> = Vec::new();

                // Push as many atomic RLE lines into the sector as possible
                while !rle.is_empty() && to_write.len() + rle.front().unwrap().len() < remain {
                    lines += 1;
                    to_write.extend_from_slice(&rle.pop_front().unwrap());
                }

                // Pad to full word, so the next header is again word aligned
                if to_write.len() & 1 == 1 {
                    to_write.push(0);
                }

                // There are 2 reasons why we are here. Either
                // 1. rle is empty. No more lines to push. We don't need to flush
                // 2. rle is not empty, but the next line is too big to fit
                // into the remaining space. We must flush

                if !rle.is_empty() {
                    // The next line will not fit, close it and flush.
                    self.mode2sec
                        .push_data(seqnum, offset, &to_write, true, false);
                    offset = lines;

                    self.mode2sec.flush(&mut self.outfile);
                    self.frametime += SECONDS_PER_SECTOR;
                } else {
                    assert!(to_write.len() > 0);

                    self.mode2sec
                        .push_data(seqnum, offset, &to_write, false, true);
                    offset = lines;
                }
            }
        }

        self.frame_sizes_in_buffer.push_back(framesize);

        let remain = self.mode2sec.remaining_storage();
        if remain == 0 {
            self.mode2sec.flush(&mut self.outfile);
            println!("Fill2!");

            self.frametime += SECONDS_PER_SECTOR;
        }

        //while cdi_buffer_level > max_cdi_buffer_level {
        while self.frame_sizes_in_buffer.len() > self.max_frames_in_buffer {
            // Add an empty sector
            self.mode2sec.push_data(seqnum, 0, &[], true, false);
            self.mode2sec.flush(&mut self.outfile);
            self.frametime += SECONDS_PER_SECTOR;
            println!("Fill1!");

            // Consume frames for display
            while self.frametime > SECONDS_PER_FRAME {
                self.frametime -= SECONDS_PER_FRAME;
                self.frame_sizes_in_buffer.pop_front().unwrap();
            }
        }

        // Consume frames for display
        while self.frametime > SECONDS_PER_FRAME {
            self.frametime -= SECONDS_PER_FRAME;
            self.frame_sizes_in_buffer.pop_front().unwrap();
        }

        println!("VBV {} {}", seqnum, self.frame_sizes_in_buffer.len());
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let folder = &args[1];
    println!("Reading from {folder}");

    let pal_mode = folder.contains("280");
    assert!(pal_mode || folder.contains("240"));

    let outpath = if pal_mode { "MOV280.DAT" } else { "MOV240.DAT" };

    let mut rlestream = RleStream::new(pal_mode, outpath);

    /* Encode all frames until the credits, which we handle special */
    for seqnum in 1..6530 {
        let path = format!("{folder}/{seqnum:05}.png");

        let colormapper = match (pal_mode, seqnum) {
            (true, 1500..1910) => map2shades,
            _ => map3shades,
        };

        rlestream.encode_frame(&path, seqnum, colormapper);
    }

    let colormapper = map7shades;
    let vertical_res_str = if pal_mode { "280" } else { "240" };

    let mut seqnum = 6530;

    let path = format!("manualpic/credits0_{vertical_res_str}.png");
    println!("Open {path}");
    rlestream.encode_frame(&path, seqnum, colormapper);
    seqnum = seqnum + 1;
    rlestream.push_empty_sectors(75);
    rlestream.encode_frame(&path, seqnum, colormapper);
    seqnum = seqnum + 1;
    rlestream.push_empty_sectors(75);
    rlestream.encode_frame(&path, seqnum, colormapper);
    seqnum = seqnum + 1;
    rlestream.push_empty_sectors(75);
    rlestream.encode_frame(&path, seqnum, colormapper);
    seqnum = seqnum + 1;
    rlestream.push_empty_sectors(75);

    let path = format!("manualpic/credits1_{vertical_res_str}.png");
    println!("Open {path}");
    rlestream.encode_frame(&path, seqnum, colormapper);
    rlestream.push_empty_sectors(10);
}
