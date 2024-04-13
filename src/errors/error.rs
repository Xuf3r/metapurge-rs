use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::{fmt, io};
use std::str::Utf8Error;
use std::sync::mpsc::SendError;
use xmp_toolkit::{XmpError, XmpErrorType};
use zip::result::ZipError;
use crate::OutMessage;


#[derive(Debug, Clone)]
pub(crate) struct ExifStructureErr {
    info: String
}


impl ExifStructureErr {
    pub(crate) fn new(info: &str) -> ExifStructureErr{
        ExifStructureErr {
            info: info.to_string()
        }
    }
}
impl Error for ExifStructureErr {}
impl fmt::Display for ExifStructureErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "mangled metadata encountered, can't process")
    }
}




#[derive(Debug)]
pub(crate) struct UISideErr {
    path: String,
    info: String
}



impl UISideErr {
    pub(crate) fn ui_show(self) -> String {
        format!("Failed to clean file at {} .\n Reason: {} \n", self.path, self.info)
    }

}
trait ToUISideErr {
    fn to_user(&self, context: String) -> UISideErr;
}


impl UISideErr {
    fn prepare(self) -> String {
        format!("File at {} was not processed due to: {} ", self.path, self.info)
    }
}
///
impl ToUISideErr for std::io::Error {
    fn to_user(&self, context: String) -> UISideErr {
        UISideErr {
            path: context,
            info: self.to_string()
        }
    }
}

impl ToUISideErr for XmpError {
    fn to_user(&self, context: String) -> UISideErr {
        match self.error_type {

            _ => UISideErr{path: context, info: "Error while parsing pdf metadata".parse().unwrap() }
        }
    }
}

impl ToUISideErr for ZipError {
    fn to_user(&self, context: String) -> UISideErr {

        let info = match self {
            ZipError::Io(_) => "I/O error",
            ZipError::InvalidArchive(_) => "Invalid archive",
            ZipError::UnsupportedArchive(_) => "Unsupported archive",
            ZipError::FileNotFound => "File not found",
        };
        UISideErr { path: context, info: info.parse().unwrap() }

    }
}

impl ToUISideErr for lopdf::Error {
    fn to_user(&self, context: String) -> UISideErr {
         UISideErr{path:context, info: "Error parsing pdf".parse().unwrap() }
    }
}

impl ToUISideErr for std::str::Utf8Error {
    fn to_user(&self, context: String) -> UISideErr {
        UISideErr{path:context, info: self.to_string() }
    }
}

impl ToUISideErr for std::sync::mpsc::SendError<OutMessage> {
    fn to_user(&self, context: String) -> UISideErr {
        UISideErr{path:context, info: self.to_string() }
    }
}

impl ToUISideErr for walkdir::Error {
    fn to_user(&self, context: String) -> UISideErr {
        UISideErr{path: self.path().unwrap().to_str().unwrap().to_string() , info: self.source().unwrap().to_string() }
    }
}
impl ToUISideErr for ExifStructureErr {
    fn to_user(&self, context: String) -> UISideErr {
        UISideErr{path: context , info: self.info.clone() }
    }
}

#[derive(Debug)]
pub(crate) enum PurgeErr {
    IoError(io::Error),
    XmpError(XmpError),
    ZipError(ZipError),
    LopdfError(lopdf::Error),
    UTF8Error(Utf8Error),
    SendErrOut(SendError<OutMessage>),
    DirError(walkdir::Error),
    ExifError(ExifStructureErr)
}


impl From<io::Error> for PurgeErr {
    fn from(error: io::Error) -> Self {
        PurgeErr::IoError(error)
    }
}

impl From<XmpError> for PurgeErr {
    fn from(error: XmpError) -> Self {
        PurgeErr::XmpError(error)
    }
}

impl From<ZipError> for PurgeErr {
    fn from(error: ZipError) -> Self {
        PurgeErr::ZipError(error)
    }
}

impl From<lopdf::Error> for PurgeErr {
    fn from(error: lopdf::Error) -> Self {
        PurgeErr::LopdfError(error)
    }
}

impl From<Utf8Error> for PurgeErr {
    fn from(error: Utf8Error) -> Self {
        PurgeErr::UTF8Error(error)
    }
}


impl From<SendError<OutMessage>> for PurgeErr {
    fn from(error: SendError<OutMessage>) -> Self {
        PurgeErr::SendErrOut(error)
    }
}

impl From<walkdir::Error> for PurgeErr {
    fn from(error: walkdir::Error) -> Self {
        PurgeErr::DirError(error)
    }
}

impl From<ExifStructureErr> for PurgeErr {
    fn from(error: ExifStructureErr) -> Self {
        PurgeErr::ExifError(error)
    }
}

///

pub trait ToUser<T> {
    fn to_user(&self, context: String) -> T;
}

impl ToUser<UISideErr> for PurgeErr {
    fn to_user(&self, context: String) -> UISideErr {
        match self {
            PurgeErr::IoError(e) => e.to_user(context),
            PurgeErr::XmpError(e) => e.to_user(context),
            PurgeErr::ZipError(e) => e.to_user(context),
            PurgeErr::LopdfError(e) => e.to_user(context),
            PurgeErr::UTF8Error(e) => e.to_user(context),
            PurgeErr::SendErrOut(e) => e.to_user(context),
            PurgeErr::DirError(e) => e.to_user(context),
            PurgeErr::ExifError(e) => {e.to_user(context)}
        }
    }
}

