use crate::pdf::PdfData;
pub trait LoadFs {
    fn load(self) -> Result<PdfData, std::io::Error>;
}

pub trait Process {
    fn process(self);
}

pub trait Finalize {
    fn save(self);
}