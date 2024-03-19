use std::io;
use std::str::Utf8Error;
use xmp_toolkit::XmpError;
use zip::result::ZipError;
use lopdf::Error;
pub(crate) enum PurgeErr {
    IoError(io::Error),
    XmpError(XmpError),
    ZipError(ZipError),
    LopdfError(lopdf::Error),
    UTF8Error(Utf8Error)
    // Add other error types as needed
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