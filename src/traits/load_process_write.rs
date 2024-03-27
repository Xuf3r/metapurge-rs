use crate::errors::error::PurgeErr;
use crate::pdf::PdfData;
use crate::traits::container::Container;

pub trait LoadFs {
    fn load(self) -> Result<Container, PurgeErr>;
}

pub trait Process {
    fn process(self) -> Result<Container, PurgeErr>;
}

pub trait Finalize {
    fn save(self) -> Result<(), PurgeErr>;
}

pub trait Getpath {
    fn getpath(&self) -> String;
}