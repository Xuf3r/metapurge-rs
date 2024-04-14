#![deny(clippy::unwrap_used)]
use std::ffi::OsString;

use std::collections::{HashSet};
use std::fmt::{Debug, Pointer};

use lopdf::{Document, Object, ObjectId,};

use std::str::{from_utf8, FromStr};
use xmp_toolkit::{ToStringOptions, XmpError, XmpMeta, XmpValue};
use crate::errors::error::PurgeErr;

use crate::traits::container::{DataPaths, Heaped, Purgable};

const ELEMENTS_TO_CLEAN: [(&str, &str); 4] = [
    ("http://purl.org/dc/elements/1.1/", "dc:title[1]" ),
    ("http://purl.org/dc/elements/1.1/", "dc:creator", ),
    ("http://ns.adobe.com/pdf/1.3/", "pdf:Producer"),
    ("http://ns.adobe.com/xap/1.0/mm/", "xmp:CreatorTool"),
];

const PDF_METADATA_KEYS: [&str; 8] = [
"Title", "Author", "Subject", "Keywords", "Creator", "Producer", "CreationDate", "ModDate"
];

fn clean_xmp(mut xmp: XmpMeta) -> Result<XmpMeta, PurgeErr>{
    for (ns, name) in ELEMENTS_TO_CLEAN {
        if let Some(_) = xmp.property(ns,name) {
            xmp.set_property(ns, name, &XmpValue::new("".to_owned()))?
        }
    }
    Ok(xmp)
}

enum state_Doc {
    Stub,
    Data(lopdf::Document)
}

enum dirty_Objs {
    Empty(ObjectId),
    Dict(ObjectId, Vec<Vec<u8>>),
    Stream(ObjectId)
}
pub(crate) struct Pdf {
    paths: DataPaths,
    data: state_Doc
}
impl Heaped for Pdf{
    fn inner_file_name(&self) -> String {
        self.paths.old_owned()
    }
}

impl Pdf {
    pub(crate) fn new(paths: DataPaths) -> Box<Self> {
        Box::from(Pdf {
            paths: paths,
            data: state_Doc::Stub
        })
    }
}

impl Pdf {

    pub(crate)  fn load(mut self: Box<Self>) -> Result<Box<Self>, PurgeErr> {
        self.data = state_Doc::Data(Document::load(&self.paths.old())?);

        Ok(self)
    }

    pub(crate) fn process(mut self: Box<Self>) -> Result<Box<Self>, PurgeErr> {

        let mut dirty_objs:Vec<dirty_Objs> = Vec::new();

        let mut doc = match &mut self.data {
            state_Doc::Stub => {unreachable!("This can and will never happen. \
            If encountered indicated a major logic flaw in the flow.")}
            state_Doc::Data(document) => document
        };

        for (object_id, object) in doc.objects.iter() {
            if let Ok(dict) = object.as_dict() {
                let dirty_keys: Vec<Vec<u8>> = dict
                    .iter()
                    .filter_map(|(key, value)|
                        {
                        if let Ok(str_key) = from_utf8(key) {
                                if PDF_METADATA_KEYS.contains(&str_key) {
                                    Some(key.clone())
                                }
                                else {None}
                            } else {
                            unreachable!("It's not supposed to happen. XMP keys are valid UTF-8 strings.\
                            I will not parse a broken PDF and refuse to believe it can happen.")
                        }
                        })
                    // .filter(Option::is_some)
                    .collect();
                if dirty_keys.len() == dict.len() {
                    dirty_objs.push(dirty_Objs::Empty(object_id.clone()))
                } else {
                    dirty_objs.push(dirty_Objs::Dict(object_id.clone(), dirty_keys))
                }
            } else if let Ok(strm) = object.as_stream() {
                if strm.dict.has("Subtype".as_bytes()) & strm.dict.has("Type".as_bytes()) {
                    dirty_objs.push(dirty_Objs::Stream(object_id.clone()))
                }
            }
        }

        for obj in dirty_objs {
            match obj{
                dirty_Objs::Empty(id) => {doc.objects.remove(&id);},

                dirty_Objs::Dict(id, keys) => {
                    if let Ok(mut dict) = doc.get_object_mut(id.clone()).unwrap().as_dict_mut() {
                        for key in keys {
                            dict.remove(key.as_slice());
                        }
                    }
                },

                dirty_Objs::Stream(id) => {
                    if let Ok(mut strm) = doc.get_object_mut(id.clone()).unwrap().as_stream_mut() {
                        let byte_slice: &[u8] = &strm.content;
                        let string_slice: &str = unsafe {

                            std::str::from_utf8_unchecked(byte_slice)
                        };
                        let mut loaded = xmp_toolkit::XmpMeta::from_str(string_slice)?;
                        match clean_xmp(loaded) {
                            Ok(cleaned)=> {
                                let cleaned_xmp = cleaned.to_string_with_options(ToStringOptions::default())?;
                            strm.set_content(cleaned_xmp.as_bytes().to_vec());
                            },
                            Err(err) => {
                                return Err(PurgeErr::from(err))
                            }
                        }
                    }
                }
            }
        }

        Ok(self)
    }

    pub(crate)  fn save(mut self: Box<Self>) -> Result<(), PurgeErr> {
        let mut data = match self.data {
            state_Doc::Stub => {unreachable!("Not possible.")}
            state_Doc::Data(data) => {data}
        };
        data.save(self.paths.old())?;
        // if let Err(hr) = std::fs::remove_file(&self.paths.old()) {
        //     std::fs::remove_file(&self.paths.temp_path);
        //     return Err(PurgeErr::from(hr))
        // };
        // if let Err(hr) = std::fs::rename(self.paths.temp(), self.paths.old()) {
        //     std::fs::remove_file(&self.paths.temp());
        //     return Err(PurgeErr::from(hr))
        // }
        Ok(())
    }

    pub(crate)  fn file_name(&self) -> String {
        self.paths.old_owned()
    }
}