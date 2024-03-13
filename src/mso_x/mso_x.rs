#![deny(clippy::unwrap_used)]
use std::ffi::OsString;
use std::fs::File;
use lopdf::Reader;

use crate::errors::error::PurgeErr;

use zip::{ZipArchive, ZipWriter};


use crate::traits::load_process_write::*;

pub(crate) struct MsoXPath{
    old_path: OsString,
    temp_path: OsString
}
pub(crate) struct MsoXData{
    src: Box<ZipArchive<File>>,
    dst: File,
    paths: MsoXPath,
}
pub(crate) struct MsoXFinal {
    finaldata: Box<ZipWriter<File>>,
    dst: File,
    paths: MsoXPath
}


impl LoadFs for MsoXPath {
    fn load(mut self) -> Result<MsoXData, PurgeErr> {
        let file = File::open(&self.old_path)?;
        let archive = Box::new(ZipArchive::new(file)?);

        let mut temp = OsString::from(&self.old_path);

        temp.push("_temp");
        self.temp_path = temp;

        let outfile = File::create(&self.temp_path)?;

        Ok(MsoXData { src: archive, dst: outfile, paths: self })
    }
}

impl Process for MsoXData {
    fn process(mut self) -> Result<MsoXFinal, PurgeErr>{
        Ok(MsoXFinal::default())
    }
}

impl Finalize for MsoXFinal {
    fn save(mut self) -> Result<(), PurgeErr>{
    }
}