use std::ffi::{OsStr, OsString};
use std::io::BufRead;
use std::path::Path;
use walkdir::DirEntry;
use crate::errors::error::PurgeErr;
use crate::pdf;
use crate::mso_x::mso_x;
use crate::mso_x::mso_x::MsoXPath;
use crate::pdf::PdfPath;
use crate::traits::container::MsoXPipe::*;
use crate::traits::container::PdfPipe::*;
use crate::traits::load_process_write::{LoadFs, Process, Finalize, Getpath};
use crate::exif::*;
use crate::traits::container::ExifPipe::{ExifDataVar, ExifFinalVar, ExifPathVar};
use crate::dyn_png::Png;
use crate::jpeg::Jpg;
// const SUPPORTED_EXT: [&str; 2] = ["docx", "xlsx"];


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
pub(crate) struct DataPaths {
    old_path: String,
    temp_path: String
}

impl DataPaths {
    pub(crate) fn new(path: &str) -> DataPaths {
        let old = path.to_string();
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
    pub(crate)  fn instantiate(self) -> Box<dyn Purgable> {
        match self.old_path.as_str() {
            PDF => Pdf::new(self) ,
            DOCX | XLSX => MsOX::new(self),
            PNG =>  Png::new(self),
            JPEG | JPG => Jpg::new(self),
            _ => panic!("Unsupported file"),
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


    pub(crate) enum Container {
        PdfPipe(PdfPipe),
        MsoXPipe(MsoXPipe),
        ExifPipe(ExifPipe)
    }

    pub(crate) enum PdfPipe {
        PdfPathVar(pdf::PdfPath),
        PdfDataVar(pdf::PdfData),
        PdfFinalVar(pdf::PdfFinal),
    }

    pub(crate) enum MsoXPipe {
        MsoXPathVar(mso_x::MsoXPath),
        MsoXDataVar(mso_x::MsoXData),
        MsoXFinalVar(mso_x::MsoXFinal),
    }
    pub(crate) enum ExifPipe {
        ExifPathVar(ExifPath),
        ExifDataVar(ExifData),
        ExifFinalVar(ExifFinal),
}

    impl Container {
        pub(crate) fn new (path: &str) -> Option<Container> {
            let extension = Path::new(path).extension().unwrap().to_str().unwrap();
            match extension.clone() {
                PDF => Some(Container::PdfPipe(PdfPathVar(PdfPath::new(path)))) ,
                DOCX | XLSX => Some(Container::MsoXPipe(MsoXPathVar(MsoXPath::new(path)))),
                img!() =>  Some(Container::ExifPipe(ExifPathVar(ExifPath::new(path)))),
                _ => None,
            }
        }
    }

impl Container {
    pub(crate) fn load (self) -> Result<Container, PurgeErr> {
        match self {
            Container::PdfPipe(PdfPathVar(pdfpath)) => {
                Ok(pdfpath.load()?)
            },
            Container::MsoXPipe(MsoXPathVar(msoxpath)) => {
                Ok(msoxpath.load()?)
            },
            Container::ExifPipe(ExifPathVar(exifpath)) => {
                Ok(exifpath.load()?)},
            _ => panic!("FOLLOW THE PIPELINE ORDER")
        }
    }
}

impl Container {
    pub(crate) fn process (self) -> Result<Container, PurgeErr> {
        match self {
            Container::PdfPipe(PdfDataVar(pdfdata)) => {
                Ok(pdfdata.process()?)
            },
            Container::MsoXPipe(MsoXDataVar(msoxdata)) => {
                Ok(msoxdata.process()?)
            },
            Container::ExifPipe(ExifDataVar(exifdata)) => {
                Ok(exifdata.process()?)},
            _ => panic!("FOLLOW THE PIPELINE ORDER")
        }
    }
}

impl Container {
    pub(crate) fn save (self) -> Result<(), PurgeErr> {
        match self {
            Container::PdfPipe(PdfFinalVar(pdffinal)) => {

                pdffinal.save()?;
                Ok(())

            },
            Container::MsoXPipe(MsoXFinalVar(msoxfinal)) => {

                msoxfinal.save()?;
                Ok(())
            },
            Container::ExifPipe(ExifFinalVar(exiffinal)) => {

                exiffinal.save()?;
                Ok(())
            },
            _ => panic!("FOLLOW THE PIPELINE ORDER")
        }
    }
}

impl Container {
    pub(crate)  fn getpath(&self) -> String {
match &self {
    Container::PdfPipe(x) => {match x {
        PdfPathVar(x) => {x.getpath()}
        PdfDataVar(x) => {x.getpath()}
        PdfFinalVar(x) => {x.getpath()}
    }
    }
    Container::MsoXPipe(x) => {match x {
        MsoXPathVar(x) => {x.getpath()}
        MsoXDataVar(x) => {x.getpath()}
        MsoXFinalVar(x) => {x.getpath()}
    }}
    Container::ExifPipe(x) => {match x {
        ExifPipe::ExifPathVar(x) => {x.getpath()}
        ExifPipe::ExifDataVar(x) => {x.getpath()}
        ExifPipe::ExifFinalVar(x) => {x.getpath()}
    }
    }
}

    }
}

pub(crate) trait Purgable {
    fn load(mut self: Box<Self>) -> Result<Box<Self>, PurgeErr>;
    fn process(mut self: Box<Self>) -> Result<Box<Self>, PurgeErr>;
    fn save(mut self: Box<Self>) -> Result<(), PurgeErr>;
    fn file_name(self: Box<Self>) -> String;
}


