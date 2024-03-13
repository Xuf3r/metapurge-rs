use std::io;
use xmp_toolkit::XmpError;
use zip::result::ZipError;

pub(crate) enum PurgeErr {
    IoError(io::Error),
    XmpError(XmpError),
    ZipError(ZipError)
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