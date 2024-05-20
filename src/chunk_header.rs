#![allow(dead_code)]

use std::io::{
    Error,
    Cursor,
};
use byteorder::{
    LittleEndian,
    ReadBytesExt,
};
use crate::xml_types::XmlTypes;

/* Header that appears at the beginning of every chunk */
#[derive(Debug)]
pub struct ChunkHeader {
    /* Type identifier for this chunk.
     * The meaning of this value depends on the containing chunk. */
    pub chunk_type: XmlTypes,

    /* Size of the chunk header (in bytes).
     * Adding this value to the address of the chunk allows you to find
     * its associated data (if any). */
    pub header_size: u16,

    /* Total size of this chunk (in bytes).
     * This is the chunkSize plus the size of any data associated with the
     * chunk. Adding this value to the chunk allows you to completely skip
     * its contents (including any child chunks). If this value is the same
     * as chunkSize, there is no data associated with the chunk */
    pub size: u32,
}

impl ChunkHeader {

    pub fn from_buff(axml_buff: &mut Cursor<Vec<u8>>, expected_type: XmlTypes) -> Result<Self, Error> {
        /* Minimum size, for a chunk with no data */
        let minimum_size = 8;

        /* Get chunk type */
        let chunk_type = XmlTypes::parse_block_type(axml_buff)
                        .expect("Error: cannot parse block type");

        /* Check if this is indeed of the expected type */
        if chunk_type != expected_type {
            panic!("Error: unexpected XML chunk type");
        }

        /* Get chunk header size and total size */
        let header_size = axml_buff.read_u16::<LittleEndian>().unwrap();
        let size = axml_buff.read_u32::<LittleEndian>().unwrap();

        /* Exhaustive checks on the announced sizes */
        if header_size < minimum_size {
            panic!("Error: parsed header size is smaller than the minimum");
        }

        if size < minimum_size.into() {
            panic!("Error: parsed total size is smaller than the minimum");
        }

        if size < header_size.into() {
            panic!("Error: parsed total size if smaller than parsed header size");
        }

        /* Build and return the object */
        Ok(ChunkHeader {
            chunk_type,
            header_size,
            size,
        })
    }

    pub fn print(&self) {
        println!("----- Chunk header -----");
        println!("Header chunk_type: {:02X}", self.chunk_type);
        println!("Header header_size: {:02X}", self.header_size);
        println!("Chunk size: {:04X}", self.size);
        println!("----- End chunk header -----");
    }
}
