#![allow(dead_code)]

pub struct XmlTypes {}

/* Type identifiers for chunks. Only includes the ones related to XML */
impl XmlTypes {
    pub const RES_NULL_TYPE: u16                = 0x0000;
    pub const RES_STRING_POOL_TYPE: u16         = 0x0001;
    pub const RES_TABLE_TYPE: u16               = 0x0002;
    pub const RES_XML_TYPE: u16                 = 0x0003;

    /* Chunk types in RES_XML_TYPE */
    pub const RES_XML_FIRST_CHUNK_TYPE: u16     = 0x0100;
    pub const RES_XML_START_NAMESPACE_TYPE: u16 = 0x0100;
    pub const RES_XML_END_NAMESPACE_TYPE: u16   = 0x0101;
    pub const RES_XML_START_ELEMENT_TYPE: u16   = 0x0102;
    pub const RES_XML_END_ELEMENT_TYPE: u16     = 0x0103;
    pub const RES_XML_CDATA_TYPE: u16           = 0x0104;
    pub const RES_XML_LAST_CHUNK_TYPE: u16      = 0x017f;

    /* This contains a uint32_t array mapping strings in the string
     * pool back to resource identifiers.  It is optional. */
    pub const RES_XML_RESOURCE_MAP_TYPE: u16    = 0x0180;

    /* Chunk types in RES_TABLE_TYPE */
    pub const RES_TABLE_PACKAGE_TYPE: u16       = 0x0200;
    pub const RES_TABLE_TYPE_TYPE: u16          = 0x0201;
    pub const RES_TABLE_TYPE_SPEC_TYPE: u16     = 0x0202;
    pub const RES_TABLE_LIBRARY_TYPE: u16       = 0x0203;
}
