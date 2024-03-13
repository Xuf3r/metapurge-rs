#![feature(slice_pattern)]
#![feature(const_trait_impl)]
extern crate core;

mod mso_x_core_xml_templates; // it doesn't seem to be used at all

mod mso_x_file_name_consts;
mod pdf;
mod load_process_write;
mod mso_x;
mod traits;
mod errors;

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
use std::sync::{Arc, Mutex, Condvar};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::time::{Duration, Instant};
use crate::mso_x_core_xml_templates::*; // this doesn't seem to be used at all
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
            let mut content = Vec::with_capacity(1024);

            match outpath.as_str() {
                to_edit @ mso_x_file_name_consts::CORE_XML => {
                    let read_result = file.by_ref().read_to_end(&mut content)?;
                    let corexml = std::str::from_utf8(&content)?;
                    let replxml = replace_corexml(corexml);
                    zipout.start_file(to_edit, algo)?;
                    zipout.write_all(replxml.as_bytes())?;
                }
                to_edit @ mso_x_file_name_consts::RELS_XML => {
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
                mso_x_file_name_consts::CUSTOM_XML => continue,
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
                         irx: Arc<Mutex<Receiver<InMessage>>>,
                         otx: Sender<OutMessage>,
                         orx: Receiver<OutMessage>,
                         lock: &Mutex<bool>,
                         cvar: &Condvar) {

    let mut started = lock.lock().unwrap();

    for file_name in docs
    {
        if let Ok(MTUnitIn) = MTUnitIn::new(&file_name) {
            itx.send(InMessage::Data(MTUnitIn));
        }
        else {
            println!("what happened during MTUnitIn creation?");
        };
    };
    itx.send(InMessage::ComputeEnd);
    itx.send(InMessage::ComputeEnd);



    loop {
        let mut irx_locked = match irx.lock() {
            Ok(lock) => lock,
            Err(error) => {
                println!("Failed to acquire lock: {:?}", error);
                panic!()
            }
        };

        //checked the lock

        match irx_locked.try_recv() {
            Ok(message) => {
                match message {
                    InMessage::Data(data) => {
                        drop(irx_locked);
                        let out_archive = match MTUnitOut::new(data, algo) {
                            Ok(content) => content,
                            Err(err) => {
                                println!("{:?}", err);
                                panic!()
                            },
                        };
                        otx.send(OutMessage::Data(out_archive));
                    },
                    InMessage::ComputeEnd => {
                        // otx.send(OutMessage::ComputeEnd).unwrap_or_else(|err| {
                        //     println!("Failed to send message: {:?}", err);
                        //
                        // });
                        drop(irx_locked);
                        break
                    },
                }
            },
            Err(TryRecvError::Empty) => {
                break
                // println!("huh? why empty in the i/o-consumer thread?"); // No message available yet, exit the reception
            },
            Err(TryRecvError::Disconnected) => {
                break
                // println!("huh? why disconnected in the i/o-consumer thread?");// No more messages available, only exit the reception since we might have writes down the line
            },
        };



        if let Ok(message) = orx.try_recv() {
            match message {
                OutMessage::Data(mut data) => {
                    let outpath = data.outfilepath;
                    // println!("{:?}", outpath);
                    let oldpath = data.oldfilepath;
                    // println!("{:?}", oldpath);
                    data.archive.finish();
                    fs::remove_file(&oldpath).unwrap();
                    fs::rename(&outpath, &oldpath).unwrap();
                }
                OutMessage::ComputeEnd => {
                    break
                }
            }

        }else {
            // If try_recv() returns an Err, skip the rest of the loop iteration
            continue;
        }
    };

    // allowing the compute thread to send the finish message

    *started = true; // Set the condition to true

    cvar.notify_one(); // Notify one waiting thread

    drop(started);


    loop {
        if let Ok(message) = orx.recv() {
            match message {
                OutMessage::Data(mut data) => {
                    let outpath = data.outfilepath;
                    // println!("{:?}", outpath);
                    let oldpath = data.oldfilepath;
                    // println!("{:?}", oldpath);
                    data.archive.finish();
                    fs::remove_file(&oldpath).unwrap();
                    fs::rename(&outpath, &oldpath).unwrap();
                }
                OutMessage::ComputeEnd => {
                    break
                }
            }
        }
    };

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
    let otx_io = otx.clone();
    let irx_arc = Arc::new(Mutex::new(irx));

    let irx_arc_c = Arc::clone(&irx_arc);
    let irx_arc_io = Arc::clone(&irx_arc);

    let pair = Arc::new((Mutex::new(false), Condvar::new()));
    let pair2 = Arc::clone(&pair);

    let io_thread = thread::spawn(move || {
        let (lock, cvar) = &*pair;

        iterate_over_archives(filtered, deflate, itx, irx_arc_io, otx_io, orx, lock, cvar);

        });


    let compute_thread = thread::spawn(move || {
        let (lock, cvar) = &*pair2;
        thread::sleep(Duration::new(0, 50*100));
        loop {
            let mut irx_locked = irx_arc_c.lock().unwrap();
            match irx_locked.recv() {
                Ok(message) => {
                    match message {
                        InMessage::Data(data) => {
                            drop(irx_locked);
                            if let Ok(edited_data) = MTUnitOut::new(data, deflate) {
                                otx.send(OutMessage::Data(edited_data));
                            } else {
                                panic!("for some reason creating MTUnitOut in the compute thread failed!");
                            }
                        },
                        InMessage::ComputeEnd => {
                            drop(irx_locked);
                            //waiting for the signal that we last real data has been sent to the channel


                            let mut started = lock.lock().unwrap();

                            while !*started {
                                started = cvar.wait(started).unwrap();
                            }


                            otx.send(OutMessage::ComputeEnd);
                             //probably can drop earlier
                            // since input queue is empty or drop implicitly but whatever
                            break
                        },
                    }
                },
                Err(_) => {
                    println!("in compute thread - probably itx closed");
                    break// it's not possible for the itx to not exist when compute thread is alive
                    // this branch is supposed to be unreachable
                },
            };

        }

    });
    io_thread.join().unwrap();
    compute_thread.join().unwrap();
}




