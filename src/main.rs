use std::{
    env,
    fs,
    process::exit,
    collections::HashMap,
};
use std::io::{
    Read,
    Error,
    Cursor,
};
use byteorder::{
    LittleEndian,
    ReadBytesExt
};

struct XmlTypes {}

/* Type identifiers for chunks. Only includes the ones related to XML */
#[allow(dead_code)]
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

/* Header that appears at the beginning of every chunk */
struct ChunkHeader {
    /* Type identifier for this chunk.
     * The meaning of this value depends on the containing chunk. */
    chunk_type: u16,

    /* Size of the chunk header (in bytes).
     * Adding this value to the address of the chunk allows you to find
     * its associated data (if any). */
    header_size: u16,

    /* Total size of this chunk (in bytes).
     * This is the chunkSize plus the size of any data associated with the
     * chunk. Adding this value to the chunk allows you to completely skip
     * its contents (including any child chunks). If this value is the same
     * as chunkSize, there is no data associated with the chunk */
    size: u32,
}

impl ChunkHeader {

    fn from_buff(axml_buff: &mut Cursor<Vec<u8>>, expected_type: u16) -> Result<Self, Error> {
        /* Minimum size, for a chunk with no data */
        let minimum_size = 8;

        /* Get chunk type */
        let chunk_type = axml_buff.read_u16::<LittleEndian>().unwrap();

        /* Check if this is indeed of the expected type */
        if chunk_type != expected_type {
            panic!("Error: unexpected XML chunk type");
        }

        /* Get chunk header size and total size */
        let chunk_header_size = axml_buff.read_u16::<LittleEndian>().unwrap();
        let chunk_total_size = axml_buff.read_u32::<LittleEndian>().unwrap();

        /* Exhaustive checks on the announced sizes */
        if chunk_header_size < minimum_size {
            panic!("Error: parsed header size is smaller than the minimum");
        }

        if chunk_total_size < minimum_size.into() {
            panic!("Error: parsed total size is smaller than the minimum");
        }

        if chunk_total_size < chunk_header_size.into() {
            panic!("Error: parsed total size if smaller than parsed header size");
        }

        /* Build and return the object */
        Ok(ChunkHeader {
            chunk_type: chunk_type,
            header_size: chunk_header_size,
            size: chunk_total_size,
        })
    }

    fn print(&self) {
        println!("Header chunk_type: {:02X}", self.chunk_type);
        println!("Header header_size: {:02X}", self.header_size);
        println!("Chunk size: {:04X}", self.size);
        println!("--------------------");
    }
}

/* Header of a chunk representing a pool of strings
 *
 * Definition for a pool of strings.  The data of this chunk is an
 * array of uint32_t providing indices into the pool, relative to
 * stringsStart.  At stringsStart are all of the UTF-16 strings
 * concatenated together; each starts with a uint16_t of the string's
 * length and each ends with a 0x0000 terminator.  If a string is >
 * 32767 characters, the high bit of the length is set meaning to take
 * those 15 bits as a high word and it will be followed by another
 * uint16_t containing the low word.
 *
 * If styleCount is not zero, then immediately following the array of
 * uint32_t indices into the string table is another array of indices
 * into a style table starting at stylesStart.  Each entry in the
 * style table is an array of ResStringPool_span structures.
 */
struct StringPool {
    /* Chunk header */
    header: ChunkHeader,

    /* Number of strings in this pool (number of uint32_t indices that
     * follow in the data). */
    string_count: u32,

     /* Number of style span arrays in the pool (number of uint32_t
      * indices follow the string indices). */
    style_count: u32,

    /* Flags. Can take two values:
     *      - SORTED_FLAG = 1<<0,
     *      - UTF8_FLAG = 1<<8
     *
     * If SORTED_FLAG is set, the string index is sorted by the string
     * values (based on strcmp16()).
     *
     * If UTF8_FLAG is set, the string pool is ended in UTF-8.  */
    flags: u32,
    is_utf8: bool,

    /* Index from header of the string data. */
    strings_start: u32,

    /* Index from header of the style data. */
    styles_start: u32,

    strings_offsets: Vec<u32>,
    styles_offsets: Vec<u32>,
    strings: HashMap<u32, String>,
}

impl StringPool {

    fn from_buff(axml_buff: &mut Cursor<Vec<u8>>) -> Result<Self, Error> {
        /* Go back 2 bytes, to account from the block type */
        let offset = axml_buff.position();
        axml_buff.set_position(offset - 2);

        /* Parse chunk header */
        let header = ChunkHeader::from_buff(axml_buff, XmlTypes::RES_STRING_POOL_TYPE)
                     .expect("Error: cannot get chunk header from string pool");

        /* Get remaining members */
        let string_count = axml_buff.read_u32::<LittleEndian>().unwrap();
        let style_count = axml_buff.read_u32::<LittleEndian>().unwrap();
        let flags = axml_buff.read_u32::<LittleEndian>().unwrap();
        let is_utf8 = (flags & (1<<8)) != 0;
        let strings_start = axml_buff.read_u32::<LittleEndian>().unwrap();
        let styles_start = axml_buff.read_u32::<LittleEndian>().unwrap();

        /* Get strings offsets */
        let mut strings_offsets = Vec::new();
        for _ in 0..string_count {
            let offset = axml_buff.read_u32::<LittleEndian>().unwrap();
            strings_offsets.push(offset);
        }

        /* Get styles offsets */
        let mut styles_offsets = Vec::new();
        for _ in 0..style_count {
            let offset = axml_buff.read_u32::<LittleEndian>().unwrap();
            styles_offsets.push(offset);
        }

        /* Strings */
        let mut strings = HashMap::<u32, String>::new();
        let mut offset = 0;

        while strings.len() < string_count as usize {
            let str_size;
            let decoded_string;

            if is_utf8 {
                str_size = axml_buff.read_u8().unwrap() as u16;
                let mut str_buff = Vec::with_capacity(str_size as usize);
                let mut chunk = axml_buff.take(str_size.into());

                chunk.read_to_end(&mut str_buff).unwrap();
                decoded_string = String::from_utf8(str_buff).unwrap();

                // strings.insert(offset, decoded_string);
                // offset += str_size as u32;
            } else {
                str_size = axml_buff.read_u16::<LittleEndian>().unwrap();
                let iter = (0..str_size as usize)
                           .map(|_| axml_buff.read_u16::<LittleEndian>().unwrap());
                decoded_string = std::char::decode_utf16(iter).collect::<Result<String, _>>().unwrap();
            }

            if str_size > 0 {
                strings.insert(offset, decoded_string);
                offset += str_size as u32;
            }
        }

        /* Build and return the object */
        Ok(StringPool {
            header: header,
            string_count: string_count,
            style_count: style_count,
            flags: flags,
            is_utf8: is_utf8,
            strings_start: strings_start,
            styles_start: styles_start,
            strings_offsets: strings_offsets,
            styles_offsets: styles_offsets,
            strings: strings
        })
    }

    fn print(&self) {
        self.header.print();
        println!("String count: {:02X}", self.string_count);
        println!("Style count: {:02X}", self.style_count);
        println!("Flags: {:02X}", self.flags);
        println!("Is UTF-8: {:?}", self.is_utf8);
        println!("Strings start: {:02X}", self.strings_start);
        println!("Styles start: {:02X}", self.styles_start);
        println!("--------------------");
    }
}

/* Header of a chunk representing a resrouce map.
 * TODO: documentation
 */
struct ResourceMap {
    /* Chunk header */
    header: ChunkHeader,

    /* Resrouces IDs */
    resources_id: Vec<u32>,
}

impl ResourceMap {

    fn from_buff(axml_buff: &mut Cursor<Vec<u8>>) -> Result<Self, Error> {
        /* Go back 2 bytes, to account from the block type */
        let offset = axml_buff.position();
        axml_buff.set_position(offset - 2);

        /* Parse chunk header */
        let header = ChunkHeader::from_buff(axml_buff, XmlTypes::RES_XML_RESOURCE_MAP_TYPE)
                     .expect("Error: cannot get chunk header from string pool");

        header.print();

        /* Get resources IDs */
        let mut resources = Vec::new();
        let nb_resources = (header.size / 4) - 2;
        for _ in 0..nb_resources {
            let id = axml_buff.read_u32::<LittleEndian>().unwrap();
            resources.push(id);
        }

        Ok(ResourceMap {
            header: header,
            resources_id: resources,
        })
    }

    fn print(&self) {
        self.header.print();
        println!("Resource ID map:");
        for item in self.resources_id.iter() {
            println!("{:02}", item);
        }
        println!("--------------------");
    }
}


fn get_next_block_type(axml_buff: &mut Cursor<Vec<u8>>) -> Result<u16, Error> {
    let raw_block_type = axml_buff.read_u16::<LittleEndian>().unwrap();

    let block_type = match raw_block_type {
        0x0000 => XmlTypes::RES_NULL_TYPE,
        0x0001 => XmlTypes::RES_STRING_POOL_TYPE,
        0x0002 => XmlTypes::RES_TABLE_TYPE,
        0x0003 => XmlTypes::RES_XML_TYPE,
        0x0100 => XmlTypes::RES_XML_START_NAMESPACE_TYPE,
        0x0101 => XmlTypes::RES_XML_END_NAMESPACE_TYPE,
        0x0102 => XmlTypes::RES_XML_START_ELEMENT_TYPE,
        0x0103 => XmlTypes::RES_XML_END_ELEMENT_TYPE,
        0x0104 => XmlTypes::RES_XML_CDATA_TYPE,
        0x017f => XmlTypes::RES_XML_LAST_CHUNK_TYPE,
        0x0180 => XmlTypes::RES_XML_RESOURCE_MAP_TYPE,
        0x0200 => XmlTypes::RES_TABLE_PACKAGE_TYPE,
        0x0201 => XmlTypes::RES_TABLE_TYPE_TYPE,
        0x0202 => XmlTypes::RES_TABLE_TYPE_SPEC_TYPE,
        0x0203 => XmlTypes::RES_TABLE_LIBRARY_TYPE,
        _ => XmlTypes::RES_NULL_TYPE
    };

    Ok(block_type)
}

fn main() {
    /* Check CLI arguments */
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: ./{:} [AXML]", args[0]);
        exit(22);
    }

    let axml_path = &args[1];
    println!("[+] Parsing {}", axml_path);

    let mut raw_file = fs::File::open(axml_path).expect("Error: cannot open AXML file");
    let mut axml_vec_buff = Vec::new();
    raw_file.read_to_end(&mut axml_vec_buff).expect("Error: cannot read AXML file");
    let mut axml_buff = Cursor::new(axml_vec_buff);

    let header = ChunkHeader::from_buff(&mut axml_buff, XmlTypes::RES_XML_TYPE)
                 .expect("Error: cannot parse AXML header");

    header.print();

    /* Now parsing the rest of the file */
    loop {
        let block_type = get_next_block_type(&mut axml_buff);
        let block_type = match block_type {
            Ok(block) => block,
            Err(e) => break,
        };

        match block_type {
            XmlTypes::RES_NULL_TYPE => continue, //println!("null"),
            XmlTypes::RES_STRING_POOL_TYPE => {
                let string_pool = StringPool::from_buff(&mut axml_buff)
                                            .expect("Error: cannot parse string pool header");
                string_pool.print();
            },
            XmlTypes::RES_TABLE_TYPE => println!("TODO: RES_TABLE_TYPE"),
            XmlTypes::RES_XML_TYPE => println!("TODO: RES_XML_TYPE"),
            XmlTypes::RES_XML_START_NAMESPACE_TYPE => println!("TODO: RES_XML_START_NAMESPACE_TYPE"),
            XmlTypes::RES_XML_END_NAMESPACE_TYPE => println!("TODO: RES_XML_END_NAMESPACE_TYPE"),
            XmlTypes::RES_XML_START_ELEMENT_TYPE => println!("TODO: RES_XML_START_ELEMENT_TYPE"),
            XmlTypes::RES_XML_END_ELEMENT_TYPE => println!("TODO: RES_XML_END_ELEMENT_TYPE"),
            XmlTypes::RES_XML_CDATA_TYPE => println!("TODO: RES_XML_CDATA_TYPE"),
            XmlTypes::RES_XML_LAST_CHUNK_TYPE => println!("TODO: RES_XML_LAST_CHUNK_TYPE"),
            XmlTypes::RES_XML_RESOURCE_MAP_TYPE => {
                let resource_map = ResourceMap::from_buff(&mut axml_buff)
                                                .expect("Error: cannot parse resource map");
                resource_map.print();
            },
            XmlTypes::RES_TABLE_PACKAGE_TYPE => println!("TODO: RES_TABLE_PACKAGE_TYPE"),
            XmlTypes::RES_TABLE_TYPE_TYPE => println!("TODO: RES_TABLE_TYPE_TYPE"),
            XmlTypes::RES_TABLE_TYPE_SPEC_TYPE => println!("TODO: RES_TABLE_TYPE_SPEC_TYPE"),
            XmlTypes::RES_TABLE_LIBRARY_TYPE => println!("TODO: RES_TABLE_LIBRARY_TYPE"),
            _ => println!("{:02X}, other", block_type),
        }
    }
}
