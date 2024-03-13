#![deny(clippy::unwrap_used)]
use std::ffi::OsString;

use crate::traits::load_process_write::*;
use std::collections::{HashSet};
use std::fmt::{Debug, Pointer};

use lopdf::{Document, Object, ObjectId,};

use std::str::{from_utf8, FromStr};
use xmp_toolkit::{ToStringOptions, XmpError, XmpMeta, XmpValue};
use crate::errors::error::PurgeErr;

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
const ELEMENTS_TO_CLEAN: [(&str, &str,XmpValue<String>); 4] = [
    ("http://purl.org/dc/elements/1.1/", "dc:title[1]", XmpValue::new("".to_owned())),
    ("http://purl.org/dc/elements/1.1/", "dc:creator", XmpValue::new("".to_owned())),
    ("http://ns.adobe.com/pdf/1.3/", "pdf:Producer", XmpValue::new("".to_owned())),
    ("http://ns.adobe.com/xap/1.0/mm/", "xmp:CreatorTool", XmpValue::new("".to_owned())),
];

fn clean_xmp(mut xmp: XmpMeta) -> Result<(), PurgeErr>{
    for (ns, name, replace) in ELEMENTS_TO_CLEAN {
        if let Some(val) = xmp.property(ns,name) {
            xmp.set_property(ns, name, &replace)?
        }
    }
    Ok(())
}

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
    fn load(mut self) -> Result<PdfData, PurgeErr> {
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
    fn process(mut self) -> Result<PdfFinal, PurgeErr> {

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
                        // println!("ping! key {} found!", utf8_key);

                    }
                }
            }
            let removed_meta:
                Vec<Option<lopdf::Object>> = meta_keys.iter().map(|key| dict.remove(key)).collect();
            if dict.is_empty() {
                // println!("pong! {:?} with id {:?} is empty now and marked for deletion!", &object, &object_id);
                empty_dicts.push(object_id.clone());
            }
        } else if let Ok(mut strm) = object.as_stream() {
                if strm.dict.has("Subtype".as_bytes()) & strm.dict.has("Type".as_bytes()) {
                    let byte_slice: &[u8] = &strm.content;
                    let string_slice: &str = unsafe {
                        // Safety: We're asserting that the byte slice contains valid UTF-8 data
                        str::from_utf8_unchecked(byte_slice)
                    };
                    let mut loaded = xmp_toolkit::XmpMeta::from_str(string_slice)?;
                    if let Ok(()) = clean_xmp(loaded) {
                        let cleaned_xmp = loaded.to_string_with_options(ToStringOptions::default())?;
                        strm.set_content(cleaned_xmp.as_bytes().to_vec());
                    }
                }
        }
    }

    let removed_dicts: Vec<_> = empty_dicts.iter().map(|id| self.src.objects.remove(id)).collect();

        // doc.compress(); //i don't know when to use it. technically since neither xmp nor sane metadata is compressed we might not compress

        Ok(PdfFinal {finaldata: self.src, paths: self.paths})
    }
}

impl Finalize for PdfFinal {
    fn save(mut self) -> Result<(), PurgeErr> {


        self.finaldata.save(&self.paths.temp_path)?;
        if let Err(hr) = std::fs::remove_file(&self.paths.old_path) {
            std::fs::remove_file(&self.paths.temp_path);
            return Err(hr)
        };
        std::fs::rename(&self.paths.temp_path, &self.paths.old_path)?;
        Ok(())
    }
}
