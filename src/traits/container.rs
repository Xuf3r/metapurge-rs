use std::ffi::{OsStr, OsString};
use std::path::Path;
use crate::errors::error::PurgeErr;
use crate::pdf;
use crate::mso_x::mso_x;
use crate::mso_x::mso_x::MsoXPath;
use crate::pdf::PdfPath;
use crate::traits::container::MsoXPipe::*;
use crate::traits::container::PdfPipe::*;
use crate::traits::load_process_write::{LoadFs, Process, Finalize};


// const SUPPORTED_EXT: [&str; 2] = ["docx", "xlsx"];
const PDF: &str = "pdf";
const DOCX: &str = "docx";
const XLSX: &str = "xlsx";
    pub(crate) enum Container {
        PdfPipe(PdfPipe),
        MsoXPipe(MsoXPipe)
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

    impl Container {
        pub(crate) fn new (path: &str) -> Option<Container> {
            let extension = Path::new(path).extension().unwrap().to_str().unwrap();
            println!("{:?}", extension);
            match extension.clone() {
                PDF => Some(Container::PdfPipe(PdfPathVar(PdfPath::new(path)))) ,
                DOCX | XLSX => Some(Container::MsoXPipe(MsoXPathVar(MsoXPath::new(path)))),
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
            _ => panic!("FOLLOW THE PIPELINE ORDER")
        }
    }
}

impl Container {
    pub(crate) fn save (self) -> Result<(), PurgeErr> {
        match self {
            Container::PdfPipe(PdfFinalVar(pdffinal)) => {
                println!("all ok 5 - calling save");
                pdffinal.save()?;
                Ok(())

            },
            Container::MsoXPipe(MsoXFinalVar(msoxfinal)) => {
                println!("all ok 5 - calling save");
                msoxfinal.save()?;
                Ok(())
            },
            _ => panic!("FOLLOW THE PIPELINE ORDER")
        }
    }
}