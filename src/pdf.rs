use std::ffi::OsString;
use std::fs::File;
use std::io::{BufWriter, Read, Write};
use lazy_static::lazy_static;
use regex::Regex;
use crate::load_process_write::*;
use std::collections::{HashSet};
use std::fmt::{Debug, Pointer};

use lopdf::{Document, Object, ObjectId,};

use std::str::from_utf8;

// /*lazy_static! {
//     static ref PRODUCER_REGEX: Regex = Regex::new(r#"Producer(.*?)"#).unwrap();
//     static ref AUTHOR_REGEX: Regex = Regex::new(r#"Author(.*?)"#).unwrap();
//     static ref SUBJECT_REGEX: Regex = Regex::new(r#"Subject(.*?)"#).unwrap();
//     static ref KEYWORDS_REGEX: Regex = Regex::new(r#"Keywords(.*?)"#).unwrap();
//     static ref CREATOR_REGEX: Regex = Regex::new(r#"Creator(.*?)"#).unwrap();
// }
//
// const PRODUCER_REPLACE: &str = "Producer()";
// const AUTHOR_REPLACE: &str = "Author()";
// const SUBJECT_REPLACE: &str = "Subject()";
// const KEYWORDS_REPLACE: &str = "Keywords()";
// const CREATOR_REPLACE: &str = "Creator()";*/

pub(crate) struct PdfPath{
    old_path: OsString,
    temp_path: OsString
}
pub(crate) struct PdfData{
    src: Box<lopdf::Document>,
    paths: PdfPath,
}
pub(crate) struct PdfFinal {
    finaldata: Box<lopdf::Document>,
    paths: PdfPath
}

impl LoadFs for PdfPath {
    fn load(mut self) -> Result<PdfData, std::io::Error> {
        // Open the file
        let doc = Document::load(&self.old_path)?;
        let mut temp = OsString::from(&self.old_path);
        temp.push("_temp");
        self.temp_path = temp;

        // Return PdfData
        Ok(PdfData{src: Box::new(doc), paths: self})
    }
}

impl Process for PdfData {
    fn process(mut self) -> PdfFinal {

        let pdf_metadata_keys: HashSet<&str> = [
        "Title", "Author", "Subject", "Keywords", "Creator", "Producer", "CreationDate", "ModDate"
    ].iter().cloned().collect();






    let mut empty_dicts:Vec<ObjectId> = vec![];
    for (object_id, mut object) in self.src.objects.iter_mut() {
        if let Ok(dict) = object.as_dict_mut() {
            let mut meta_keys: Vec<Vec<u8>> = vec![];
            for (key, value) in dict.iter_mut() {
                if let Ok(utf8_key) = from_utf8(&key) {
                    if pdf_metadata_keys.contains(utf8_key) {
                        meta_keys.push(key.clone());
                        println!("ping! key {} found!", utf8_key);

                    }
                }
            }
            let removed_meta:
                Vec<Option<lopdf::Object>> = meta_keys.iter().map(|key| dict.remove(key)).collect();
            if dict.is_empty() {
                println!("pong! {:?} with id {:?} is empty now and marked for deletion!", &object, &object_id);
                empty_dicts.push(object_id.clone());
            }
        }
    }

    let removed_dicts: Vec<_> = empty_dicts.iter().map(|id| self.src.objects.remove(id)).collect();

        // doc.compress(); //i don't know when to use it. technically since neither xmp nor sane metadata is compressed we might not compress

        PdfFinal {finaldata: self.src, paths: self.paths}
    }
}

impl Finalize for PdfFinal {
    fn save(mut self) -> Result<(), std::io::Error> {


        self.finaldata.save(&self.paths.temp_path)?;
        if let Err(hr) = std::fs::remove_file(&self.paths.old_path) {
            std::fs::remove_file(&self.paths.temp_path);
            return Err(hr)
        };
        std::fs::rename(&self.paths.temp_path, &self.paths.old_path)?;
        Ok(())
    }
}
