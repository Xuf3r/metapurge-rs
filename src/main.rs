
extern crate core;
mod pdf;
mod mso_x;
mod traits;
mod errors;
mod dyn_png;
mod jpeg;


use std::ffi::OsStr;

use std::{thread};

use std::io::{BufRead, Read, Write};
use std::ops::Deref;
use walkdir::{DirEntry, WalkDir};

use std::sync::{Arc, Mutex, Condvar};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::time::{Duration};
use crate::mso_x::mso_x_core_xml_templates::*; // this doesn't seem to be used at all
use regex::Regex;
use zip::write::FileOptions;

use lazy_static::lazy_static;
use crate::errors::error::{PurgeErr, ToUser, UISideErr};

use crate::traits::container::{DataPaths, DocumentType, Purgable};
use native_dialog::{MessageDialog,};

fn echo(name: &str) {
    MessageDialog::new()
        .set_title("Result")
        .set_text(&format!("{}", &name))
        .show_alert()
        .unwrap();
}
fn echo_succ() {
    MessageDialog::new()
        .set_title("Success")
        .set_text(&format!("All documents have been successfully purged"))
        .show_alert()
        .unwrap();
}

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
    Data(DocumentType),
    ComputeEnd
}

enum InMessage {
    Data(DocumentType),
    ComputeEnd,
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


fn iterate_over_stubs(docs: Vec<DocumentType>,
                         itx: Sender<InMessage>,
                         irx: Arc<Mutex<Receiver<InMessage>>>,
                         otx: Sender<OutMessage>,
                         orx: Receiver<OutMessage>,
                         lock: &Mutex<bool>,
                         cvar: &Condvar) -> Vec<UISideErr> {
    let mut err_vec:Vec<UISideErr> = vec![];
    let mut started = lock.lock().unwrap();

    for stub in docs
    {
            let context = stub.file_name();
            match stub.load() {
                Ok(loaded) => {

                    itx.send(InMessage::Data(loaded));
                },
                Err(err) => {
                    err_vec.push(err.to_user(context));
                    continue
                },
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
                        let context = data.file_name();
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
                    let context = data.file_name();
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

    drop(started); // Drop the lock


    loop {
        if let Ok(message) = orx.recv() {

            match message {
                OutMessage::Data(mut data) => {
                    let context = data.file_name();
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





    // let path = env::args().nth(1).unwrap_or_else(|| {
    //     println!("Usage: {} <directory>", env::args().next().unwrap());
    //     std::process::exit(1);
    // });

    let (oks, errs): (Vec<_>, Vec<_>) = WalkDir::new("C:\\ferrprojs\\metapurge-rs")
        .into_iter()
        .partition(|path|path.is_ok());


    let paths: Vec<DirEntry> = oks.into_iter().map(Result::unwrap).collect();

    let dirty_stubs: Vec<DocumentType> = paths.into_iter()
        .filter(|path| DataPaths::is_supported(path))
        .map(DataPaths::new)
        .map(DataPaths::instantiate)
        .collect();

    let mut errs: Vec<UISideErr> = errs
        .into_iter()
        .map(|err| PurgeErr::from(err.unwrap_err()).to_user("".to_string()))
        .collect();

    // println!("{:?}", errs.into_iter());
    // println!("{:?}", &filtered);
    if dirty_stubs.len() == 0 {
        let errs = errs.into_iter().map( |item| item.ui_show()).collect::<Vec<String>>().join("\n");

        echo(&errs);
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

        iterate_over_stubs(dirty_stubs, itx, irx_arc_io, otx_io, orx, lock, cvar)

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
                            let context = data.file_name();
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

                            break
                        },
                    }
                },
                Err(_) => unreachable!("'itx' is guaranteed to be open while compute_thread is active"),
            };

        }
    err_vec
    });


    errs.extend(io_thread.join().unwrap());
    errs.extend(compute_thread.join().unwrap());

    if errs.len() != 0 {
        let errs = errs.into_iter().map(|item| item.ui_show()).collect::<Vec<String>>().join("\n");
        echo(&errs);
    }
    echo_succ();
}




