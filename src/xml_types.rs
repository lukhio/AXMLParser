#![allow(dead_code)]

use std::fmt;

/* Type identifiers for chunks. Only includes the ones related to XML */
#[derive(PartialEq)]
pub enum XmlTypes {
    ResNullType,
    ResStringPoolType,
    ResTableType,
    ResXmlType,

    /* Chunk types in RES_XML_Type */
    // TODO: for some reason this chunk has the same value has ResXmlStartNamespaceType which is
    // annoying. Need to figure out a way to deal with this. In the meantime, ignore it.
    // ResXmlFirstChunkType,
    ResXmlStartNamespaceType,
    ResXmlEndNamespaceType,
    ResXmlStartElementType,
    ResXmlEndElementType,
    ResXmlCDataType,
    ResXmlLastChunkType,

    /* This contains a uint32_t array mapping strings in the string
     * pool back to resource identifiers.  It is optional. */
    ResXmlResourceMapType,

    /* Chunk types in RES_TABLE_Type */
    ResTablePackageType,
    ResTableTypeType,
    ResTableTypeSpecType,
    ResTableLibraryType,
}

/* Implementation of the UpperHex trait for XmlTypes */
impl fmt::UpperHex for XmlTypes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            XmlTypes::ResNullType => write!(f, "{:X}", 0x0000),
            XmlTypes::ResStringPoolType => write!(f, "{:X}", 0x0001),
            XmlTypes::ResTableType => write!(f, "{:X}", 0x0002),
            XmlTypes::ResXmlType => write!(f, "{:X}", 0x0003),

            // TODO: see comment above.
            // XmlTypes::ResXmlFirstChunkType => write!(f, "{:X}", 0x0100),
            XmlTypes::ResXmlStartNamespaceType => write!(f, "{:X}", 0x0100),
            XmlTypes::ResXmlEndNamespaceType => write!(f, "{:X}", 0x0101),
            XmlTypes::ResXmlStartElementType => write!(f, "{:X}", 0x0102),
            XmlTypes::ResXmlEndElementType => write!(f, "{:X}", 0x0103),
            XmlTypes::ResXmlCDataType => write!(f, "{:X}", 0x0104),
            XmlTypes::ResXmlLastChunkType => write!(f, "{:X}", 0x017f),

            XmlTypes::ResXmlResourceMapType => write!(f, "{:X}", 0x0180),

            XmlTypes::ResTablePackageType => write!(f, "{:X}", 0x0200),
            XmlTypes::ResTableTypeType => write!(f, "{:X}", 0x0201),
            XmlTypes::ResTableTypeSpecType => write!(f, "{:X}", 0x0202),
            XmlTypes::ResTableLibraryType => write!(f, "{:X}", 0x0203),
        };
        Ok(())
    }
}
