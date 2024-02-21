#![feature(slice_pattern)]
#![feature(const_trait_impl)]
extern crate core;

mod consts;

mod xml_consts;

use core::slice::SlicePattern;
use std::ffi::OsStr;
use std::fmt::Error;
use std::{fs, io, ptr, thread};
use std::fs::File;
use std::io::{BufRead, Cursor, Read, SeekFrom, Write};
use std::ops::Deref;
use walkdir::{DirEntry, WalkDir};
use std::path::{Path, PathBuf};
use std::str::from_utf8;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::time::Instant;
use crate::consts::*;
use regex::Regex;
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};
use lazy_static::lazy_static;


lazy_static! {
    static ref RE1: Regex = Regex::new(r"<dc:title.*<dcterms:created").unwrap();
}
lazy_static! {
    static ref RE2: Regex = Regex::new(r"<cp:category.*</cp:coreProperties>").unwrap();
}
lazy_static! {
    static ref RE3: Regex = Regex::new(r#"<Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/custom-properties" Target="docProps/custom.xml"/>"#).unwrap();
}


const TARGET: &[u8] = br#"<Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/custom-properties" Target="docProps/custom.xml"/></Relationships>"#;
const REPLACEMENT: &[u8; 16] = br#"</Relationships>"#;

struct MTUnitIn {
    archive: Box<ZipArchive<File>>,
    outfile: File,
    oldfilepath: String,
    outfilepath: PathBuf

}
impl MTUnitIn {
    fn new(file_name: &String) -> Result<Box<MTUnitIn>, Box<dyn std::error::Error>> {
        let file = File::open(file_name)?;
        let archive = Box::new(ZipArchive::new(file)?);
        let outdocxpath = format!("{file_name}_temp");
        let outdocxpath_2 = outdocxpath.clone();
        let outfile = File::create(&outdocxpath).unwrap();

        Ok(Box::new(MTUnitIn {
            archive,
            outfile,
            oldfilepath: file_name.clone(),
            outfilepath: PathBuf::from(outdocxpath_2)}))
    }
}

struct MTUnitOut {
    archive: Box<ZipWriter<File>>,
    oldfilepath: String,
    outfilepath: PathBuf
}
impl MTUnitOut {
    fn new(mut mtunit: Box<MTUnitIn>,
           algo: FileOptions)
        -> Result<Box<MTUnitOut>, Box<dyn std::error::Error>> {
        let outfile = mtunit.outfile;
        let mut zipout_heap = Box::new(ZipWriter::new(outfile));
        let mut zipout = &mut *zipout_heap;

        for i in 0..mtunit.archive.len() {
            let mut file = mtunit.archive.by_index(i)?;
            let outpath = match file.enclosed_name() {
                Some(path) => path.to_str().unwrap().to_owned(), //we unwrap because there's no possible way for path to be None. If it's none we're better off panicking.
                None => continue,
            };
            let mut content = Vec::with_capacity(1000);

            match outpath.as_str() {
                to_edit @ xml_consts::CORE_XML => {
                    let read_result = file.by_ref().read_to_end(&mut content)?;
                    let corexml = std::str::from_utf8(&content)?;
                    let replxml = replace_corexml(corexml);
                    zipout.start_file(to_edit, algo)?;
                    zipout.write_all(replxml.as_bytes())?;
                }
                to_edit @ xml_consts::RELS_XML => {
                    file.read_to_end(&mut content);
                    if let Some(index) = find_rells(&content) {
                        // println!("{}", index);
                        let rels = remove_rells(content, index);
                        zipout.start_file(to_edit, algo)?;
                        zipout.write_all(&rels);
                    } else {
                        zipout.start_file(to_edit, algo)?;
                        zipout.write_all(content.as_slice());
                    }
                }
                xml_consts::CUSTOM_XML => continue,
                no_edit => {
                    // file.read_to_end(&mut content).unwrap();
                    zipout.start_file(no_edit, algo)?;
                    // zipout.write_all(content.as_slice()).unwrap();
                    io::copy(&mut file, &mut zipout);
                }
            }

        }
        let oldfilpath = mtunit.oldfilepath;
        let oufilepath = mtunit.outfilepath;
        Ok(Box::new(MTUnitOut{
            archive: zipout_heap,
            oldfilepath: oldfilpath,
            outfilepath: oufilepath  }))


    }
}

enum OutMessage {
    Data(Box<MTUnitOut>),
    ComputeEnd
}

enum InMessage {
    Data(Box<MTUnitIn>),
    ComputeEnd,
}
fn is_in(item: &DirEntry, filter: &Vec<&OsStr>) -> Result<Option<String>, std::io::Error> {

    let binding = item;
    let extension = binding.path().extension();
    if extension.is_none() {
        Ok(None)
    }
    else if filter.contains(&extension.unwrap()) {
        Ok(Some(item.path().to_string_lossy().into_owned()))
    }
    else {
        Ok(None)
    }

}


fn replace_corexml(data: &str) -> String {

    RE2.
        replace_all(&RE1.replace_all(data, CoreXmlStr::TEXT_TEMPLATE_1), CoreXmlStr::TEXT_TEMPLATE_2)
        .to_string()
}


fn find_rells(data: &Vec<u8>) -> Option<usize> {


    data.windows(TARGET.len()).position(|subslice| subslice == TARGET)
}

fn remove_rells(mut data: Vec<u8>, index: usize) -> Vec<u8> {

    data.truncate(index);
    data.extend_from_slice(REPLACEMENT);
    data

}


fn iterate_over_archives(docs: Vec<String>,
                         algo: FileOptions,
                         itx: Sender<InMessage>,
                         orx: Receiver<OutMessage>) {
    for file_name in docs
    {
        let MTUnitIn = MTUnitIn::new(&file_name).unwrap();
        itx.send(InMessage::Data(MTUnitIn));

        match orx.try_recv() {
            Ok(mut message) => {
                match message {
                    OutMessage::Data(mut data) => {
                        let outpath = data.outfilepath;
                        let oldpath = data.oldfilepath;
                        data.archive.finish();
                        fs::remove_file(&oldpath).unwrap();
                        fs::rename(&outpath, &oldpath).unwrap();
                    },
                    OutMessage::ComputeEnd => {
                        return
                    }
                }
            },
            Err(TryRecvError::Empty) => continue,
            Err(TryRecvError::Disconnected) => break,
        }
    };
    itx.send(InMessage::ComputeEnd);
    while let Ok(message) = orx.recv() {
        match message {
            OutMessage::Data(mut data) => {
                let outpath = data.outfilepath;
                // println!("{:?}", outpath);
                let oldpath = data.oldfilepath;
                // println!("{:?}", oldpath);
                data.archive.finish();
                fs::remove_file(&oldpath).unwrap();
                fs::rename(&outpath, &oldpath).unwrap();
            },
            OutMessage::ComputeEnd => {
                return
            }
        }
    }
}

fn main() -> () {
    let deflate = FileOptions::default();
    // let start_time = Instant::now();


    let filter_vec = vec![OsStr::new("docx"),OsStr::new("xlsx")];

    let (oks, errs): (Vec<_>, Vec<_>) = WalkDir::new("C:\\Users\\stp\\ferrprojs\\test0")
        .into_iter()
        .filter_map(Result::ok)
        .map(|path| is_in(&path, &filter_vec))
        .partition(Result::is_ok);

    let filtered: Vec<String> = oks.into_iter()
        .filter_map(|result| result.ok()) // Extract the `Ok` values
        .flatten() // Flatten the `Vec<Option<String>>` into a `Vec<String>`
        .collect();

    // println!("{:?}", errs.into_iter());
    // println!("{:?}", &filtered);
    if filtered.len() == 0 {
        return;
    }

    let input_len_for_io = filtered.len();
    let input_len_for_compute = input_len_for_io.clone();


    let (itx, irx) = channel();
    let (otx, orx) = channel();



    let io_thread = thread::spawn(move || {


        iterate_over_archives(filtered, deflate, itx, orx);

        });


    let compute_thread = thread::spawn(move || {
        let mut counter = 0;
        while counter <= input_len_for_io {
            let ram_archive = match irx.recv() {
                Ok(message) => {
                    match message {
                        InMessage::Data(data) => data,
                        InMessage::ComputeEnd => {
                            otx.send(OutMessage::ComputeEnd);
                            break
                        },
                    }
                },
                Err(error) => {println!("{:?}", error); panic!()},
            };

            let out_archive = MTUnitOut::new(ram_archive, deflate);
            let out_archive = match out_archive {
                Ok(content) =>{content},
                Err(err) => {println!("{:?}",err); panic!()}
            };
            otx.send(OutMessage::Data(out_archive));
            counter += 1;
        }

    });
    io_thread.join().unwrap();
    compute_thread.join().unwrap();
}




