#![feature(slice_pattern)]
#![feature(const_trait_impl)]
extern crate core;

use mso_x::mso_x_core_xml_templates; // it doesn't seem to be used at all


mod pdf;
mod mso_x;
mod traits;
mod errors;

use core::slice::SlicePattern;
use std::ffi::OsStr;
use std::fmt::Error;
use std::{fs, io, ptr, thread};
use std::collections::HashSet;
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
use crate::errors::error::{PurgeErr, ToUser, UISideErr};
use crate::traits::load_process_write::LoadFs;
use crate::traits::container;
use crate::traits::container::Container as Cont;

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

enum OutMessage {
    Data(Cont),
    ComputeEnd
}

enum InMessage {
    Data(Cont),
    ComputeEnd,
}
fn is_in(item: &DirEntry, filter: &Vec<&OsStr>) -> Option<String> {

    // let binding = item;
    let extension = item.path().extension();
    if extension.is_none() {
        None
    }
    else if filter.contains(&extension.unwrap()) {
        Some(item.path().to_string_lossy().into_owned())
    }
    else {
        None
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
                         cvar: &Condvar) -> Vec<UISideErr> {
    let mut err_vec:Vec<UISideErr> = vec![];
    let mut started = lock.lock().unwrap();

    for file_name in docs
    {
        if let Some(cont) = Cont::new(&file_name) {
            match cont.load() {
                Ok(loaded) => {

                    itx.send(InMessage::Data(loaded));
                },
                Err(err) => {
                    err_vec.push(err.to_user(file_name.clone()));
                    continue
                },
            }
        }
        else {
            println!("?? how exactly did not supported extension snuck up in here? it's:" );
            continue
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
            Ok(message) =>  {
                match message {
                    InMessage::Data(data) => {
                        drop(irx_locked);
                        let context = data.getpath();
                         match data.process() {
                            Ok(content) => {

                                let _ = otx.send(OutMessage::Data(content));
                            },
                            Err(err) => {
                               let _ = err_vec.push(err.to_user(context));
                                // panic!() why is it here??

                            },
                        };

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
                    let context = data.getpath();
                   if let Err(err) =  data.save() {
                       err_vec.push(err.to_user(context));
                   }
                }
                OutMessage::ComputeEnd => {
                    break
                }
            }

        } else {
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
                    let context = data.getpath();
                    if let Err(err) =  data.save() {
                        err_vec.push(err.to_user(context));
                    }
                }
                OutMessage::ComputeEnd => {
                    break
                }
            }
        }
    };
err_vec
}

fn main() -> () {
    let deflate = FileOptions::default();
    // let start_time = Instant::now();


    let filter_vec = vec![OsStr::new("docx"),OsStr::new("xlsx"),OsStr::new("pdf")];
    let filter_set: HashSet<_> = filter_vec.iter().cloned().collect();

    let (oks, errs): (Vec<_>, Vec<_>) = WalkDir::new("C:\\$Recycle.Bin")
        .into_iter()
        .partition(|path|path.is_ok());


    let filtered:Vec<String> = oks
        .iter()
        .filter_map(|path| is_in(path.as_ref().unwrap(), &filter_vec))
        .collect();

    let mut errs: Vec<UISideErr> = errs
        .into_iter()
        .map(|err| PurgeErr::from(err.unwrap_err()).to_user("".to_string()))
        .collect();

    // println!("{:?}", errs.into_iter());
    // println!("{:?}", &filtered);
    if filtered.len() == 0 {
        for err in errs {
            println!("{}", err.ui_show());
        }
        return;
    }

    // let input_len_for_io = filtered.len();
    // let input_len_for_compute = input_len_for_io.clone();


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

        iterate_over_archives(filtered, deflate, itx, irx_arc_io, otx_io, orx, lock, cvar)

        });


    let compute_thread = thread::spawn(move || {
        let mut err_vec: Vec<UISideErr> = vec![];
        let (lock, cvar) = &*pair2;
        thread::sleep(Duration::new(0, 50*100));
        loop {
            let mut irx_locked = irx_arc_c.lock().unwrap();
            match irx_locked.recv() {
                Ok(message) => {
                    match message {
                        InMessage::Data(data) => {
                            drop(irx_locked);
                            let context = data.getpath();
                            match  data.process() {
                                Ok(edited_data) =>  {
                                    if let Err(err ) = otx.send(OutMessage::Data(edited_data)) {
                                        err_vec.push(PurgeErr::from(err).to_user(context))
                                    }
                                },
                                Err(err) => err_vec.push(PurgeErr::from(err).to_user(context))
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
    err_vec
    });
    errs.extend(io_thread.join().unwrap());
    errs.extend(compute_thread.join().unwrap());

    for err in errs {
        println!("{}", err.ui_show());
    }
}




