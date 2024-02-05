pub(crate) struct CoreXmlStr;

impl CoreXmlStr {
    pub const CORE_PATTERN_BEFORE_DATE: &'static [u8] = b"<dc:title";
    pub const CORE_PATTERN_AFTER_DATE: &'static [u8] = b"<dcterms:created";
    pub const CORE_TEMPLATE_BEFORE_DATE: &'static [u8] = b"<dc:title></dc:title>
        <dc:subject></dc:subject>
        <dc:creator></dc:creator>
        <cp:keywords></cp:keywords>
        <dc:description></dc:description>
        <cp:lastModifiedBy></cp:lastModifiedBy>
        <cp:revision></cp:revision>";
    pub const CORE_TEMPLATE_AFTER_DATE: &'static [u8] = b"World";
    pub const TEXT_TEMPLATE_1: &'static str = "<dc:title></dc:title>
        <dc:subject></dc:subject>
        <dc:creator></dc:creator>
        <cp:keywords></cp:keywords>
        <dc:description></dc:description>
        <cp:lastModifiedBy></cp:lastModifiedBy>
        <cp:revision></cp:revision><dcterms:created";

    pub const TEXT_TEMPLATE_2: &'static str = "<cp:category></cp:category>
<cp:contentStatus></cp:contentStatus>
</cp:coreProperties>";
}

