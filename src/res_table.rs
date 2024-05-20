#![allow(dead_code)]

use crate::chunk_header::ChunkHeader;
use crate::string_pool::StringPool;
use crate::xml_types::XmlTypes;

use std::io::{
    Error,
    Cursor,
};
use byteorder::{
    LittleEndian,
    ReadBytesExt
};

/**
 * Header for a resource table
 *
 * Its data contains a series of additional chunks:
 *   * A ResStringPool_header containing all table values.  This string pool
 *     contains all of the string values in the entire resource table (not
 *     the names of entries or type identifiers however).
 *   * One or more ResTable_package chunks.
 *
 * Specific entries within a resource table can be uniquely identified
 * with a single integer as defined by the ResTable_ref structure.
 */
pub struct ResTable {
    /* Chunk header */
    header: ChunkHeader,

    /* The number of ResTable_package structures */
    pub package_count: u32,
}

impl ResTable {
    pub fn parse(axml_buff: &mut Cursor<Vec<u8>>) {

        /* Go back 2 bytes, to account from the block type */
        let initial_offset = axml_buff.position();
        axml_buff.set_position(initial_offset - 2);

        /* Parse chunk header */
        let header = ChunkHeader::from_buff(axml_buff, XmlTypes::ResTableType)
                     .expect("Error: cannot get chunk header from string pool");

        /* Get package count */
        let package_count = axml_buff.read_u32::<LittleEndian>().unwrap();

        let mut strings = Vec::<String>::new();
        for _ in 0..package_count {
            let block_type = XmlTypes::parse_block_type(axml_buff)
                            .expect("Error: cannot parse block type");
            match block_type {
                XmlTypes::ResStringPoolType => {
                    StringPool::from_buff(axml_buff, &mut strings)
                               .expect("Error: cannot parse string pool header");
                },
                XmlTypes::ResTablePackageType => {
                    ResTablePackage::parse(axml_buff)
                                    .expect("Error: cannot parse table package");
                },
                _ => { panic!("######## Unexpected block type: {:02X}", block_type); }
            };
        }
    }
}

/**
 * A collection of resource data types within a package.  Followed by
 * one or more ResTable_type and ResTable_typeSpec structures containing the
 * entry values for each resource type.
 */
#[derive(Debug)]
pub struct ResTablePackage {
    /* Package header */
    // header: ResTable,
    header: ChunkHeader,

    /* If this is a base package, its ID.  Package IDs start
     * at 1 (corresponding to the value of the package bits in a
     * resource identifier).  0 means this is not a base package. */
    id: u32,

    /* Actual name of this package, \0-terminated. */
    name: [u16; 128],

    /* Offset to a ResStringPool_header defining the resource
     * type symbol table.  If zero, this package is inheriting from
     * another base package (overriding specific values in it). */
    type_strings: u32,

    /* Last index into typeStrings that is for public use by others. */
    last_public_type: u32,

    /* Offset to a ResStringPool_header defining the resource
     * key symbol table.  If zero, this package is inheriting from
     * another base package (overriding specific values in it). */
    key_strings: u32,

    /* Last index into keyStrings that is for public use by others. */
    last_public_key: u32,

    type_id_offset: u32,
}

impl ResTablePackage {
    pub fn parse(axml_buff: &mut Cursor<Vec<u8>>) -> Result<Self, Error> {

        /* Go back 2 bytes, to account from the block type */
        let initial_offset = axml_buff.position();
        axml_buff.set_position(initial_offset - 2);

        /* Parse chunk header */
        // let header = ResTable::from_buff(axml_buff)
        //              .expect("Error: cannot parse resource table header from string pool");
        let header = ChunkHeader::from_buff(axml_buff, XmlTypes::ResTablePackageType)
                     .expect("Error: cannot get chunk header for ResTablePackage");
        // let header = ChunkHeader { chunk_type: 0x0, header_size: 0x0, size: 0x0 };

        /* Get other members */
        let id = axml_buff.read_u32::<LittleEndian>().unwrap();

        // TODO: this could be simpler with an iterator
        let mut name: [u16; 128] = [0; 128];
        for i in 0..128 {
            name[i] = axml_buff.read_u16::<LittleEndian>().unwrap();
            if name[i] == 0x00 {
                break;
            }
        }
        let type_strings = axml_buff.read_u32::<LittleEndian>().unwrap();
        let last_public_type = axml_buff.read_u32::<LittleEndian>().unwrap();
        let key_strings = axml_buff.read_u32::<LittleEndian>().unwrap();
        let last_public_key = axml_buff.read_u32::<LittleEndian>().unwrap();
        let type_id_offset = axml_buff.read_u32::<LittleEndian>().unwrap();

        /* Build and return the object */
        Ok(ResTablePackage {
            header,
            id,
            name,
            type_strings,
            last_public_type,
            key_strings,
            last_public_key,
            type_id_offset
        })
    }
}
