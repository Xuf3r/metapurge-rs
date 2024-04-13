use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use crate::errors::error::PurgeErr;
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


fn get_app_ranges(src: &Vec<u8>) -> Vec<Range> {

    let mut has_SOI: bool = false;

    let mut app_indices: Vec<Range> = Vec::new();


    let mut file_iter = src.windows(2).enumerate();
    while let Some((index,twobytes)) = file_iter.next() {
        match twobytes {
            [0xff, 0xe0..=0xef] => {
                if let Some(_) = file_iter.next() {
                    if let Some (real_win) = file_iter.next() {
                        let length = u16::from_be_bytes([real_win.1[0], real_win.1[1]]);
                        let app_index_start: u16 = index as u16;
                        let app_index_end = app_index_start + length;

                        if let Some(end_i) = app_indices.last() {
                            if app_index_end < end_i.end {
                                continue
                            }
                        }

                        let app_range = Range::new(app_index_start, app_index_end);
                        // println!("{:?} with app {:#02x},", app_range.clone(), app);
                        app_indices.push(app_range);
                    }
                }
            },
            [0xff, 0xd8] => {
                if !has_SOI{
                    has_SOI = true;
                    // println!("found SOI at {:?}", index);
                    continue
                }
            },
            [0xff, 0xda] => {
                let mut SOS_walker = file_iter.clone();


                let mut validity_buf: Vec<u8> = Vec::new();

                let _ = SOS_walker.next();

                for _ in 0..=4 {
                    validity_buf.push(SOS_walker.next().unwrap().1[0])
                }

                let ns = validity_buf[2];
                if ns & 0b0000_0011 != ns {
                    continue
                }

                let t_d_a_j =  validity_buf[4];
                if t_d_a_j & 0b0011_0011 != t_d_a_j {
                    continue
                }
                // println!("found SOS at {:?}", index);
            },
            [0xff, 0xc0..=0xc3] => {
                let mut SOF_walker = file_iter.clone();

                let mut validity_buf: Vec<u8> = Vec::new();
                let _ = SOF_walker.next();

                for _ in 0..=10 {
                    validity_buf.push(SOF_walker.next().unwrap().1[0])
                }

                let length = &validity_buf[0..=1];

                if let h_n_v_sampl = &validity_buf[9].clone() {
                    if h_n_v_sampl & 0b0011_0011 != *h_n_v_sampl {
                        continue
                    }

                }

                if let quant_tab_sel = &validity_buf[10].clone() {
                    if quant_tab_sel & 0b0000_0011 != *quant_tab_sel {
                        continue
                    }
                }

                // let sof_len = u16::from_be_bytes([length[0], length[1]]);

                // let _ = file_iter.nth(sof_len as usize + index);

                // println!("found SOF at {:?}", index);
                continue
            },
            [0xff, 0xdb] => {
                let mut DQT_walker = file_iter.clone();


                let mut validity_buf: Vec<u8> = Vec::new();

                let _ = DQT_walker.next();

                for _ in 0..=4 {
                    validity_buf.push(DQT_walker.next().unwrap().1[0])
                }

                let p_t_q = validity_buf[2];
                if p_t_q & 0b0001_0011 != p_t_q {
                    continue
                }
                // println!("found DQT  at {index}");
            },
            [0xff, 0xc4] => {
                let mut DHT_walker = file_iter.clone();


                let mut validity_buf: Vec<u8> = Vec::new();

                let _ = DHT_walker.next();

                for _ in 0..=4 {
                    validity_buf.push(DHT_walker.next().unwrap().1[0])
                }

                let p_t_q = validity_buf[2];
                if p_t_q & 0b0001_0011 != p_t_q {
                    continue
                }
                // println!("found DHT  at {index}");
            },
            [0xff, 0xd9] => {

                // println!("found ffd9 at {:?}", index);
                continue},
            _ => {} ,
        }
    }

    app_indices
}

pub(crate) struct Jpg {
    paths: DataPaths,
    data: Vec<u8>
}

impl Jpg {
    pub(crate) fn new(paths: DataPaths) -> Box<Jpg> {
        Box::from(Jpg {
            paths: paths,
            data: Vec::new()
        })
    }
}

impl Jpg {
    pub(crate) fn load(mut self: Box<Self>) -> Result<Box<Self>, PurgeErr> {
        let mut file = fs::File::open(self.paths.old())?;
    file.read_to_end(&mut self.data)?;

        Ok(self)


    }

    pub(crate)  fn process(mut self: Box<Self>) -> Result<Box<Self>, PurgeErr> {
        let ranges = get_app_ranges(&self.data);
        self.data = dont_take_ranges(&self.data, ranges);

        Ok(self)
    }

    pub(crate)  fn save(self) -> Result<(), PurgeErr> {
        let mut temp = File::create(self.paths.temp())?;
        temp.write_all(self.data.as_slice())?;
        // if let Err(hr) = std::fs::remove_file(&self.paths.old()) {
        //     std::fs::remove_file(&self.paths.temp());
        //     return Err(PurgeErr::from(hr))
        // };
        // rename() already removes the file.

        if let Err(hr) = std::fs::rename(&self.paths.temp(), &self.paths.old()) {
            std::fs::remove_file(&self.paths.temp());
            //     return Err(PurgeErr::from(hr))
        }
        // We still have to remove the temp it remove() fails
        Ok(())
    }

    pub(crate) fn file_name(&self) -> String {
        self.paths.old_owned()
    }
}