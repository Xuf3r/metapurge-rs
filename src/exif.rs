use std::collections::HashSet;
use std::ffi::OsString;
use std::panic::catch_unwind;
use std::str::from_utf8;
use std::time::Instant;
use little_exif_panic::metadata::Metadata;
use little_exif_panic::exif_tag::ExifTag;
use lopdf::{Document, ObjectId};
use xmp_toolkit::ToStringOptions;
use crate::errors::error::{ExifStructureErr, PurgeErr};
use crate::pdf::{PdfData, PdfFinal, PdfPath};
use crate::traits::container::{Container, ExifPipe, PdfPipe};
use crate::traits::container::PdfPipe::PdfFinalVar;
use crate::traits::load_process_write::{Finalize, Getpath, LoadFs, Process};

pub(crate) struct ExifPath{
    old_path: OsString,
    temp_path: OsString
}
pub(crate) struct ExifData{
    src: little_exif_panic::metadata::Metadata ,
    paths: ExifPath,
}
pub(crate) struct ExifFinal {
    finaldata: little_exif_panic::metadata::Metadata,
    paths: ExifPath
}

impl Getpath for ExifPath{
    fn getpath(&self) -> String {
        self.old_path.clone().into_string().unwrap()
    }
}
impl Getpath for ExifData{
    fn getpath(&self) -> String {
        self.paths.old_path.clone().into_string().unwrap()
    }
}
impl Getpath for ExifFinal{
    fn getpath(&self) -> String {
        self.paths.old_path.clone().into_string().unwrap()
    }
}

impl ExifPath {
    pub(crate) fn new(path: &str) -> ExifPath {
        ExifPath {
            old_path: OsString::from(path),
            temp_path: OsString::new()
        }
    }
}
impl LoadFs for ExifPath {
    fn load(mut self) -> Result<Container, PurgeErr> {
        Ok(Container::ExifPipe(ExifPipe::ExifDataVar(ExifData {src: Metadata::new(), paths: self})))
        }

    }

impl Process for ExifData {
    fn process(mut self) -> Result<Container, PurgeErr> {

        Ok(Container::ExifPipe(ExifPipe::ExifFinalVar(ExifFinal {finaldata: self.src, paths: self.paths})))
    }
}

impl Finalize for ExifFinal {
    fn save(mut self) -> Result<(), PurgeErr> {
        let start = Instant::now();
        self.finaldata.write_to_file(self.paths.old_path.as_ref())?;
        let duration = start.elapsed();
        println!("{:?}", duration);
        Ok(())
    }
}