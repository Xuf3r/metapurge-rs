#![deny(clippy::unwrap_used)]
use std::fs::File;
use std::{fs, io};
use std::io::{Read, Write};

use crate::errors::error::PurgeErr;

use zip::{ZipArchive, ZipWriter};
use zip::write::FileOptions;
use crate::{find_rells, remove_rells, replace_corexml};
use crate::mso_x::mso_x_file_name_consts;


use lazy_static::lazy_static;
use crate::traits::container::{DataPaths, Heaped};


lazy_static! {
    static ref DEFLATE_OPTION: FileOptions = FileOptions::default();
}

enum rw_MsOX {
    Stub,
    Archive(ZipArchive<File>),
    Writer(ZipWriter<File>)
}
pub(crate) struct MsOX {
    paths: DataPaths,
    data: rw_MsOX
}


impl Heaped for  MsOX {
    fn new(paths: DataPaths) -> Box<Self> {
        Box::new(
            MsOX {
                paths,
                data: rw_MsOX::Stub,
            }
        )
    }

    fn inner_file_name(&self) -> String {
        self.paths.old_owned()
    }
    fn load(&mut self) -> Result<(), PurgeErr> {
        let file = File::open(self.paths.old())?;
        self.data = rw_MsOX::Archive(ZipArchive::new(file)?);

        Ok(())
    }

    fn process(&mut self) -> Result<(), PurgeErr> {
        let file = File::create(self.paths.temp())?;
        let mut zipout = ZipWriter::new(file);

        let mut archive = match &mut self.data {
            rw_MsOX::Stub => {unreachable!("It can't happen.")},
            rw_MsOX::Archive(archive) => {archive}
            rw_MsOX::Writer(_) => {unreachable!("It can't happen.")}
        };
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
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
                    file.by_ref().read_to_end(&mut content)?;
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
                        zipout.write_all(content.as_slice())?;
                    }
                }
                mso_x_file_name_consts::CUSTOM_XML => continue,

                no_edit => {
                    // file.read_to_end(&mut content).unwrap();
                    zipout.start_file(no_edit, *DEFLATE_OPTION)?;
                    // zipout.write_all(content.as_slice()).unwrap();
                    io::copy(&mut file, &mut zipout)?;
                }
            };

        };

        self.data = rw_MsOX::Writer(zipout);
        Ok(())
    }

    fn save(&mut self) -> Result<(), PurgeErr>{
        let mut archive = match &mut self.data {
            rw_MsOX::Stub => {unreachable!("Can't happen.")},
            rw_MsOX::Archive(_) => {unreachable!("Can't happen.")},
            rw_MsOX::Writer(archive) => {archive}
        };
        archive.finish()?;
        if let Err(_) = fs::rename(&self.paths.temp(), &self.paths.old()) {
            fs::remove_file(&self.paths.old())?;
        }

        Ok(())

    }
}