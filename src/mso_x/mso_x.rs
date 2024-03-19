#![deny(clippy::unwrap_used)]
use std::ffi::OsString;
use std::fs::File;
use std::{fs, io};
use std::io::{Read, Write};
use lopdf::Reader;

use crate::errors::error::PurgeErr;

use zip::{ZipArchive, ZipWriter};
use zip::write::FileOptions;
use crate::{find_rells, remove_rells, replace_corexml};
use crate::mso_x::mso_x_file_name_consts;

use crate::traits::load_process_write::*;

use lazy_static::lazy_static;
use crate::pdf::PdfPath;
use crate::traits::container::{Container, MsoXPipe};
use crate::traits::container::MsoXPipe::{MsoXFinalVar, MsoXPathVar};


lazy_static! {
    static ref DEFLATE_OPTION: FileOptions = FileOptions::default();
}
pub(crate) struct MsoXPath {
    old_path: OsString,
    temp_path: OsString
}
pub(crate) struct MsoXData {
    src: Box<ZipArchive<File>>,
    dst: File,
    paths: MsoXPath,
}
pub(crate) struct MsoXFinal {
    finaldata: Box<ZipWriter<File>>,
    paths: MsoXPath
}

impl MsoXPath {
    pub(crate) fn new(path: &str) -> MsoXPath {
        MsoXPath {
            old_path: OsString::from(path),
            temp_path: OsString::new()
        }
    }
}
impl LoadFs for MsoXPath {
    fn load(mut self) -> Result<Container, PurgeErr> {
        let file = File::open(&self.old_path)?;
        let archive = Box::new(ZipArchive::new(file)?);

        let mut temp = OsString::from(&self.old_path);

        temp.push("_temp");
        self.temp_path = temp;

        let outfile = File::create(&self.temp_path)?;

        Ok(
            Container::MsoXPipe(MsoXPipe::MsoXDataVar(MsoXData { src: archive, dst: outfile, paths: self }))
        )
    }
}

impl Process for MsoXData {
    fn process(mut self) -> Result<Container, PurgeErr> {
        let outfile = self.dst;
        let mut zipout_heap = Box::new(ZipWriter::new(outfile));
        let mut zipout = &mut *zipout_heap;

        for i in 0..self.src.len() {
            let mut file = self.src.by_index(i)?;
            let outpath = match file.enclosed_name() {
                Some(path) => path
                    .to_str()
                    .expect("how did Some() produce none?")
                    .to_owned(), //we unwrap because there's no possible way for path to be None. If it's none we're better off panicking.
                None => continue,
            };
            let mut content = Vec::with_capacity(1024);

            match outpath.as_str() {
                to_edit @ mso_x_file_name_consts::CORE_XML => {
                    let read_result = file.by_ref().read_to_end(&mut content)?;
                    let corexml = std::str::from_utf8(&content)?;
                    let replxml = replace_corexml(corexml);
                    zipout.start_file(to_edit, *DEFLATE_OPTION)?;
                    zipout.write_all(replxml.as_bytes())?;
                }
                to_edit @ mso_x_file_name_consts::RELS_XML => {
                    file.read_to_end(&mut content);
                    if let Some(index) = find_rells(&content) {
                        // println!("{}", index);
                        let rels = remove_rells(content, index);
                        zipout.start_file(to_edit, *DEFLATE_OPTION)?;
                        zipout.write_all(&rels);
                    } else {
                        zipout.start_file(to_edit, *DEFLATE_OPTION)?;
                        zipout.write_all(content.as_slice());
                    }
                }
                mso_x_file_name_consts::CUSTOM_XML => continue,
                no_edit => {
                    // file.read_to_end(&mut content).unwrap();
                    zipout.start_file(no_edit, *DEFLATE_OPTION)?;
                    // zipout.write_all(content.as_slice()).unwrap();
                    io::copy(&mut file, &mut zipout);
                }
            }

        }
        
        Ok(
            Container::MsoXPipe(MsoXFinalVar(MsoXFinal { finaldata: zipout_heap, paths: self.paths }))
        )
    }
}

impl Finalize for MsoXFinal {
    fn save(mut self) -> Result<(), PurgeErr> {


    self.finaldata.finish()?;
        fs::remove_file(&self.paths.old_path)?;
        fs::rename(&self.paths.temp_path, &self.paths.old_path)?;
        Ok(())

    }
}