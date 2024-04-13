
use walkdir::DirEntry;
use crate::errors::error::PurgeErr;

use crate::pdf::{Pdf};

use crate::dyn_png::Png;
use crate::jpeg::Jpg;
use crate::mso_x::mso_x::MsOX;

macro_rules! img {
    () => {
        "png"|"jpeg"|"jpg"
    };
}
const PDF: &str = "pdf";
const DOCX: &str = "docx";
const XLSX: &str = "xlsx";
const JPEG: &str = "jpeg";
const JPG: &str = "jpg";
const PNG: &str = "png";

#[derive()]
pub(crate) enum DocumentType {
    Pdf(Box<Pdf>),
    Png(Box<Png>),
    Jpg(Box<Jpg>),
    MsOX(Box<MsOX>),
}
macro_rules! impl_document_type {
    ($($variant:ident),*) => {
        impl DocumentType {
            pub(crate) fn load(self) -> Result<Self, PurgeErr> {
                match self {
                    $(Self::$variant(inner) => inner.load().map(Self::$variant),)*
                }
            }

            pub(crate) fn process(self) -> Result<Self, PurgeErr> {
                match self {
                    $(Self::$variant(inner) => inner.process().map(Self::$variant),)*
                }
            }

           pub(crate)  fn save(self) -> Result<(), PurgeErr> {
                match self {
                    $(Self::$variant(inner) => {
                        inner.save()
                    },)*
                }
            }

           pub(crate)  fn file_name(&self) -> String {
                match self {
                    $(Self::$variant(ref inner) => inner.file_name(),)*
                }
            }
        }
    };
}

impl_document_type!(Pdf, Png, Jpg, MsOX);

#[derive()]
pub(crate) struct DataPaths {
    old_path: String,
    temp_path: String
}

impl DataPaths {
    pub(crate) fn new(path: walkdir::DirEntry) -> DataPaths {
        let old = path.path().to_str().unwrap().to_string();
        let mut temp = old.clone();
        temp.push_str("_temp");
        DataPaths {
            old_path: old,
            temp_path: temp
        }
    }

    pub(crate)  fn is_supported(path: &DirEntry) -> bool {
    let extension = match path.path().extension() {
            Some(string) => string.to_str().unwrap(),
            None => return false
        };
    match extension {
    PDF | DOCX | XLSX | PNG | JPEG | JPG => true,
    _ => false,
        }
    }
    pub(crate) fn instantiate(self) -> DocumentType {
        match self.old_path.split(".").last().unwrap() {
            PDF => DocumentType::Pdf(Pdf::new(self)),
            DOCX | XLSX => DocumentType::MsOX(MsOX::new(self)),
            PNG => DocumentType::Png(Png::new(self)),
            JPEG | JPG => DocumentType::Jpg(Jpg::new(self)),
            sum @ _  => panic!("Unsupported file type {sum}"),
        }
    }
    pub(crate) fn old_owned(&self) -> String {
        self.old_path.clone()
    }
    pub(crate) fn old(&self) -> &str {
        &self.old_path
    }

    pub(crate) fn temp_owned(&self) -> String {
        self.temp_path.clone()
    }
    pub(crate) fn temp(&self) -> &str {
        &self.temp_path
    }

    // why did I write this? the temp suffix is always the same
    // pub(crate) fn set_temp(mut self, temp: &str) -> Self {
    //
    //     self.temp_path = self.old_path.clone();
    //     self.temp_path.push_str(temp);
    //     self
    // }
}


pub(crate) trait Purgable {
    fn load(self: Box<Self>) -> Result<Box<Self>, PurgeErr>;
    fn process(self: Box<Self>) -> Result<Box<Self>, PurgeErr>;
    fn save(self: Box<Self>) -> Result<(), PurgeErr>;
    fn file_name(&self) -> String;
}


