#![allow(dead_code)]

use std::fmt;
use std::io::{
    Error,
    Cursor,
};
use byteorder::{
    LittleEndian,
    ReadBytesExt
};

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

impl XmlTypes {
    pub fn parse_block_type(buff: &mut Cursor<Vec<u8>>) -> Result<Self, Error> {
        let raw_block_type = buff.read_u16::<LittleEndian>();
        let raw_block_type = match raw_block_type {
            Ok(block) => block,
            Err(e) => return Err(e),
        };

        let block_type = match raw_block_type {
            0x0000 => XmlTypes::ResNullType,
            0x0001 => XmlTypes::ResStringPoolType,
            0x0002 => XmlTypes::ResTableType,
            0x0003 => XmlTypes::ResXmlType,

            /* Chunk types in RES_XML_TYPE */
            // TODO: see comment above.
            // 0x0100 => XmlTypes::ResXmlFirstChunkType,
            0x0100 => XmlTypes::ResXmlStartNamespaceType,
            0x0101 => XmlTypes::ResXmlEndNamespaceType,
            0x0102 => XmlTypes::ResXmlStartElementType,
            0x0103 => XmlTypes::ResXmlEndElementType,
            0x0104 => XmlTypes::ResXmlCDataType,
            0x017f => XmlTypes::ResXmlLastChunkType,

            /* This contains a uint32_t array mapping strings in the string
             * pool back to resource identifiers. It is optional. */
            0x0180 => XmlTypes::ResXmlResourceMapType,

            /* Chunk types in RES_TABLE_TYPE */
            0x0200 => XmlTypes::ResTablePackageType,
            0x0201 => XmlTypes::ResTableTypeType,
            0x0202 => XmlTypes::ResTableTypeSpecType,
            0x0203 => XmlTypes::ResTableLibraryType,

            /* If we find an unknown type, we stop and panic */
            _ => panic!("Error: unknown block type {:02X}", raw_block_type)
        };

        Ok(block_type)
    }
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
