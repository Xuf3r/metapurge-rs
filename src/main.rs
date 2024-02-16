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
use std::path::Path;
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
    outfilepath: PathBuf

}
impl MTUnitIn {
    fn new(file_name: &String) -> Result<Box<MTUnitIn>, Box<dyn std::error::Error>> {
        let file = File::open(file_name)?;
        let archive = Box::new(ZipArchive::new(file)?);
        let outdocxpath = format!("{file_name}_temp");
        let outdocxpath_2 = outdocxpath.clone();
        let outfile = File::create(&outdocxpath).unwrap();
        Ok(Box::new(MTUnitIn { archive, outfile, outfilepath: Path::new(&outdocxpath_2)}))
    }
}

struct MTUnitOut {
    archive: Box<ZipWriter<File>>,
    outfile: File,
    outfilepath: Path
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
        let outfilepath = mtunit.outfilepath.to_path();
        Ok(Box::new(MTUnitOut{archive: zipout_heap, outfile: outfile, outfilepath:  }))


    }
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
                         itx: Sender<Box<MTUnitIn>>,
                         orx: Receiver<Box<MTUnitOut>>)
    -> Vec<Result<(), String>> {
    let results:Vec<Result<(), String>> = docs.iter().map(|file_name| {
        let MTUnitIn = MTUnitIn::new(file_name).unwrap();
        itx.send(MTUnitIn);

        match orx.try_recv() {
            Ok(mut message) => {
                let outpath = message.outfile.path().to_path_buf(); ;
                message.archive.finish();
                fs::remove_file(file_name).unwrap();
                fs::rename(outpath, file_name).unwrap();
            },
            Err(TryRecvError::Empty) => {

            },
            Err(TryRecvError::Disconnected) => {
                // Channel has been closed
                println!("Channel disconnected");
            }
        }
    Ok(())
    }).collect();
    results

}

// fn iterate_over_inner(mut zip_data: Box<MTUnitIn>, algo: FileOptions) -> Result<(), String> {
//
//     for i in 0..zip_data.archive.len() {
//         let mut file = zip_data.archive.by_index(i).unwrap();
//         let outpath = match file.enclosed_name() {
//             Some(path) => path.to_str().unwrap().to_owned(),
//             None => continue,
//         };
//         let mut content = Vec::with_capacity(1000);
//
//         match outpath.as_str() {
//             to_edit @ xml_consts::CORE_XML => {
//                 let num = file.by_ref().read_to_end(&mut content).expect("Unable to read file");
//                 let corexml = std::str::from_utf8(&content).unwrap();
//                 let replxml = replace_corexml(corexml);
//                 zipout.start_file(to_edit, algo).unwrap();
//                 zipout.write_all(replxml.as_bytes()).unwrap();
//             }
//             to_edit @ xml_consts::RELS_XML => {
//                 file.read_to_end(&mut content);
//                 if let Some(index) = find_rells(&content) {
//                     // println!("{}", index);
//                     let rels = remove_rells(content, index);
//                     zipout.start_file(to_edit, algo).unwrap();
//                     zipout.write_all(&rels);
//                 } else {
//                     zipout.start_file(to_edit, algo).unwrap();
//                     zipout.write_all(content.as_slice());
//                 }
//             }
//             xml_consts::CUSTOM_XML => continue,
//             no_edit => {
//                 // file.read_to_end(&mut content).unwrap();
//                 zipout.start_file(no_edit, algo).unwrap();
//                 // zipout.write_all(content.as_slice()).unwrap();
//                 io::copy(&mut file, &mut zipout);
//             }
//         }
//
//     }
//     otx.send(zipout_heap);
//     zipout.finish().unwrap();
//     Ok(())
// }
fn main() -> () {
    let deflate = FileOptions::default();
    let start_time = Instant::now();


    let filter_vec = vec![OsStr::new("docx")];

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

    let irx = Arc::new(Mutex::new(irx));
    let otx = Arc::new(Mutex::new(otx));

    // Clone the Arc for each thread
    let irx_clone = Arc::clone(&irx);
    let otx_clone = Arc::clone(&otx);


    let io_thread = thread::spawn(move || {


        iterate_over_archives(filtered, deflate, itx, orx);

        });
    let compute_thread = thread::spawn(move || {
        let mut counter = 0;
        while counter <= input_len_for_io {
            let ram_archive = irx.lock().unwrap().recv().unwrap();
            let out_archive = MTUnitOut::new(ram_archive, deflate);
            otx.send(out_archive);
            counter += 1;
        }

    });

}




