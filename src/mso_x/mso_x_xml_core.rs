use crate::mso_x::mso_x_core_xml_templates as templates;
use crate::mso_x::utils;
struct CoreXml {
    data: Vec<u8>,

}
impl CoreXml {
    fn new(file_data: Vec<u8>) -> Self {
        CoreXml {data: file_data}
    }
}
trait MetadataRemove {

    fn before_date_data(self) -> Result<Self, String> where Self: Sized;
    fn after_date_data(self) -> Result<Self, String> where Self: Sized;

}

trait MetadataChainCaller: MetadataRemove {
    fn caller(self) -> Result<Self, String> where Self: Sized {
        Ok(self.before_date_data()?)

    }
}

impl MetadataRemove for CoreXml {
    fn before_date_data(mut self) -> Result<Self, String> where Self: Sized {
        // Implementation for before_date_data
        let start = utils::find_pattern_index(&self.data, &templates::CoreXmlStr::CORE_PATTERN_BEFORE_DATE);
        let end = utils::find_pattern_index(&self.data, &templates::CoreXmlStr::CORE_PATTERN_AFTER_DATE);
        if let (Some(start), Some(end)) = (start, end) {
            self.data[start..=end].copy_from_slice(&templates::CoreXmlStr::CORE_TEMPLATE_BEFORE_DATE);
            Ok(self)
        } else {
            Err("Unexpected document structure!".to_string())
        }
    }

    fn after_date_data(self) -> Result<Self, String> where Self: Sized {
        // Implementation for after_date_data
        Ok(self)
    }
}

