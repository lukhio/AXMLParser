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
        println!("----- Chunk header -----");
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
    strings: Vec<String>,
}

impl StringPool {

    fn from_buff(axml_buff: &mut Cursor<Vec<u8>>) -> Result<Self, Error> {
        /* Go back 2 bytes, to account from the block type */
        let initial_offset = axml_buff.position();
        axml_buff.set_position(initial_offset - 2);

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
        let mut strings = Vec::new();

        for offset in strings_offsets.iter() {
            let current_start = (strings_start + offset + 8) as u64;
            axml_buff.set_position(current_start);

            let str_size;
            let decoded_string;

            if is_utf8 {
                str_size = axml_buff.read_u8().unwrap() as u32;
                let mut str_buff = Vec::with_capacity(str_size as usize);
                let mut chunk = axml_buff.take(str_size.into());

                chunk.read_to_end(&mut str_buff).unwrap();
                decoded_string = String::from_utf8(str_buff).unwrap();
            } else {
                str_size = axml_buff.read_u16::<LittleEndian>().unwrap() as u32;
                let iter = (0..str_size as usize)
                        .map(|_| axml_buff.read_u16::<LittleEndian>().unwrap());
                decoded_string = std::char::decode_utf16(iter).collect::<Result<String, _>>().unwrap();
            }

            if str_size > 0 {
                strings.push(decoded_string);
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
        println!("----- String pool header -----");
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
        println!("----- Resource map header -----");
        println!("Resource ID map:");
        for item in self.resources_id.iter() {
            let string = get_resource_string(*item).unwrap();
            println!("{:?}", string);
        }
        println!("--------------------");
    }
}

struct DataValueType {}

#[allow(dead_code)]
impl DataValueType {
    /* The 'data' is either 0 or 1, specifying this resource is either undefined or empty,
     * respectively */
    pub const TYPE_NULL: u8	                = 0x00;
    /* The 'data' holds a ResTable_ref, a reference to another resource table entry */
    pub const TYPE_REFERENCE: u8	        = 0x01;
    /* The 'data' holds an attribute resource identifier */
    pub const TYPE_ATTRIBUTE: u8	        = 0x02;
    /* The 'data' holds an index into the containing resource table's global value string pool */
    pub const TYPE_STRING: u8	            = 0x03;
    /* The 'data' holds a single-precision floating point number */
    pub const TYPE_FLOAT: u8	            = 0x04;
    /* The 'data' holds a complex number encoding a dimension value, such as "100in" */
    pub const TYPE_DIMENSION: u8	        = 0x05;
    /* The 'data' holds a complex number encoding a fraction of a container */
    pub const TYPE_FRACTION: u8	            = 0x06;
    /* The 'data' holds a dynamic ResTable_ref, which needs to be resolved before it can be used
    * like a TYPE_REFERENCE */
    pub const TYPE_DYNAMIC_REFERENCE: u8    = 0x07;
    /* The 'data' holds an attribute resource identifier, which needs to be resolved before it can
    * be used like a TYPE_ATTRIBUTE */
    pub const TYPE_DYNAMIC_ATTRIBUTE: u8	= 0x08;

    /* Beginning of integer flavors... */
    pub const TYPE_FIRST_INT: u8	        = 0x10;

    /* The 'data' is a raw integer value of the form n..n */
    pub const TYPE_INT_DEC: u8	            = 0x10;
    /* The 'data' is a raw integer value of the form 0xn..n */
    pub const TYPE_INT_HEX: u8	            = 0x11;
    /* The 'data' is either 0 or 1, for input "false" or "true" respectively */
    pub const TYPE_INT_BOOLEAN: u8	        = 0x12;

    /* Beginning of color integer flavors... */
    pub const TYPE_FIRST_COLOR_INT: u8		= 0x1c;

    /* The 'data' is a raw integer value of the form #aarrggbb */
    pub const TYPE_INT_COLOR_ARGB8: u8		= 0x1c;
    /* The 'data' is a raw integer value of the form #rrggbb */
    pub const TYPE_INT_COLOR_RGB8: u8		= 0x1d;
    /* The 'data' is a raw integer value of the form #argb */
    pub const TYPE_INT_COLOR_ARGB4: u8		= 0x1e;
    /* The 'data' is a raw integer value of the form #rgb */
    pub const TYPE_INT_COLOR_RGB4: u8		= 0x1f;

    /* ...end of integer flavors */
    pub const TYPE_LAST_COLOR_INT: u8		= 0x1f;

    /* ...end of integer flavors */
    pub const TYPE_LAST_INT: u8	            = 0x1f;

    fn from_val(value: u8) -> u8 {
        return match value {
            0x00 => DataValueType::TYPE_NULL,
            0x01 => DataValueType::TYPE_REFERENCE,
            0x02 => DataValueType::TYPE_ATTRIBUTE,
            0x03 => DataValueType::TYPE_STRING,
            0x04 => DataValueType::TYPE_FLOAT,
            0x05 => DataValueType::TYPE_DIMENSION,
            0x06 => DataValueType::TYPE_FRACTION,
            0x07 => DataValueType::TYPE_DYNAMIC_REFERENCE,
            0x08 => DataValueType::TYPE_DYNAMIC_ATTRIBUTE,
            0x10 => DataValueType::TYPE_INT_DEC,
            0x11 => DataValueType::TYPE_INT_HEX,
            0x12 => DataValueType::TYPE_INT_BOOLEAN,
            0x1c => DataValueType::TYPE_INT_COLOR_ARGB8,
            0x1d => DataValueType::TYPE_INT_COLOR_RGB8,
            0x1e => DataValueType::TYPE_INT_COLOR_ARGB4,
            0x1f => DataValueType::TYPE_INT_COLOR_RGB4,
            _ => DataValueType::TYPE_NULL,
        }
    }
}

/* Representation of a value in a resource, supplying type
 * information.
 */
struct ResValue {
    /* Number of bytes in this structure */
    size: u16,

    /* Always set to 0 */
    res0: u8,

    data_type: u8,
    data: u32,
}

impl ResValue {
    fn from_buff(axml_buff: &mut Cursor<Vec<u8>>) -> Result<Self, Error> {
        let size = axml_buff.read_u16::<LittleEndian>().unwrap();
        let res0 = axml_buff.read_u8().unwrap();

        if res0 != 0 {
            panic!("res0 is not 0");
        }

        let data_type = DataValueType::from_val(axml_buff.read_u8().unwrap());
        let data = axml_buff.read_u32::<LittleEndian>().unwrap();

        Ok(ResValue {
            size: size,
            res0: res0,
            data_type: data_type,
            data: data
        })
    }
}

fn get_next_block_type(axml_buff: &mut Cursor<Vec<u8>>) -> Result<u16, Error> {
    let raw_block_type = axml_buff.read_u16::<LittleEndian>();
    let raw_block_type = match raw_block_type {
        Ok(block) => block,
        Err(e) => return Err(e),
    };

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

fn parse_start_namespace(axml_buff: &mut Cursor<Vec<u8>>, strings: &Vec::<String>) {
    /* Go back 2 bytes, to account from the block type */
    let offset = axml_buff.position();
    axml_buff.set_position(offset - 2);

    /* Parse chunk header */
    let header = ChunkHeader::from_buff(axml_buff, XmlTypes::RES_XML_START_NAMESPACE_TYPE)
                 .expect("Error: cannot get header from start namespace chunk");

    let line_number = axml_buff.read_u32::<LittleEndian>().unwrap();
    let comment = axml_buff.read_u32::<LittleEndian>().unwrap();
    let prefix = axml_buff.read_u32::<LittleEndian>().unwrap();
    let uri = axml_buff.read_u32::<LittleEndian>().unwrap();

    println!("----- Start namespace header -----");
    println!("prefix: {:?}", strings.get(prefix as usize).unwrap());
    println!("uri: {:?}", strings.get(uri as usize).unwrap());
}

fn parse_end_namespace(axml_buff: &mut Cursor<Vec<u8>>) {
    /* Go back 2 bytes, to account from the block type */
    let offset = axml_buff.position();
    axml_buff.set_position(offset - 2);

    /* Parse chunk header */
    let header = ChunkHeader::from_buff(axml_buff, XmlTypes::RES_XML_END_NAMESPACE_TYPE)
                 .expect("Error: cannot get header from start namespace chunk");

    let line_number = axml_buff.read_u32::<LittleEndian>().unwrap();
    let comment = axml_buff.read_u32::<LittleEndian>().unwrap();
    let prefix = axml_buff.read_u32::<LittleEndian>().unwrap();
    let uri = axml_buff.read_u32::<LittleEndian>().unwrap();

    println!("----- End namespace header -----");
}

fn parse_start_element(axml_buff: &mut Cursor<Vec<u8>>, strings: &Vec::<String>) {
    /* Go back 2 bytes, to account from the block type */
    let offset = axml_buff.position();
    axml_buff.set_position(offset - 2);

    /* Parse chunk header */
    let header = ChunkHeader::from_buff(axml_buff, XmlTypes::RES_XML_START_ELEMENT_TYPE)
                 .expect("Error: cannot get header from start namespace chunk");

    let line_number = axml_buff.read_u32::<LittleEndian>().unwrap();
    let comment = axml_buff.read_u32::<LittleEndian>().unwrap();
    let namespace = axml_buff.read_u32::<LittleEndian>().unwrap();
    let name = axml_buff.read_u32::<LittleEndian>().unwrap();
    let attribute_size = axml_buff.read_u32::<LittleEndian>().unwrap();
    let attribute_count = axml_buff.read_u16::<LittleEndian>().unwrap();
    let id_index = axml_buff.read_u16::<LittleEndian>().unwrap();
    let class_index = axml_buff.read_u16::<LittleEndian>().unwrap();
    let style_index = axml_buff.read_u16::<LittleEndian>().unwrap();

    println!("----- Start element header -----");
    if namespace != 0xffffffff {
        println!("namespace: {:?}", strings.get(namespace as usize).unwrap());
    }
    println!("name: {:?}", strings.get(name as usize).unwrap());

    println!("attribute count: {:02X}", attribute_count);

    for _ in 0..attribute_count {
        let attr_namespace = axml_buff.read_u32::<LittleEndian>().unwrap();
        let attr_name = axml_buff.read_u32::<LittleEndian>().unwrap();
        let attr_raw_val = axml_buff.read_u32::<LittleEndian>().unwrap();
        let data_value_type = ResValue::from_buff(axml_buff).unwrap();

        if attr_namespace != 0xffffffff {
            println!("--- attr_namespace: {:?}", strings.get(attr_namespace as usize).unwrap());
        }

        let mut decoded_attr = String::from(strings.get(attr_name as usize).unwrap());
        decoded_attr.push('=');
        // println!("--- attr_name: {:?}", strings.get(attr_name as usize).unwrap());
        if attr_raw_val != 0xffffffff {
            // println!("--- attr_raw_val: {:?}", strings.get(attr_raw_val as usize).unwrap());
            decoded_attr.push_str(&format!("\"{}\"", strings.get(attr_raw_val as usize).unwrap()));
        } else {
            match data_value_type.data_type {
                DataValueType::TYPE_NULL => println!("TODO: DataValueType::TYPE_NULL"),
                DataValueType::TYPE_REFERENCE => println!("TODO: DataValueType::TYPE_REFERENCE"),
                DataValueType::TYPE_ATTRIBUTE => println!("TODO: DataValueType::TYPE_ATTRIBUTE"),
                DataValueType::TYPE_STRING => println!("TODO: DataValueType::TYPE_STRING"),
                DataValueType::TYPE_FLOAT => println!("TODO: DataValueType::TYPE_FLOAT"),
                DataValueType::TYPE_DIMENSION => println!("TODO: DataValueType::TYPE_DIMENSION"),
                DataValueType::TYPE_FRACTION => println!("TODO: DataValueType::TYPE_FRACTION"),
                DataValueType::TYPE_DYNAMIC_REFERENCE => println!("TODO: DataValueType::TYPE_DYNAMIC_REFERENCE"),
                DataValueType::TYPE_DYNAMIC_ATTRIBUTE => println!("TODO: DataValueType::TYPE_DYNAMIC_ATTRIBUTE"),
                DataValueType::TYPE_INT_DEC => decoded_attr.push_str(&data_value_type.data.to_string()),
                DataValueType::TYPE_INT_HEX => {
                    decoded_attr.push_str("0x");
                    decoded_attr.push_str(&format!("{:x}", &data_value_type.data).to_string());
                },
                DataValueType::TYPE_INT_BOOLEAN => {
                    if data_value_type.data == 0 {
                        decoded_attr.push_str("\"false\"");
                    } else {
                        decoded_attr.push_str("\"true\"");
                    }
                },
                DataValueType::TYPE_INT_COLOR_ARGB8 => println!("TODO: DataValueType::TYPE_INT_COLOR_ARGB8"),
                DataValueType::TYPE_INT_COLOR_RGB8 => println!("TODO: DataValueType::TYPE_INT_COLOR_RGB8"),
                DataValueType::TYPE_INT_COLOR_ARGB4 => println!("TODO: DataValueType::TYPE_INT_COLOR_ARGB4"),
                DataValueType::TYPE_INT_COLOR_RGB4 => println!("TODO: DataValueType::TYPE_INT_COLOR_RGB4"),
                _ => println!("DataValueType::TYPE_NULL"),
            }
        }
        println!("===== {}", decoded_attr);

    }
}

fn parse_end_element(axml_buff: &mut Cursor<Vec<u8>>) {
    /* Go back 2 bytes, to account from the block type */
    let offset = axml_buff.position();
    axml_buff.set_position(offset - 2);

    /* Parse chunk header */
    let header = ChunkHeader::from_buff(axml_buff, XmlTypes::RES_XML_END_ELEMENT_TYPE)
                 .expect("Error: cannot get header from start namespace chunk");

    let line_number = axml_buff.read_u32::<LittleEndian>().unwrap();
    let comment = axml_buff.read_u32::<LittleEndian>().unwrap();
    let namespace = axml_buff.read_u32::<LittleEndian>().unwrap();
    let name = axml_buff.read_u32::<LittleEndian>().unwrap();
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
    /* TODO: probably not the best way to do this */
    let mut global_strings = Vec::new();

    loop {
        let block_type = get_next_block_type(&mut axml_buff);
        let block_type = match block_type {
            Ok(block) => block,
            Err(e) => break,
        };

        match block_type {
            XmlTypes::RES_NULL_TYPE => continue,
            XmlTypes::RES_STRING_POOL_TYPE => {
                let string_pool = StringPool::from_buff(&mut axml_buff)
                                            .expect("Error: cannot parse string pool header");
                for string in string_pool.strings.iter() {
                    global_strings.push(string.to_string());
                }
            },
            XmlTypes::RES_TABLE_TYPE => panic!("TODO: RES_TABLE_TYPE"),
            XmlTypes::RES_XML_TYPE => panic!("TODO: RES_XML_TYPE"),

            XmlTypes::RES_XML_START_NAMESPACE_TYPE => {
                parse_start_namespace(&mut axml_buff, &global_strings);
            },
            XmlTypes::RES_XML_END_NAMESPACE_TYPE => {
                parse_end_namespace(&mut axml_buff);
            },
            XmlTypes::RES_XML_START_ELEMENT_TYPE => {
                parse_start_element(&mut axml_buff, &global_strings);
            },
            XmlTypes::RES_XML_END_ELEMENT_TYPE => {
                parse_end_element(&mut axml_buff);
            },
            XmlTypes::RES_XML_CDATA_TYPE => panic!("TODO: RES_XML_CDATA_TYPE"),
            XmlTypes::RES_XML_LAST_CHUNK_TYPE => panic!("TODO: RES_XML_LAST_CHUNK_TYPE"),

            XmlTypes::RES_XML_RESOURCE_MAP_TYPE => {
                let resource_map = ResourceMap::from_buff(&mut axml_buff)
                                                .expect("Error: cannot parse resource map");
                resource_map.print();
            },

            XmlTypes::RES_TABLE_PACKAGE_TYPE => panic!("TODO: RES_TABLE_PACKAGE_TYPE"),
            XmlTypes::RES_TABLE_TYPE_TYPE => panic!("TODO: RES_TABLE_TYPE_TYPE"),
            XmlTypes::RES_TABLE_TYPE_SPEC_TYPE => panic!("TODO: RES_TABLE_TYPE_SPEC_TYPE"),
            XmlTypes::RES_TABLE_LIBRARY_TYPE => panic!("TODO: RES_TABLE_LIBRARY_TYPE"),

            _ => println!("{:02X}, other", block_type),
        }
    }

    println!("==========");
    println!("Finished parsing file.");
}

fn get_resource_string(mut id: u32) -> Result<String, Error> {
    let mut attr_names = Vec::new();
    attr_names.push("theme");
    attr_names.push("label");
    attr_names.push("icon");
    attr_names.push("name");
    attr_names.push("manageSpaceActivity");
    attr_names.push("allowClearUserData");
    attr_names.push("permission");
    attr_names.push("readPermission");
    attr_names.push("writePermission");
    attr_names.push("protectionLevel");
    attr_names.push("permissionGroup");
    attr_names.push("sharedUserId");
    attr_names.push("hasCode");
    attr_names.push("persistent");
    attr_names.push("enabled");
    attr_names.push("debuggable");
    attr_names.push("exported");
    attr_names.push("process");
    attr_names.push("taskAffinity");
    attr_names.push("multiprocess");
    attr_names.push("finishOnTaskLaunch");
    attr_names.push("clearTaskOnLaunch");
    attr_names.push("stateNotNeeded");
    attr_names.push("excludeFromRecents");
    attr_names.push("authorities");
    attr_names.push("syncable");
    attr_names.push("initOrder");
    attr_names.push("grantUriPermissions");
    attr_names.push("priority");
    attr_names.push("launchMode");
    attr_names.push("screenOrientation");
    attr_names.push("configChanges");
    attr_names.push("description");
    attr_names.push("targetPackage");
    attr_names.push("handleProfiling");
    attr_names.push("functionalTest");
    attr_names.push("value");
    attr_names.push("resource");
    attr_names.push("mimeType");
    attr_names.push("scheme");
    attr_names.push("host");
    attr_names.push("port");
    attr_names.push("path");
    attr_names.push("pathPrefix");
    attr_names.push("pathPattern");
    attr_names.push("action");
    attr_names.push("data");
    attr_names.push("targetClass");
    attr_names.push("colorForeground");
    attr_names.push("colorBackground");
    attr_names.push("backgroundDimAmount");
    attr_names.push("disabledAlpha");
    attr_names.push("textAppearance");
    attr_names.push("textAppearanceInverse");
    attr_names.push("textColorPrimary");
    attr_names.push("textColorPrimaryDisableOnly");
    attr_names.push("textColorSecondary");
    attr_names.push("textColorPrimaryInverse");
    attr_names.push("textColorSecondaryInverse");
    attr_names.push("textColorPrimaryNoDisable");
    attr_names.push("textColorSecondaryNoDisable");
    attr_names.push("textColorPrimaryInverseNoDisable");
    attr_names.push("textColorSecondaryInverseNoDisable");
    attr_names.push("textColorHintInverse");
    attr_names.push("textAppearanceLarge");
    attr_names.push("textAppearanceMedium");
    attr_names.push("textAppearanceSmall");
    attr_names.push("textAppearanceLargeInverse");
    attr_names.push("textAppearanceMediumInverse");
    attr_names.push("textAppearanceSmallInverse");
    attr_names.push("textCheckMark");
    attr_names.push("textCheckMarkInverse");
    attr_names.push("buttonStyle");
    attr_names.push("buttonStyleSmall");
    attr_names.push("buttonStyleInset");
    attr_names.push("buttonStyleToggle");
    attr_names.push("galleryItemBackground");
    attr_names.push("listPreferredItemHeight");
    attr_names.push("expandableListPreferredItemPaddingLeft");
    attr_names.push("expandableListPreferredChildPaddingLeft");
    attr_names.push("expandableListPreferredItemIndicatorLeft");
    attr_names.push("expandableListPreferredItemIndicatorRight");
    attr_names.push("expandableListPreferredChildIndicatorLeft");
    attr_names.push("expandableListPreferredChildIndicatorRight");
    attr_names.push("windowBackground");
    attr_names.push("windowFrame");
    attr_names.push("windowNoTitle");
    attr_names.push("windowIsFloating");
    attr_names.push("windowIsTranslucent");
    attr_names.push("windowContentOverlay");
    attr_names.push("windowTitleSize");
    attr_names.push("windowTitleStyle");
    attr_names.push("windowTitleBackgroundStyle");
    attr_names.push("alertDialogStyle");
    attr_names.push("panelBackground");
    attr_names.push("panelFullBackground");
    attr_names.push("panelColorForeground");
    attr_names.push("panelColorBackground");
    attr_names.push("panelTextAppearance");
    attr_names.push("scrollbarSize");
    attr_names.push("scrollbarThumbHorizontal");
    attr_names.push("scrollbarThumbVertical");
    attr_names.push("scrollbarTrackHorizontal");
    attr_names.push("scrollbarTrackVertical");
    attr_names.push("scrollbarAlwaysDrawHorizontalTrack");
    attr_names.push("scrollbarAlwaysDrawVerticalTrack");
    attr_names.push("absListViewStyle");
    attr_names.push("autoCompleteTextViewStyle");
    attr_names.push("checkboxStyle");
    attr_names.push("dropDownListViewStyle");
    attr_names.push("editTextStyle");
    attr_names.push("expandableListViewStyle");
    attr_names.push("galleryStyle");
    attr_names.push("gridViewStyle");
    attr_names.push("imageButtonStyle");
    attr_names.push("imageWellStyle");
    attr_names.push("listViewStyle");
    attr_names.push("listViewWhiteStyle");
    attr_names.push("popupWindowStyle");
    attr_names.push("progressBarStyle");
    attr_names.push("progressBarStyleHorizontal");
    attr_names.push("progressBarStyleSmall");
    attr_names.push("progressBarStyleLarge");
    attr_names.push("seekBarStyle");
    attr_names.push("ratingBarStyle");
    attr_names.push("ratingBarStyleSmall");
    attr_names.push("radioButtonStyle");
    attr_names.push("scrollbarStyle");
    attr_names.push("scrollViewStyle");
    attr_names.push("spinnerStyle");
    attr_names.push("starStyle");
    attr_names.push("tabWidgetStyle");
    attr_names.push("textViewStyle");
    attr_names.push("webViewStyle");
    attr_names.push("dropDownItemStyle");
    attr_names.push("spinnerDropDownItemStyle");
    attr_names.push("dropDownHintAppearance");
    attr_names.push("spinnerItemStyle");
    attr_names.push("mapViewStyle");
    attr_names.push("preferenceScreenStyle");
    attr_names.push("preferenceCategoryStyle");
    attr_names.push("preferenceInformationStyle");
    attr_names.push("preferenceStyle");
    attr_names.push("checkBoxPreferenceStyle");
    attr_names.push("yesNoPreferenceStyle");
    attr_names.push("dialogPreferenceStyle");
    attr_names.push("editTextPreferenceStyle");
    attr_names.push("ringtonePreferenceStyle");
    attr_names.push("preferenceLayoutChild");
    attr_names.push("textSize");
    attr_names.push("typeface");
    attr_names.push("textStyle");
    attr_names.push("textColor");
    attr_names.push("textColorHighlight");
    attr_names.push("textColorHint");
    attr_names.push("textColorLink");
    attr_names.push("state_focused");
    attr_names.push("state_window_focused");
    attr_names.push("state_enabled");
    attr_names.push("state_checkable");
    attr_names.push("state_checked");
    attr_names.push("state_selected");
    attr_names.push("state_active");
    attr_names.push("state_single");
    attr_names.push("state_first");
    attr_names.push("state_middle");
    attr_names.push("state_last");
    attr_names.push("state_pressed");
    attr_names.push("state_expanded");
    attr_names.push("state_empty");
    attr_names.push("state_above_anchor");
    attr_names.push("ellipsize");
    attr_names.push("x");
    attr_names.push("y");
    attr_names.push("windowAnimationStyle");
    attr_names.push("gravity");
    attr_names.push("autoLink");
    attr_names.push("linksClickable");
    attr_names.push("entries");
    attr_names.push("layout_gravity");
    attr_names.push("windowEnterAnimation");
    attr_names.push("windowExitAnimation");
    attr_names.push("windowShowAnimation");
    attr_names.push("windowHideAnimation");
    attr_names.push("activityOpenEnterAnimation");
    attr_names.push("activityOpenExitAnimation");
    attr_names.push("activityCloseEnterAnimation");
    attr_names.push("activityCloseExitAnimation");
    attr_names.push("taskOpenEnterAnimation");
    attr_names.push("taskOpenExitAnimation");
    attr_names.push("taskCloseEnterAnimation");
    attr_names.push("taskCloseExitAnimation");
    attr_names.push("taskToFrontEnterAnimation");
    attr_names.push("taskToFrontExitAnimation");
    attr_names.push("taskToBackEnterAnimation");
    attr_names.push("taskToBackExitAnimation");
    attr_names.push("orientation");
    attr_names.push("keycode");
    attr_names.push("fullDark");
    attr_names.push("topDark");
    attr_names.push("centerDark");
    attr_names.push("bottomDark");
    attr_names.push("fullBright");
    attr_names.push("topBright");
    attr_names.push("centerBright");
    attr_names.push("bottomBright");
    attr_names.push("bottomMedium");
    attr_names.push("centerMedium");
    attr_names.push("id");
    attr_names.push("tag");
    attr_names.push("scrollX");
    attr_names.push("scrollY");
    attr_names.push("background");
    attr_names.push("padding");
    attr_names.push("paddingLeft");
    attr_names.push("paddingTop");
    attr_names.push("paddingRight");
    attr_names.push("paddingBottom");
    attr_names.push("focusable");
    attr_names.push("focusableInTouchMode");
    attr_names.push("visibility");
    attr_names.push("fitsSystemWindows");
    attr_names.push("scrollbars");
    attr_names.push("fadingEdge");
    attr_names.push("fadingEdgeLength");
    attr_names.push("nextFocusLeft");
    attr_names.push("nextFocusRight");
    attr_names.push("nextFocusUp");
    attr_names.push("nextFocusDown");
    attr_names.push("clickable");
    attr_names.push("longClickable");
    attr_names.push("saveEnabled");
    attr_names.push("drawingCacheQuality");
    attr_names.push("duplicateParentState");
    attr_names.push("clipChildren");
    attr_names.push("clipToPadding");
    attr_names.push("layoutAnimation");
    attr_names.push("animationCache");
    attr_names.push("persistentDrawingCache");
    attr_names.push("alwaysDrawnWithCache");
    attr_names.push("addStatesFromChildren");
    attr_names.push("descendantFocusability");
    attr_names.push("layout");
    attr_names.push("inflatedId");
    attr_names.push("layout_width");
    attr_names.push("layout_height");
    attr_names.push("layout_margin");
    attr_names.push("layout_marginLeft");
    attr_names.push("layout_marginTop");
    attr_names.push("layout_marginRight");
    attr_names.push("layout_marginBottom");
    attr_names.push("listSelector");
    attr_names.push("drawSelectorOnTop");
    attr_names.push("stackFromBottom");
    attr_names.push("scrollingCache");
    attr_names.push("textFilterEnabled");
    attr_names.push("transcriptMode");
    attr_names.push("cacheColorHint");
    attr_names.push("dial");
    attr_names.push("hand_hour");
    attr_names.push("hand_minute");
    attr_names.push("format");
    attr_names.push("checked");
    attr_names.push("button");
    attr_names.push("checkMark");
    attr_names.push("foreground");
    attr_names.push("measureAllChildren");
    attr_names.push("groupIndicator");
    attr_names.push("childIndicator");
    attr_names.push("indicatorLeft");
    attr_names.push("indicatorRight");
    attr_names.push("childIndicatorLeft");
    attr_names.push("childIndicatorRight");
    attr_names.push("childDivider");
    attr_names.push("animationDuration");
    attr_names.push("spacing");
    attr_names.push("horizontalSpacing");
    attr_names.push("verticalSpacing");
    attr_names.push("stretchMode");
    attr_names.push("columnWidth");
    attr_names.push("numColumns");
    attr_names.push("src");
    attr_names.push("antialias");
    attr_names.push("filter");
    attr_names.push("dither");
    attr_names.push("scaleType");
    attr_names.push("adjustViewBounds");
    attr_names.push("maxWidth");
    attr_names.push("maxHeight");
    attr_names.push("tint");
    attr_names.push("baselineAlignBottom");
    attr_names.push("cropToPadding");
    attr_names.push("textOn");
    attr_names.push("textOff");
    attr_names.push("baselineAligned");
    attr_names.push("baselineAlignedChildIndex");
    attr_names.push("weightSum");
    attr_names.push("divider");
    attr_names.push("dividerHeight");
    attr_names.push("choiceMode");
    attr_names.push("itemTextAppearance");
    attr_names.push("horizontalDivider");
    attr_names.push("verticalDivider");
    attr_names.push("headerBackground");
    attr_names.push("itemBackground");
    attr_names.push("itemIconDisabledAlpha");
    attr_names.push("rowHeight");
    attr_names.push("maxRows");
    attr_names.push("maxItemsPerRow");
    attr_names.push("moreIcon");
    attr_names.push("max");
    attr_names.push("progress");
    attr_names.push("secondaryProgress");
    attr_names.push("indeterminate");
    attr_names.push("indeterminateOnly");
    attr_names.push("indeterminateDrawable");
    attr_names.push("progressDrawable");
    attr_names.push("indeterminateDuration");
    attr_names.push("indeterminateBehavior");
    attr_names.push("minWidth");
    attr_names.push("minHeight");
    attr_names.push("interpolator");
    attr_names.push("thumb");
    attr_names.push("thumbOffset");
    attr_names.push("numStars");
    attr_names.push("rating");
    attr_names.push("stepSize");
    attr_names.push("isIndicator");
    attr_names.push("checkedButton");
    attr_names.push("stretchColumns");
    attr_names.push("shrinkColumns");
    attr_names.push("collapseColumns");
    attr_names.push("layout_column");
    attr_names.push("layout_span");
    attr_names.push("bufferType");
    attr_names.push("text");
    attr_names.push("hint");
    attr_names.push("textScaleX");
    attr_names.push("cursorVisible");
    attr_names.push("maxLines");
    attr_names.push("lines");
    attr_names.push("height");
    attr_names.push("minLines");
    attr_names.push("maxEms");
    attr_names.push("ems");
    attr_names.push("width");
    attr_names.push("minEms");
    attr_names.push("scrollHorizontally");
    attr_names.push("password");
    attr_names.push("singleLine");
    attr_names.push("selectAllOnFocus");
    attr_names.push("includeFontPadding");
    attr_names.push("maxLength");
    attr_names.push("shadowColor");
    attr_names.push("shadowDx");
    attr_names.push("shadowDy");
    attr_names.push("shadowRadius");
    attr_names.push("numeric");
    attr_names.push("digits");
    attr_names.push("phoneNumber");
    attr_names.push("inputMethod");
    attr_names.push("capitalize");
    attr_names.push("autoText");
    attr_names.push("editable");
    attr_names.push("freezesText");
    attr_names.push("drawableTop");
    attr_names.push("drawableBottom");
    attr_names.push("drawableLeft");
    attr_names.push("drawableRight");
    attr_names.push("drawablePadding");
    attr_names.push("completionHint");
    attr_names.push("completionHintView");
    attr_names.push("completionThreshold");
    attr_names.push("dropDownSelector");
    attr_names.push("popupBackground");
    attr_names.push("inAnimation");
    attr_names.push("outAnimation");
    attr_names.push("flipInterval");
    attr_names.push("fillViewport");
    attr_names.push("prompt");
    attr_names.push("startYear");
    attr_names.push("endYear");
    attr_names.push("mode");
    attr_names.push("layout_x");
    attr_names.push("layout_y");
    attr_names.push("layout_weight");
    attr_names.push("layout_toLeftOf");
    attr_names.push("layout_toRightOf");
    attr_names.push("layout_above");
    attr_names.push("layout_below");
    attr_names.push("layout_alignBaseline");
    attr_names.push("layout_alignLeft");
    attr_names.push("layout_alignTop");
    attr_names.push("layout_alignRight");
    attr_names.push("layout_alignBottom");
    attr_names.push("layout_alignParentLeft");
    attr_names.push("layout_alignParentTop");
    attr_names.push("layout_alignParentRight");
    attr_names.push("layout_alignParentBottom");
    attr_names.push("layout_centerInParent");
    attr_names.push("layout_centerHorizontal");
    attr_names.push("layout_centerVertical");
    attr_names.push("layout_alignWithParentIfMissing");
    attr_names.push("layout_scale");
    attr_names.push("visible");
    attr_names.push("variablePadding");
    attr_names.push("constantSize");
    attr_names.push("oneshot");
    attr_names.push("duration");
    attr_names.push("drawable");
    attr_names.push("shape");
    attr_names.push("innerRadiusRatio");
    attr_names.push("thicknessRatio");
    attr_names.push("startColor");
    attr_names.push("endColor");
    attr_names.push("useLevel");
    attr_names.push("angle");
    attr_names.push("type");
    attr_names.push("centerX");
    attr_names.push("centerY");
    attr_names.push("gradientRadius");
    attr_names.push("color");
    attr_names.push("dashWidth");
    attr_names.push("dashGap");
    attr_names.push("radius");
    attr_names.push("topLeftRadius");
    attr_names.push("topRightRadius");
    attr_names.push("bottomLeftRadius");
    attr_names.push("bottomRightRadius");
    attr_names.push("left");
    attr_names.push("top");
    attr_names.push("right");
    attr_names.push("bottom");
    attr_names.push("minLevel");
    attr_names.push("maxLevel");
    attr_names.push("fromDegrees");
    attr_names.push("toDegrees");
    attr_names.push("pivotX");
    attr_names.push("pivotY");
    attr_names.push("insetLeft");
    attr_names.push("insetRight");
    attr_names.push("insetTop");
    attr_names.push("insetBottom");
    attr_names.push("shareInterpolator");
    attr_names.push("fillBefore");
    attr_names.push("fillAfter");
    attr_names.push("startOffset");
    attr_names.push("repeatCount");
    attr_names.push("repeatMode");
    attr_names.push("zAdjustment");
    attr_names.push("fromXScale");
    attr_names.push("toXScale");
    attr_names.push("fromYScale");
    attr_names.push("toYScale");
    attr_names.push("fromXDelta");
    attr_names.push("toXDelta");
    attr_names.push("fromYDelta");
    attr_names.push("toYDelta");
    attr_names.push("fromAlpha");
    attr_names.push("toAlpha");
    attr_names.push("delay");
    attr_names.push("animation");
    attr_names.push("animationOrder");
    attr_names.push("columnDelay");
    attr_names.push("rowDelay");
    attr_names.push("direction");
    attr_names.push("directionPriority");
    attr_names.push("factor");
    attr_names.push("cycles");
    attr_names.push("searchMode");
    attr_names.push("searchSuggestAuthority");
    attr_names.push("searchSuggestPath");
    attr_names.push("searchSuggestSelection");
    attr_names.push("searchSuggestIntentAction");
    attr_names.push("searchSuggestIntentData");
    attr_names.push("queryActionMsg");
    attr_names.push("suggestActionMsg");
    attr_names.push("suggestActionMsgColumn");
    attr_names.push("menuCategory");
    attr_names.push("orderInCategory");
    attr_names.push("checkableBehavior");
    attr_names.push("title");
    attr_names.push("titleCondensed");
    attr_names.push("alphabeticShortcut");
    attr_names.push("numericShortcut");
    attr_names.push("checkable");
    attr_names.push("selectable");
    attr_names.push("orderingFromXml");
    attr_names.push("key");
    attr_names.push("summary");
    attr_names.push("order");
    attr_names.push("widgetLayout");
    attr_names.push("dependency");
    attr_names.push("defaultValue");
    attr_names.push("shouldDisableView");
    attr_names.push("summaryOn");
    attr_names.push("summaryOff");
    attr_names.push("disableDependentsState");
    attr_names.push("dialogTitle");
    attr_names.push("dialogMessage");
    attr_names.push("dialogIcon");
    attr_names.push("positiveButtonText");
    attr_names.push("negativeButtonText");
    attr_names.push("dialogLayout");
    attr_names.push("entryValues");
    attr_names.push("ringtoneType");
    attr_names.push("showDefault");
    attr_names.push("showSilent");
    attr_names.push("scaleWidth");
    attr_names.push("scaleHeight");
    attr_names.push("scaleGravity");
    attr_names.push("ignoreGravity");
    attr_names.push("foregroundGravity");
    attr_names.push("tileMode");
    attr_names.push("targetActivity");
    attr_names.push("alwaysRetainTaskState");
    attr_names.push("allowTaskReparenting");
    attr_names.push("searchButtonText");
    attr_names.push("colorForegroundInverse");
    attr_names.push("textAppearanceButton");
    attr_names.push("listSeparatorTextViewStyle");
    attr_names.push("streamType");
    attr_names.push("clipOrientation");
    attr_names.push("centerColor");
    attr_names.push("minSdkVersion");
    attr_names.push("windowFullscreen");
    attr_names.push("unselectedAlpha");
    attr_names.push("progressBarStyleSmallTitle");
    attr_names.push("ratingBarStyleIndicator");
    attr_names.push("apiKey");
    attr_names.push("textColorTertiary");
    attr_names.push("textColorTertiaryInverse");
    attr_names.push("listDivider");
    attr_names.push("soundEffectsEnabled");
    attr_names.push("keepScreenOn");
    attr_names.push("lineSpacingExtra");
    attr_names.push("lineSpacingMultiplier");
    attr_names.push("listChoiceIndicatorSingle");
    attr_names.push("listChoiceIndicatorMultiple");
    attr_names.push("versionCode");
    attr_names.push("versionName");
    attr_names.push("marqueeRepeatLimit");
    attr_names.push("windowNoDisplay");
    attr_names.push("backgroundDimEnabled");
    attr_names.push("inputType");
    attr_names.push("isDefault");
    attr_names.push("windowDisablePreview");
    attr_names.push("privateImeOptions");
    attr_names.push("editorExtras");
    attr_names.push("settingsActivity");
    attr_names.push("fastScrollEnabled");
    attr_names.push("reqTouchScreen");
    attr_names.push("reqKeyboardType");
    attr_names.push("reqHardKeyboard");
    attr_names.push("reqNavigation");
    attr_names.push("windowSoftInputMode");
    attr_names.push("imeFullscreenBackground");
    attr_names.push("noHistory");
    attr_names.push("headerDividersEnabled");
    attr_names.push("footerDividersEnabled");
    attr_names.push("candidatesTextStyleSpans");
    attr_names.push("smoothScrollbar");
    attr_names.push("reqFiveWayNav");
    attr_names.push("keyBackground");
    attr_names.push("keyTextSize");
    attr_names.push("labelTextSize");
    attr_names.push("keyTextColor");
    attr_names.push("keyPreviewLayout");
    attr_names.push("keyPreviewOffset");
    attr_names.push("keyPreviewHeight");
    attr_names.push("verticalCorrection");
    attr_names.push("popupLayout");
    attr_names.push("state_long_pressable");
    attr_names.push("keyWidth");
    attr_names.push("keyHeight");
    attr_names.push("horizontalGap");
    attr_names.push("verticalGap");
    attr_names.push("rowEdgeFlags");
    attr_names.push("codes");
    attr_names.push("popupKeyboard");
    attr_names.push("popupCharacters");
    attr_names.push("keyEdgeFlags");
    attr_names.push("isModifier");
    attr_names.push("isSticky");
    attr_names.push("isRepeatable");
    attr_names.push("iconPreview");
    attr_names.push("keyOutputText");
    attr_names.push("keyLabel");
    attr_names.push("keyIcon");
    attr_names.push("keyboardMode");
    attr_names.push("isScrollContainer");
    attr_names.push("fillEnabled");
    attr_names.push("updatePeriodMillis");
    attr_names.push("initialLayout");
    attr_names.push("voiceSearchMode");
    attr_names.push("voiceLanguageModel");
    attr_names.push("voicePromptText");
    attr_names.push("voiceLanguage");
    attr_names.push("voiceMaxResults");
    attr_names.push("bottomOffset");
    attr_names.push("topOffset");
    attr_names.push("allowSingleTap");
    attr_names.push("handle");
    attr_names.push("content");
    attr_names.push("animateOnClick");
    attr_names.push("configure");
    attr_names.push("hapticFeedbackEnabled");
    attr_names.push("innerRadius");
    attr_names.push("thickness");
    attr_names.push("sharedUserLabel");
    attr_names.push("dropDownWidth");
    attr_names.push("dropDownAnchor");
    attr_names.push("imeOptions");
    attr_names.push("imeActionLabel");
    attr_names.push("imeActionId");
    attr_names.push("UNKNOWN");
    attr_names.push("imeExtractEnterAnimation");
    attr_names.push("imeExtractExitAnimation");
    attr_names.push("tension");
    attr_names.push("extraTension");
    attr_names.push("anyDensity");
    attr_names.push("searchSuggestThreshold");
    attr_names.push("includeInGlobalSearch");
    attr_names.push("onClick");
    attr_names.push("targetSdkVersion");
    attr_names.push("maxSdkVersion");
    attr_names.push("testOnly");
    attr_names.push("contentDescription");
    attr_names.push("gestureStrokeWidth");
    attr_names.push("gestureColor");
    attr_names.push("uncertainGestureColor");
    attr_names.push("fadeOffset");
    attr_names.push("fadeDuration");
    attr_names.push("gestureStrokeType");
    attr_names.push("gestureStrokeLengthThreshold");
    attr_names.push("gestureStrokeSquarenessThreshold");
    attr_names.push("gestureStrokeAngleThreshold");
    attr_names.push("eventsInterceptionEnabled");
    attr_names.push("fadeEnabled");
    attr_names.push("backupAgent");
    attr_names.push("allowBackup");
    attr_names.push("glEsVersion");
    attr_names.push("queryAfterZeroResults");
    attr_names.push("dropDownHeight");
    attr_names.push("smallScreens");
    attr_names.push("normalScreens");
    attr_names.push("largeScreens");
    attr_names.push("progressBarStyleInverse");
    attr_names.push("progressBarStyleSmallInverse");
    attr_names.push("progressBarStyleLargeInverse");
    attr_names.push("searchSettingsDescription");
    attr_names.push("textColorPrimaryInverseDisableOnly");
    attr_names.push("autoUrlDetect");
    attr_names.push("resizeable");
    attr_names.push("required");
    attr_names.push("accountType");
    attr_names.push("contentAuthority");
    attr_names.push("userVisible");
    attr_names.push("windowShowWallpaper");
    attr_names.push("wallpaperOpenEnterAnimation");
    attr_names.push("wallpaperOpenExitAnimation");
    attr_names.push("wallpaperCloseEnterAnimation");
    attr_names.push("wallpaperCloseExitAnimation");
    attr_names.push("wallpaperIntraOpenEnterAnimation");
    attr_names.push("wallpaperIntraOpenExitAnimation");
    attr_names.push("wallpaperIntraCloseEnterAnimation");
    attr_names.push("wallpaperIntraCloseExitAnimation");
    attr_names.push("supportsUploading");
    attr_names.push("killAfterRestore");
    attr_names.push("restoreNeedsApplication");
    attr_names.push("smallIcon");
    attr_names.push("accountPreferences");
    attr_names.push("textAppearanceSearchResultSubtitle");
    attr_names.push("textAppearanceSearchResultTitle");
    attr_names.push("summaryColumn");
    attr_names.push("detailColumn");
    attr_names.push("detailSocialSummary");
    attr_names.push("thumbnail");
    attr_names.push("detachWallpaper");
    attr_names.push("finishOnCloseSystemDialogs");
    attr_names.push("scrollbarFadeDuration");
    attr_names.push("scrollbarDefaultDelayBeforeFade");
    attr_names.push("fadeScrollbars");
    attr_names.push("colorBackgroundCacheHint");
    attr_names.push("dropDownHorizontalOffset");
    attr_names.push("dropDownVerticalOffset");
    attr_names.push("quickContactBadgeStyleWindowSmall");
    attr_names.push("quickContactBadgeStyleWindowMedium");
    attr_names.push("quickContactBadgeStyleWindowLarge");
    attr_names.push("quickContactBadgeStyleSmallWindowSmall");
    attr_names.push("quickContactBadgeStyleSmallWindowMedium");
    attr_names.push("quickContactBadgeStyleSmallWindowLarge");
    attr_names.push("author");
    attr_names.push("autoStart");
    attr_names.push("expandableListViewWhiteStyle");
    attr_names.push("installLocation");
    attr_names.push("vmSafeMode");
    attr_names.push("webTextViewStyle");
    attr_names.push("restoreAnyVersion");
    attr_names.push("tabStripLeft");
    attr_names.push("tabStripRight");
    attr_names.push("tabStripEnabled");
    attr_names.push("logo");
    attr_names.push("xlargeScreens");
    attr_names.push("immersive");
    attr_names.push("overScrollMode");
    attr_names.push("overScrollHeader");
    attr_names.push("overScrollFooter");
    attr_names.push("filterTouchesWhenObscured");
    attr_names.push("textSelectHandleLeft");
    attr_names.push("textSelectHandleRight");
    attr_names.push("textSelectHandle");
    attr_names.push("textSelectHandleWindowStyle");
    attr_names.push("popupAnimationStyle");
    attr_names.push("screenSize");
    attr_names.push("screenDensity");
    attr_names.push("allContactsName");
    attr_names.push("windowActionBar");
    attr_names.push("actionBarStyle");
    attr_names.push("navigationMode");
    attr_names.push("displayOptions");
    attr_names.push("subtitle");
    attr_names.push("customNavigationLayout");
    attr_names.push("hardwareAccelerated");
    attr_names.push("measureWithLargestChild");
    attr_names.push("animateFirstView");
    attr_names.push("dropDownSpinnerStyle");
    attr_names.push("actionDropDownStyle");
    attr_names.push("actionButtonStyle");
    attr_names.push("showAsAction");
    attr_names.push("previewImage");
    attr_names.push("actionModeBackground");
    attr_names.push("actionModeCloseDrawable");
    attr_names.push("windowActionModeOverlay");
    attr_names.push("valueFrom");
    attr_names.push("valueTo");
    attr_names.push("valueType");
    attr_names.push("propertyName");
    attr_names.push("ordering");
    attr_names.push("fragment");
    attr_names.push("windowActionBarOverlay");
    attr_names.push("fragmentOpenEnterAnimation");
    attr_names.push("fragmentOpenExitAnimation");
    attr_names.push("fragmentCloseEnterAnimation");
    attr_names.push("fragmentCloseExitAnimation");
    attr_names.push("fragmentFadeEnterAnimation");
    attr_names.push("fragmentFadeExitAnimation");
    attr_names.push("actionBarSize");
    attr_names.push("imeSubtypeLocale");
    attr_names.push("imeSubtypeMode");
    attr_names.push("imeSubtypeExtraValue");
    attr_names.push("splitMotionEvents");
    attr_names.push("listChoiceBackgroundIndicator");
    attr_names.push("spinnerMode");
    attr_names.push("animateLayoutChanges");
    attr_names.push("actionBarTabStyle");
    attr_names.push("actionBarTabBarStyle");
    attr_names.push("actionBarTabTextStyle");
    attr_names.push("actionOverflowButtonStyle");
    attr_names.push("actionModeCloseButtonStyle");
    attr_names.push("titleTextStyle");
    attr_names.push("subtitleTextStyle");
    attr_names.push("iconifiedByDefault");
    attr_names.push("actionLayout");
    attr_names.push("actionViewClass");
    attr_names.push("activatedBackgroundIndicator");
    attr_names.push("state_activated");
    attr_names.push("listPopupWindowStyle");
    attr_names.push("popupMenuStyle");
    attr_names.push("textAppearanceLargePopupMenu");
    attr_names.push("textAppearanceSmallPopupMenu");
    attr_names.push("breadCrumbTitle");
    attr_names.push("breadCrumbShortTitle");
    attr_names.push("listDividerAlertDialog");
    attr_names.push("textColorAlertDialogListItem");
    attr_names.push("loopViews");
    attr_names.push("dialogTheme");
    attr_names.push("alertDialogTheme");
    attr_names.push("dividerVertical");
    attr_names.push("homeAsUpIndicator");
    attr_names.push("enterFadeDuration");
    attr_names.push("exitFadeDuration");
    attr_names.push("selectableItemBackground");
    attr_names.push("autoAdvanceViewId");
    attr_names.push("useIntrinsicSizeAsMinimum");
    attr_names.push("actionModeCutDrawable");
    attr_names.push("actionModeCopyDrawable");
    attr_names.push("actionModePasteDrawable");
    attr_names.push("textEditPasteWindowLayout");
    attr_names.push("textEditNoPasteWindowLayout");
    attr_names.push("textIsSelectable");
    attr_names.push("windowEnableSplitTouch");
    attr_names.push("indeterminateProgressStyle");
    attr_names.push("progressBarPadding");
    attr_names.push("animationResolution");
    attr_names.push("state_accelerated");
    attr_names.push("baseline");
    attr_names.push("homeLayout");
    attr_names.push("opacity");
    attr_names.push("alpha");
    attr_names.push("transformPivotX");
    attr_names.push("transformPivotY");
    attr_names.push("translationX");
    attr_names.push("translationY");
    attr_names.push("scaleX");
    attr_names.push("scaleY");
    attr_names.push("rotation");
    attr_names.push("rotationX");
    attr_names.push("rotationY");
    attr_names.push("showDividers");
    attr_names.push("dividerPadding");
    attr_names.push("borderlessButtonStyle");
    attr_names.push("dividerHorizontal");
    attr_names.push("itemPadding");
    attr_names.push("buttonBarStyle");
    attr_names.push("buttonBarButtonStyle");
    attr_names.push("segmentedButtonStyle");
    attr_names.push("staticWallpaperPreview");
    attr_names.push("allowParallelSyncs");
    attr_names.push("isAlwaysSyncable");
    attr_names.push("verticalScrollbarPosition");
    attr_names.push("fastScrollAlwaysVisible");
    attr_names.push("fastScrollThumbDrawable");
    attr_names.push("fastScrollPreviewBackgroundLeft");
    attr_names.push("fastScrollPreviewBackgroundRight");
    attr_names.push("fastScrollTrackDrawable");
    attr_names.push("fastScrollOverlayPosition");
    attr_names.push("customTokens");
    attr_names.push("nextFocusForward");
    attr_names.push("firstDayOfWeek");
    attr_names.push("showWeekNumber");
    attr_names.push("minDate");
    attr_names.push("maxDate");
    attr_names.push("shownWeekCount");
    attr_names.push("selectedWeekBackgroundColor");
    attr_names.push("focusedMonthDateColor");
    attr_names.push("unfocusedMonthDateColor");
    attr_names.push("weekNumberColor");
    attr_names.push("weekSeparatorLineColor");
    attr_names.push("selectedDateVerticalBar");
    attr_names.push("weekDayTextAppearance");
    attr_names.push("dateTextAppearance");
    attr_names.push("UNKNOWN");
    attr_names.push("spinnersShown");
    attr_names.push("calendarViewShown");
    attr_names.push("state_multiline");
    attr_names.push("detailsElementBackground");
    attr_names.push("textColorHighlightInverse");
    attr_names.push("textColorLinkInverse");
    attr_names.push("editTextColor");
    attr_names.push("editTextBackground");
    attr_names.push("horizontalScrollViewStyle");
    attr_names.push("layerType");
    attr_names.push("alertDialogIcon");
    attr_names.push("windowMinWidthMajor");
    attr_names.push("windowMinWidthMinor");
    attr_names.push("queryHint");
    attr_names.push("fastScrollTextColor");
    attr_names.push("largeHeap");
    attr_names.push("windowCloseOnTouchOutside");
    attr_names.push("datePickerStyle");
    attr_names.push("calendarViewStyle");
    attr_names.push("textEditSidePasteWindowLayout");
    attr_names.push("textEditSideNoPasteWindowLayout");
    attr_names.push("actionMenuTextAppearance");
    attr_names.push("actionMenuTextColor");
    attr_names.push("textCursorDrawable");
    attr_names.push("resizeMode");
    attr_names.push("requiresSmallestWidthDp");
    attr_names.push("compatibleWidthLimitDp");
    attr_names.push("largestWidthLimitDp");
    attr_names.push("state_hovered");
    attr_names.push("state_drag_can_accept");
    attr_names.push("state_drag_hovered");
    attr_names.push("stopWithTask");
    attr_names.push("switchTextOn");
    attr_names.push("switchTextOff");
    attr_names.push("switchPreferenceStyle");
    attr_names.push("switchTextAppearance");
    attr_names.push("track");
    attr_names.push("switchMinWidth");
    attr_names.push("switchPadding");
    attr_names.push("thumbTextPadding");
    attr_names.push("textSuggestionsWindowStyle");
    attr_names.push("textEditSuggestionItemLayout");
    attr_names.push("rowCount");
    attr_names.push("rowOrderPreserved");
    attr_names.push("columnCount");
    attr_names.push("columnOrderPreserved");
    attr_names.push("useDefaultMargins");
    attr_names.push("alignmentMode");
    attr_names.push("layout_row");
    attr_names.push("layout_rowSpan");
    attr_names.push("layout_columnSpan");
    attr_names.push("actionModeSelectAllDrawable");
    attr_names.push("isAuxiliary");
    attr_names.push("accessibilityEventTypes");
    attr_names.push("packageNames");
    attr_names.push("accessibilityFeedbackType");
    attr_names.push("notificationTimeout");
    attr_names.push("accessibilityFlags");
    attr_names.push("canRetrieveWindowContent");
    attr_names.push("listPreferredItemHeightLarge");
    attr_names.push("listPreferredItemHeightSmall");
    attr_names.push("actionBarSplitStyle");
    attr_names.push("actionProviderClass");
    attr_names.push("backgroundStacked");
    attr_names.push("backgroundSplit");
    attr_names.push("textAllCaps");
    attr_names.push("colorPressedHighlight");
    attr_names.push("colorLongPressedHighlight");
    attr_names.push("colorFocusedHighlight");
    attr_names.push("colorActivatedHighlight");
    attr_names.push("colorMultiSelectHighlight");
    attr_names.push("drawableStart");
    attr_names.push("drawableEnd");
    attr_names.push("actionModeStyle");
    attr_names.push("minResizeWidth");
    attr_names.push("minResizeHeight");
    attr_names.push("actionBarWidgetTheme");
    attr_names.push("uiOptions");
    attr_names.push("subtypeLocale");
    attr_names.push("subtypeExtraValue");
    attr_names.push("actionBarDivider");
    attr_names.push("actionBarItemBackground");
    attr_names.push("actionModeSplitBackground");
    attr_names.push("textAppearanceListItem");
    attr_names.push("textAppearanceListItemSmall");
    attr_names.push("targetDescriptions");
    attr_names.push("directionDescriptions");
    attr_names.push("overridesImplicitlyEnabledSubtype");
    attr_names.push("listPreferredItemPaddingLeft");
    attr_names.push("listPreferredItemPaddingRight");
    attr_names.push("requiresFadingEdge");
    attr_names.push("publicKey");
    attr_names.push("parentActivityName");
    attr_names.push("UNKNOWN");
    attr_names.push("isolatedProcess");
    attr_names.push("importantForAccessibility");
    attr_names.push("keyboardLayout");
    attr_names.push("fontFamily");
    attr_names.push("mediaRouteButtonStyle");
    attr_names.push("mediaRouteTypes");
    attr_names.push("supportsRtl");
    attr_names.push("textDirection");
    attr_names.push("textAlignment");
    attr_names.push("layoutDirection");
    attr_names.push("paddingStart");
    attr_names.push("paddingEnd");
    attr_names.push("layout_marginStart");
    attr_names.push("layout_marginEnd");
    attr_names.push("layout_toStartOf");
    attr_names.push("layout_toEndOf");
    attr_names.push("layout_alignStart");
    attr_names.push("layout_alignEnd");
    attr_names.push("layout_alignParentStart");
    attr_names.push("layout_alignParentEnd");
    attr_names.push("listPreferredItemPaddingStart");
    attr_names.push("listPreferredItemPaddingEnd");
    attr_names.push("singleUser");
    attr_names.push("presentationTheme");
    attr_names.push("subtypeId");
    attr_names.push("initialKeyguardLayout");
    attr_names.push("UNKNOWN");
    attr_names.push("widgetCategory");
    attr_names.push("permissionGroupFlags");
    attr_names.push("labelFor");
    attr_names.push("permissionFlags");
    attr_names.push("checkedTextViewStyle");
    attr_names.push("showOnLockScreen");
    attr_names.push("format12Hour");
    attr_names.push("format24Hour");
    attr_names.push("timeZone");
    attr_names.push("mipMap");
    attr_names.push("mirrorForRtl");
    attr_names.push("windowOverscan");
    attr_names.push("requiredForAllUsers");
    attr_names.push("indicatorStart");
    attr_names.push("indicatorEnd");
    attr_names.push("childIndicatorStart");
    attr_names.push("childIndicatorEnd");
    attr_names.push("restrictedAccountType");
    attr_names.push("requiredAccountType");
    attr_names.push("canRequestTouchExplorationMode");
    attr_names.push("canRequestEnhancedWebAccessibility");
    attr_names.push("canRequestFilterKeyEvents");
    attr_names.push("layoutMode");
    attr_names.push("keySet");
    attr_names.push("targetId");
    attr_names.push("fromScene");
    attr_names.push("toScene");
    attr_names.push("transition");
    attr_names.push("transitionOrdering");
    attr_names.push("fadingMode");
    attr_names.push("startDelay");
    attr_names.push("ssp");
    attr_names.push("sspPrefix");
    attr_names.push("sspPattern");
    attr_names.push("addPrintersActivity");
    attr_names.push("vendor");
    attr_names.push("category");
    attr_names.push("isAsciiCapable");
    attr_names.push("autoMirrored");
    attr_names.push("supportsSwitchingToNextInputMethod");
    attr_names.push("requireDeviceUnlock");
    attr_names.push("apduServiceBanner");
    attr_names.push("accessibilityLiveRegion");
    attr_names.push("windowTranslucentStatus");
    attr_names.push("windowTranslucentNavigation");
    attr_names.push("advancedPrintOptionsActivity");
    attr_names.push("banner");
    attr_names.push("windowSwipeToDismiss");
    attr_names.push("isGame");
    attr_names.push("allowEmbedded");
    attr_names.push("setupActivity");
    attr_names.push("fastScrollStyle");
    attr_names.push("windowContentTransitions");
    attr_names.push("windowContentTransitionManager");
    attr_names.push("translationZ");
    attr_names.push("tintMode");
    attr_names.push("controlX1");
    attr_names.push("controlY1");
    attr_names.push("controlX2");
    attr_names.push("controlY2");
    attr_names.push("transitionName");
    attr_names.push("transitionGroup");
    attr_names.push("viewportWidth");
    attr_names.push("viewportHeight");
    attr_names.push("fillColor");
    attr_names.push("pathData");
    attr_names.push("strokeColor");
    attr_names.push("strokeWidth");
    attr_names.push("trimPathStart");
    attr_names.push("trimPathEnd");
    attr_names.push("trimPathOffset");
    attr_names.push("strokeLineCap");
    attr_names.push("strokeLineJoin");
    attr_names.push("strokeMiterLimit");
    attr_names.push("UNKNOWN");
    attr_names.push("UNKNOWN");
    attr_names.push("UNKNOWN");
    attr_names.push("UNKNOWN");
    attr_names.push("UNKNOWN");
    attr_names.push("UNKNOWN");
    attr_names.push("UNKNOWN");
    attr_names.push("UNKNOWN");
    attr_names.push("UNKNOWN");
    attr_names.push("UNKNOWN");
    attr_names.push("UNKNOWN");
    attr_names.push("UNKNOWN");
    attr_names.push("UNKNOWN");
    attr_names.push("UNKNOWN");
    attr_names.push("UNKNOWN");
    attr_names.push("UNKNOWN");
    attr_names.push("UNKNOWN");
    attr_names.push("UNKNOWN");
    attr_names.push("UNKNOWN");
    attr_names.push("UNKNOWN");
    attr_names.push("UNKNOWN");
    attr_names.push("UNKNOWN");
    attr_names.push("UNKNOWN");
    attr_names.push("UNKNOWN");
    attr_names.push("UNKNOWN");
    attr_names.push("UNKNOWN");
    attr_names.push("UNKNOWN");
    attr_names.push("colorControlNormal");
    attr_names.push("colorControlActivated");
    attr_names.push("colorButtonNormal");
    attr_names.push("colorControlHighlight");
    attr_names.push("persistableMode");
    attr_names.push("titleTextAppearance");
    attr_names.push("subtitleTextAppearance");
    attr_names.push("slideEdge");
    attr_names.push("actionBarTheme");
    attr_names.push("textAppearanceListItemSecondary");
    attr_names.push("colorPrimary");
    attr_names.push("colorPrimaryDark");
    attr_names.push("colorAccent");
    attr_names.push("nestedScrollingEnabled");
    attr_names.push("windowEnterTransition");
    attr_names.push("windowExitTransition");
    attr_names.push("windowSharedElementEnterTransition");
    attr_names.push("windowSharedElementExitTransition");
    attr_names.push("windowAllowReturnTransitionOverlap");
    attr_names.push("windowAllowEnterTransitionOverlap");
    attr_names.push("sessionService");
    attr_names.push("stackViewStyle");
    attr_names.push("switchStyle");
    attr_names.push("elevation");
    attr_names.push("excludeId");
    attr_names.push("excludeClass");
    attr_names.push("hideOnContentScroll");
    attr_names.push("actionOverflowMenuStyle");
    attr_names.push("documentLaunchMode");
    attr_names.push("maxRecents");
    attr_names.push("autoRemoveFromRecents");
    attr_names.push("stateListAnimator");
    attr_names.push("toId");
    attr_names.push("fromId");
    attr_names.push("reversible");
    attr_names.push("splitTrack");
    attr_names.push("targetName");
    attr_names.push("excludeName");
    attr_names.push("matchOrder");
    attr_names.push("windowDrawsSystemBarBackgrounds");
    attr_names.push("statusBarColor");
    attr_names.push("navigationBarColor");
    attr_names.push("contentInsetStart");
    attr_names.push("contentInsetEnd");
    attr_names.push("contentInsetLeft");
    attr_names.push("contentInsetRight");
    attr_names.push("paddingMode");
    attr_names.push("layout_rowWeight");
    attr_names.push("layout_columnWeight");
    attr_names.push("translateX");
    attr_names.push("translateY");
    attr_names.push("selectableItemBackgroundBorderless");
    attr_names.push("elegantTextHeight");
    attr_names.push("UNKNOWN");
    attr_names.push("UNKNOWN");
    attr_names.push("UNKNOWN");
    attr_names.push("windowTransitionBackgroundFadeDuration");
    attr_names.push("overlapAnchor");
    attr_names.push("progressTint");
    attr_names.push("progressTintMode");
    attr_names.push("progressBackgroundTint");
    attr_names.push("progressBackgroundTintMode");
    attr_names.push("secondaryProgressTint");
    attr_names.push("secondaryProgressTintMode");
    attr_names.push("indeterminateTint");
    attr_names.push("indeterminateTintMode");
    attr_names.push("backgroundTint");
    attr_names.push("backgroundTintMode");
    attr_names.push("foregroundTint");
    attr_names.push("foregroundTintMode");
    attr_names.push("buttonTint");
    attr_names.push("buttonTintMode");
    attr_names.push("thumbTint");
    attr_names.push("thumbTintMode");
    attr_names.push("fullBackupOnly");
    attr_names.push("propertyXName");
    attr_names.push("propertyYName");
    attr_names.push("relinquishTaskIdentity");
    attr_names.push("tileModeX");
    attr_names.push("tileModeY");
    attr_names.push("actionModeShareDrawable");
    attr_names.push("actionModeFindDrawable");
    attr_names.push("actionModeWebSearchDrawable");
    attr_names.push("transitionVisibilityMode");
    attr_names.push("minimumHorizontalAngle");
    attr_names.push("minimumVerticalAngle");
    attr_names.push("maximumAngle");
    attr_names.push("searchViewStyle");
    attr_names.push("closeIcon");
    attr_names.push("goIcon");
    attr_names.push("searchIcon");
    attr_names.push("voiceIcon");
    attr_names.push("commitIcon");
    attr_names.push("suggestionRowLayout");
    attr_names.push("queryBackground");
    attr_names.push("submitBackground");
    attr_names.push("buttonBarPositiveButtonStyle");
    attr_names.push("buttonBarNeutralButtonStyle");
    attr_names.push("buttonBarNegativeButtonStyle");
    attr_names.push("popupElevation");
    attr_names.push("actionBarPopupTheme");
    attr_names.push("multiArch");
    attr_names.push("touchscreenBlocksFocus");
    attr_names.push("windowElevation");
    attr_names.push("launchTaskBehindTargetAnimation");
    attr_names.push("launchTaskBehindSourceAnimation");
    attr_names.push("restrictionType");
    attr_names.push("dayOfWeekBackground");
    attr_names.push("dayOfWeekTextAppearance");
    attr_names.push("headerMonthTextAppearance");
    attr_names.push("headerDayOfMonthTextAppearance");
    attr_names.push("headerYearTextAppearance");
    attr_names.push("yearListItemTextAppearance");
    attr_names.push("yearListSelectorColor");
    attr_names.push("calendarTextColor");
    attr_names.push("recognitionService");
    attr_names.push("timePickerStyle");
    attr_names.push("timePickerDialogTheme");
    attr_names.push("headerTimeTextAppearance");
    attr_names.push("headerAmPmTextAppearance");
    attr_names.push("numbersTextColor");
    attr_names.push("numbersBackgroundColor");
    attr_names.push("numbersSelectorColor");
    attr_names.push("amPmTextColor");
    attr_names.push("amPmBackgroundColor");
    attr_names.push("UNKNOWN");
    attr_names.push("checkMarkTint");
    attr_names.push("checkMarkTintMode");
    attr_names.push("popupTheme");
    attr_names.push("toolbarStyle");
    attr_names.push("windowClipToOutline");
    attr_names.push("datePickerDialogTheme");
    attr_names.push("showText");
    attr_names.push("windowReturnTransition");
    attr_names.push("windowReenterTransition");
    attr_names.push("windowSharedElementReturnTransition");
    attr_names.push("windowSharedElementReenterTransition");
    attr_names.push("resumeWhilePausing");
    attr_names.push("datePickerMode");
    attr_names.push("timePickerMode");
    attr_names.push("inset");
    attr_names.push("letterSpacing");
    attr_names.push("fontFeatureSettings");
    attr_names.push("outlineProvider");
    attr_names.push("contentAgeHint");
    attr_names.push("country");
    attr_names.push("windowSharedElementsUseOverlay");
    attr_names.push("reparent");
    attr_names.push("reparentWithOverlay");
    attr_names.push("ambientShadowAlpha");
    attr_names.push("spotShadowAlpha");
    attr_names.push("navigationIcon");
    attr_names.push("navigationContentDescription");
    attr_names.push("fragmentExitTransition");
    attr_names.push("fragmentEnterTransition");
    attr_names.push("fragmentSharedElementEnterTransition");
    attr_names.push("fragmentReturnTransition");
    attr_names.push("fragmentSharedElementReturnTransition");
    attr_names.push("fragmentReenterTransition");
    attr_names.push("fragmentAllowEnterTransitionOverlap");
    attr_names.push("fragmentAllowReturnTransitionOverlap");
    attr_names.push("patternPathData");
    attr_names.push("strokeAlpha");
    attr_names.push("fillAlpha");
    attr_names.push("windowActivityTransitions");
    attr_names.push("colorEdgeEffect");
    attr_names.push("resizeClip");
    attr_names.push("collapseContentDescription");
    attr_names.push("accessibilityTraversalBefore");
    attr_names.push("accessibilityTraversalAfter");
    attr_names.push("dialogPreferredPadding");
    attr_names.push("searchHintIcon");
    attr_names.push("revisionCode");
    attr_names.push("drawableTint");
    attr_names.push("drawableTintMode");
    attr_names.push("fraction");
    attr_names.push("trackTint");
    attr_names.push("trackTintMode");
    attr_names.push("start");
    attr_names.push("end");
    attr_names.push("breakStrategy");
    attr_names.push("hyphenationFrequency");
    attr_names.push("allowUndo");
    attr_names.push("windowLightStatusBar");
    attr_names.push("numbersInnerTextColor");
    attr_names.push("colorBackgroundFloating");
    attr_names.push("titleTextColor");
    attr_names.push("subtitleTextColor");
    attr_names.push("thumbPosition");
    attr_names.push("scrollIndicators");
    attr_names.push("contextClickable");
    attr_names.push("fingerprintAuthDrawable");
    attr_names.push("logoDescription");
    attr_names.push("extractNativeLibs");
    attr_names.push("fullBackupContent");
    attr_names.push("usesCleartextTraffic");
    attr_names.push("lockTaskMode");
    attr_names.push("autoVerify");
    attr_names.push("showForAllUsers");
    attr_names.push("supportsAssist");
    attr_names.push("supportsLaunchVoiceAssistFromKeyguard");
    attr_names.push("listMenuViewStyle");
    attr_names.push("subMenuArrow");
    attr_names.push("defaultWidth");
    attr_names.push("defaultHeight");
    attr_names.push("resizeableActivity");
    attr_names.push("supportsPictureInPicture");
    attr_names.push("titleMargin");
    attr_names.push("titleMarginStart");
    attr_names.push("titleMarginEnd");
    attr_names.push("titleMarginTop");
    attr_names.push("titleMarginBottom");
    attr_names.push("maxButtonHeight");
    attr_names.push("buttonGravity");
    attr_names.push("collapseIcon");
    attr_names.push("level");
    attr_names.push("contextPopupMenuStyle");
    attr_names.push("textAppearancePopupMenuHeader");
    attr_names.push("windowBackgroundFallback");
    attr_names.push("defaultToDeviceProtectedStorage");
    attr_names.push("directBootAware");
    attr_names.push("preferenceFragmentStyle");
    attr_names.push("canControlMagnification");
    attr_names.push("languageTag");
    attr_names.push("pointerIcon");
    attr_names.push("tickMark");
    attr_names.push("tickMarkTint");
    attr_names.push("tickMarkTintMode");
    attr_names.push("canPerformGestures");
    attr_names.push("externalService");
    attr_names.push("supportsLocalInteraction");
    attr_names.push("startX");
    attr_names.push("startY");
    attr_names.push("endX");
    attr_names.push("endY");
    attr_names.push("offset");
    attr_names.push("use32bitAbi");
    attr_names.push("bitmap");
    attr_names.push("hotSpotX");
    attr_names.push("hotSpotY");
    attr_names.push("version");
    attr_names.push("backupInForeground");
    attr_names.push("countDown");
    attr_names.push("canRecord");
    attr_names.push("tunerCount");
    attr_names.push("fillType");
    attr_names.push("popupEnterTransition");
    attr_names.push("popupExitTransition");
    attr_names.push("forceHasOverlappingRendering");
    attr_names.push("contentInsetStartWithNavigation");
    attr_names.push("contentInsetEndWithActions");
    attr_names.push("numberPickerStyle");
    attr_names.push("enableVrMode");
    attr_names.push("UNKNOWN");
    attr_names.push("networkSecurityConfig");
    attr_names.push("shortcutId");
    attr_names.push("shortcutShortLabel");
    attr_names.push("shortcutLongLabel");
    attr_names.push("shortcutDisabledMessage");
    attr_names.push("roundIcon");
    attr_names.push("contextUri");
    attr_names.push("contextDescription");
    attr_names.push("showMetadataInPreview");
    attr_names.push("colorSecondary");

    // For now, we only care about the attribute names.
    id -= 0x1010000;

    Ok(attr_names[id as usize].to_string())
}
