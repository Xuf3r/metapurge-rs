use std::ffi::OsStr;
use crate::pdf;
use crate::mso_x::mso_x;
use crate::mso_x::mso_x::MsoXPath;
use crate::pdf::PdfPath;
use crate::traits::Container::MsoXPipe::MsoXPathVar;
use crate::traits::Container::PdfPipe::PdfPathVar;

pub(crate) enum Container {
    PdfPipe(PdfPipe),
    MsoXPipe(MsoXPipe)
}

enum PdfPipe {
    PdfPathVar(pdf::PdfPath),
    PdfDataVar(pdf::PdfData),
    PdfFinalVar(pdf::PdfFinal),
}

enum MsoXPipe {
    MsoXPathVar(mso_x::MsoXPath),
    MsoXDataVar(mso_x::MsoXData),
    MsoXFinalVar(mso_x::MsoXFinal),
}

impl Container {
    fn new (path: String) -> Container {
        match extensions.get(path) {
            OsStr::new("pdf") => Container::PdfPipe(PdfPathVar(PdfPath::new())) ,
            OsStr::new("docx") | OsStr::new("xlsx") => Container::MsoXPipe(MsoXPathVar(MsoXPath::new()))
        }
    }
}