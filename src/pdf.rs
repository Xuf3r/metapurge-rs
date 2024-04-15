#![deny(clippy::unwrap_used)]

use std::fmt::{Debug, Pointer};

use lopdf::{Document, ObjectId, Stream};

use std::str::{from_utf8};
use xmp_toolkit::{XmpMeta, XmpProperty};
use crate::errors::error::PurgeErr;

use crate::traits::container::{DataPaths, Heaped, Purgable};

const ELEMENTS_TO_CLEAN: [(&str, &str); 4] = [
    ("http://purl.org/dc/elements/1.1/", "dc:title[1]" ),
    ("http://purl.org/dc/elements/1.1/", "dc:creator", ),
    ("http://ns.adobe.com/pdf/1.3/", "pdf:Producer"),
    ("http://ns.adobe.com/xap/1.0/mm/", "xmp:CreatorTool"),
];

const PDF_METADATA_KEYS: [&str; 11] = [
"Title", "Author", "Subject", "Keywords", "Creator", "Producer", "CreationDate", "ModDate", "Comments", "Company", "SourceModified"
];

const XMP_META_STREAM_KEYS: [&str; 2] = ["Subtype", "Type"];
const XMP_META_STREAM_SUBKEYS: [&str; 2] = ["XML", "Metadata"];
fn is_xmp_meta_stream(strm: &Stream ) -> Option<bool> {

    Option::from(
        XMP_META_STREAM_KEYS.iter()
        .filter_map(    |key|
            strm.dict.get(key.as_bytes()).ok()?
            .as_name_str().ok()
        )
        .collect::<Vec<&str>>()
        .eq(&XMP_META_STREAM_SUBKEYS)
    )

}
fn clean_xmp(mut xmp: XmpMeta) -> Result<XmpMeta, PurgeErr>{
    // for (ns, name) in ELEMENTS_TO_CLEAN {
    //     if let Some(_) = xmp.property(ns,name) {
    //         xmp.set_property(ns, name, &XmpValue::new("".to_owned()))?
    //     }
    // }
    let mut properties: Vec<XmpProperty> = Vec::new();
    for property in xmp.iter(Default::default()){
        if &property.name != "" {
            println!("For property '{}' schema is {}.", &property.name, &property.schema_ns);
            properties.push(property.clone())
        }
    };

    for prop in properties.iter() {
        xmp.delete_property(&prop.schema_ns, &prop.name).unwrap()
    };

    println!("_________________________\n\n");
    for property in xmp.iter(Default::default()){
        if &property.name != "" {
            println!("For property '{}' schema is {}.", &property.name, &property.schema_ns);
            properties.push(property.clone())
        }
    };
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
impl Pdf {
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

impl Heaped for Pdf {

    fn new(paths: DataPaths) -> Box<Self> {
        Box::from(Pdf {
            paths: paths,
            data: state_Doc::Stub
        })
    }

    fn inner_file_name(&self) -> String {
        self.paths.old_owned()
    }

    fn load(&mut self) -> Result<(), PurgeErr> {
        self.data = state_Doc::Data(Document::load(&self.paths.old())?);

        Ok(())
    }

    fn process(&mut self) -> Result<(), PurgeErr>  {

        let mut dirty_objs:Vec<dirty_Objs> = Vec::new();

        let mut doc = match &mut self.data {
            state_Doc::Stub => {unreachable!("This can and will never happen. \
            If encountered indicated a major logic flaw in the flow.")}
            state_Doc::Data(document) => document
        };

        for (object_id, object) in doc.objects.iter() {
            if let Ok(dict) = object.as_dict() {
                if dict.len() == 0 {
                    continue
                }
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
                } else if dirty_keys.len() != 0 {
                    dirty_objs.push(dirty_Objs::Dict(object_id.clone(), dirty_keys))
                }
            }
            else if object
                .as_stream().ok()
                .and_then(|strm| is_xmp_meta_stream(strm))
                .unwrap_or(false) {
                dirty_objs.push(dirty_Objs::Stream(object_id.clone()));
            }
            // else if let Ok(Ok(strm)) = object.as_stream() {
            //     if strm.dict.has("Subtype".as_bytes()) & strm.dict.has("Type".as_bytes()) {
            //         if (strm.dict.get("Subtype".as_bytes()).unwrap().as_name_str().unwrap() == "XML")
            //             &&  (strm.dict.get("Type".as_bytes()).unwrap().as_name_str().unwrap() == "Metadata") {
            //
            //         dirty_objs.push(dirty_Objs::Stream(object_id.clone()))
            //             }
            //     }
            // }
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
                    doc.objects.remove(&id);

                    // Potentially we can instead overwrite the metadata with empty values, so it looks less conscious.
                    // Probably has to be made into a CMD argument feature.

                    // if let Ok(mut strm) = doc.get_object_mut(id.clone()).unwrap().as_stream_mut() {
                    //     let byte_slice: &[u8] = &strm.content;
                    //     let string_slice: &str = unsafe {
                    //
                    //         std::str::from_utf8_unchecked(byte_slice)
                    //     };
                    //     let mut loaded = xmp_toolkit::XmpMeta::from_str(string_slice)?;
                    //     match clean_xmp(loaded) {
                    //         Ok(cleaned)=> {
                    //         let cleaned_xmp = cleaned.to_string_with_options(ToStringOptions::default())?;
                    //         strm.set_content(cleaned_xmp.as_bytes().to_vec());
                    //         },
                    //         Err(err) => {
                    //             panic!("pdf error!!");
                    //             return Err(PurgeErr::from(err))
                    //         }
                    //     }
                    // }
                }
            }
        }

        Ok(())
    }

    fn save(&mut self) -> Result<(), PurgeErr> {
        let  mut data = match &mut self.data {
            state_Doc::Stub => {unreachable!("Not possible.")}
            state_Doc::Data(ref mut data) => {data}
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

}