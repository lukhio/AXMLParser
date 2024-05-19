#![allow(dead_code)]

use crate::chunk_header::ChunkHeader;
use crate::xml_types::XmlTypes;

use std::io::{
    Read,
    Error,
    Cursor,
};
use byteorder::{
    LittleEndian,
    ReadBytesExt
};

/**
 * Header of a chunk representing a pool of strings
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
#[derive(Debug)]
pub struct StringPool {
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

    /** FROM GOOGLE
     * Strings in UTF-8 format have length indicated by a length encoded in the
     * stored data. It is either 1 or 2 characters of length data. This allows a
     * maximum length of 0x7FFF (32767 bytes), but you should consider storing
     * text in another way if you're using that much data in a single string.
     *
     * If the high bit is set, then there are two characters or 2 bytes of length
     * data encoded. In that case, drop the high bit of the first character and
     * add it together with the next character.
     */
    fn decode_length_utf8(axml_buff: &mut Cursor<Vec<u8>>) -> u32 {
        let mut size = axml_buff.read_u8().unwrap() as u32;
        if (size & 0x80) != 0 {
            let size_b2 = axml_buff.read_u8().unwrap() as u32;
            size = ((size & 0x7F) << 8) | size_b2;
        }

        size
    }

    /** FROM GOOGLE
     * Strings in UTF-16 format have length indicated by a length encoded in the
     * stored data. It is either 1 or 2 characters of length data. This allows a
     * maximum length of 0x7FFFFFF (2147483647 bytes), but if you're storing that
     * much data in a string, you're abusing them.
     *
     * If the high bit is set, then there are two characters or 4 bytes of length
     * data encoded. In that case, drop the high bit of the first character and
     * add it together with the next character.
     */
    fn decode_length_utf16(axml_buff: &mut Cursor<Vec<u8>>) -> u32 {
        let mut size = axml_buff.read_u16::<LittleEndian>().unwrap() as u32;
        if (size & 0x8000) != 0 {
            let size_b2 = axml_buff.read_u16::<LittleEndian>().unwrap() as u32;
            size = ((size & 0x7FFF) << 16) | size_b2;
        }

        size
    }

    pub fn from_buff(axml_buff: &mut Cursor<Vec<u8>>,
                     global_strings: &mut Vec<String>) -> Result<Self, Error> {

        /* Go back 2 bytes, to account from the block type */
        let initial_offset = axml_buff.position() - 2;
        axml_buff.set_position(initial_offset);
        let initial_offset = initial_offset as u32;

        /* Parse chunk header */
        let header = ChunkHeader::from_buff(axml_buff, XmlTypes::ResStringPoolType)
                     .expect("Error: cannot get chunk header from string pool");
        header.print();

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
        for offset in strings_offsets.iter() {
            let current_start = (initial_offset + strings_start + offset) as u64;
            axml_buff.set_position(current_start);

            let str_size = 0;
            let decoded_string;

            if is_utf8 {
                let u16len = Self::decode_length_utf8(axml_buff);
                let u8len = Self::decode_length_utf8(axml_buff);
                let mut str_buff = Vec::with_capacity((u8len) as usize);
                let mut chunk = axml_buff.take((u8len).into());

                chunk.read_to_end(&mut str_buff).unwrap();
                decoded_string = String::from_utf8(str_buff).unwrap();
            } else {
                let u16len = Self::decode_length_utf16(axml_buff);
                let iter = (0..u16len as usize)
                        .map(|_| axml_buff.read_u16::<LittleEndian>().unwrap());
                decoded_string = std::char::decode_utf16(iter).collect::<Result<String, _>>().unwrap();
            }

            if str_size > 0 {
                global_strings.push(decoded_string);
            }
        }
        let strings = global_strings.to_vec();

        /* Build and return the object */
        Ok(StringPool {
            header,
            string_count,
            style_count,
            flags,
            is_utf8,
            strings_start,
            styles_start,
            strings_offsets,
            styles_offsets,
            strings
        })
    }

    pub fn print(&self) {
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
