
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

pub(crate) trait Heaped {
    fn inner_file_name(&self) -> String;
}

impl Heaped for Png {
    fn inner_file_name(&self) -> String {
        self.paths.old_owned()
    }
}

pub(crate) struct DataBox<T: Heaped + Sized> {
    data: Box<T>
}

impl DataBox<Png> {
    fn new(obj: Box<Png>, paths: DataPaths) -> Box<DataBox<Png>>{
        Box::new(DataBox {
            data: Png::new(paths)
        })
    }
}

impl<T: Heaped + Sized> DataBox<T> {
    pub(crate) fn file_name(&self) -> String {
        self.data.inner_file_name()
    }
}

impl Purgable for DataBox<Png> {
    fn load(mut self: Box<Self>) -> Result<Box<dyn Purgable>, PurgeErr> {
        self.data.load();
        Ok(self as Box<dyn Purgable>)
    }

    fn process(mut self: Box<Self>) -> Result<Box<dyn Purgable>, PurgeErr> {
        self.data.process();
        Ok(self as Box<dyn Purgable>)
    }

    fn save(mut self: Box<Self>) -> Result<(), PurgeErr> {
        self.data.save()?;
        Ok(())
    }

    fn file_name(&self) -> String {
        self.file_name()
    }
}
#[derive(Clone)]
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
    // PDF | DOCX | XLSX | PNG | JPEG | JPG => true,
        PNG => true,
    _ => false,
        }
    }
    pub(crate) fn instantiate(self) -> Box<dyn Purgable> {
        match self.old_path.split(".").last().unwrap() {
            PNG => DataBox::new(Png::new(self.clone()), self),
            // DOCX | XLSX => DataBox::new(MsOX::new(self)),
            // PDF => DataBox::new(Png::new(self)),
            // JPEG | JPG => DataBox::new(Jpg::new(self)),
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


pub(crate) trait Purgable: Send {
    fn load(self: Box<Self>) -> Result<Box<dyn Purgable>, PurgeErr>;
    fn process(self: Box<Self>) -> Result<Box<dyn Purgable>, PurgeErr>;
    fn save(self: Box<Self>) -> Result<(), PurgeErr>;

    fn file_name(&self) -> String;
}


