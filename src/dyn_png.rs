use std::ffi::OsString;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use crate::errors::error::{ExifStructureErr, PurgeErr};
use crate::traits::container::{DataPaths, Purgable};


#[derive(Debug, Copy, Clone)]
struct Range {
    start: u16,
    end: u16
}

impl Range {
    fn new(start: u16, end: u16) -> Self {
        Range {
            start,
            end,
        }
    }
}

fn dont_take_ranges(src: &Vec<u8>, ranges: Vec<Range>) -> Vec<u8> {

    let mut src_iter = src.iter().enumerate();
    let mut rg_iter = ranges.iter();

    let mut clean_buf: Vec<u8> = Vec::new();

    while let Some(range) = rg_iter.next() {
        let start = range.start;
        let end = range.end;
        while let Some((index, data)) = src_iter.next() {
            if index < start as usize {
                clean_buf.push(data.clone())
            } else if index == end as usize {
                {
                    break
                }
            }
        }

    };
    while let Some ((_, data)) = src_iter.next() {
        clean_buf.push(data.clone())
    };

    clean_buf
}

fn get_ancil_ranges(src: &Vec<u8>) -> Result<Vec<Range>,PurgeErr> {

    let mut anxil_ranges: Vec<Range> = Vec::new();
    if let Some(data) = src.get(..=7) {
        if data != [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A] {
            return Err(PurgeErr::from(ExifStructureErr::new("not png")))
        }
    }

    let mut vec_iter = src.windows(8).enumerate();

    if let None =  vec_iter.nth(8) {
        return Err(PurgeErr::from(ExifStructureErr::new("empty png")))
    }

    while let Some((index, bytes)) = vec_iter.next() {
        match bytes {
            chunk @ [_,_,_,_,116, 69, 88, 116] // tEXt
            | chunk @ [_,_,_,_,0x69, 0x54, 0x58, 0x74] //iTXt
            => {
                let length = chunk.get(0..=3).unwrap();

                let d_len = u32::from_be_bytes(
                    [length[0],
                        length[1],
                        length[2],
                        length[3]]) as usize;

                // println!("d_len is: {:?}", d_len);

                let start = index;

                // 11 accounts for length bytes, type bytes and CNC.
                // Unfortunately I do NOT know why it's 11 instead of supposed 12.
                let end = index
                    + d_len
                    + 11;

                // println!("tEXt found at: [{} - {}]", &start, &end);
                anxil_ranges.push(Range::new(start as u16, end as u16))
            },
            _ => {}
        }

    }

    Ok(anxil_ranges)
}


pub(crate) struct Png {
    pub(crate) paths: DataPaths,
    data: Vec<u8>
}


impl Png {
    pub(crate) fn new(paths: DataPaths) -> Box<Self> {
        Box::from(Png {
            paths: paths,
            data: Vec::new()
        })
    }
}
impl Png {
    pub(crate)  fn load(&mut self) -> Result<(), PurgeErr> {

        let mut file = fs::File::open(self.paths.old())?;
        file.read_to_end(&mut self.data)?;

        Ok(())

        }



    pub(crate) fn process(&mut self) -> Result<(), PurgeErr> {

        let ancil_ranges = get_ancil_ranges(&self.data)?;
        self.data = dont_take_ranges(&self.data, ancil_ranges);

        Ok(())
    }

    pub(crate) fn save(&mut self) -> Result<(), PurgeErr> {

        let mut temp = File::create(self.paths.temp())?;
        temp.write_all(self.data.as_slice())?;
        // if let Err(hr) = std::fs::remove_file(&self.paths.old()) {
        //     std::fs::remove_file(&self.paths.temp());
        //     return Err(PurgeErr::from(hr))
        // };
        // rename() already removes the file.

        if let Err(hr) = std::fs::rename(&self.paths.temp(), &self.paths.old()) {
            std::fs::remove_file(&self.paths.temp());
            return Err(PurgeErr::from(hr))
        }
        // We still have to remove the temp it remove() fails
        Ok(())
    }

    pub(crate)  fn inner_file_name(&self) -> String {
        self.paths.old_owned()
    }
}
