#![allow(non_snake_case, unused_variables, dead_code)]

use std::{
    fs,
    collections::HashMap,
};
use std::io::{
    Write,
    Cursor,
};

use quick_xml::Writer;

use axml_parser::create_cursor;
use axml_parser::chunk_header::ChunkHeader;
use axml_parser::resource_map::ResourceMap;
use axml_parser::res_table::{
    ResTable,
    ResTablePackage
};
use axml_parser::string_pool::StringPool;
use axml_parser::xml_types::XmlTypes;
use axml_parser::parser;
use axml_parser::cli;

fn main() {
    // Check CLI arguments
    let args = cli::parse_args();

    // Check the file type
    let arg_type = args.get_arg_type();
    let arg_path = args.get_arg_path();

    // Create cursor over input file contents
    let mut axml_cursor = create_cursor(arg_type, &arg_path);

    let mut global_strings = Vec::new();
    let mut namespace_prefixes = HashMap::<String, String>::new();
    let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 2);

    // Now parsing the rest of the file
    loop {
        let block_type = XmlTypes::parse_block_type(&mut axml_cursor);
        let block_type = match block_type {
            Ok(block) => block,
            Err(e) => break,
        };

        match block_type {
            XmlTypes::ResNullType => continue,
            XmlTypes::ResStringPoolType => {
                let _foo = StringPool::from_buff(&mut axml_cursor, &mut global_strings)
                                       .expect("Error: cannot parse string pool header");
            },
            XmlTypes::ResTableType => {
                ResTable::parse(&mut axml_cursor); // .expect("Error: cannot parse resource table");
            },
            XmlTypes::ResXmlType => {
                // TODO: should we do something more here?
                /* Go back 2 bytes, to account from the block type */
                let initial_offset = axml_cursor.position();
                axml_cursor.set_position(initial_offset - 2);

                let _ = ChunkHeader::from_buff(&mut axml_cursor, XmlTypes::ResXmlType)
                        .expect("Error: cannot parse AXML header");
            },
            XmlTypes::ResXmlStartNamespaceType => {
                parser::parse_start_namespace(&mut axml_cursor, &global_strings, &mut namespace_prefixes);
            },
            XmlTypes::ResXmlEndNamespaceType => {
                parser::parse_end_namespace(&mut axml_cursor, &global_strings);
            },
            XmlTypes::ResXmlStartElementType => {
                let (element_name, attrs) = parser::parse_start_element(&mut axml_cursor, &global_strings, &namespace_prefixes).unwrap();
                parser::handle_event(&mut writer, element_name, attrs, &namespace_prefixes, XmlTypes::ResXmlStartElementType);
            },
            XmlTypes::ResXmlEndElementType => {
                let element_name = parser::parse_end_element(&mut axml_cursor, &global_strings).unwrap();
                parser::handle_event(&mut writer, element_name, Vec::new(), &namespace_prefixes, XmlTypes::ResXmlEndElementType);
            },
            XmlTypes::ResXmlCDataType => panic!("TODO: RES_XML_CDATA_TYPE"),
            XmlTypes::ResXmlLastChunkType => panic!("TODO: RES_XML_LAST_CHUNK_TYPE"),

            XmlTypes::ResXmlResourceMapType => {
                let resource_map = ResourceMap::from_buff(&mut axml_cursor)
                                                .expect("Error: cannot parse resource map");
            },

            XmlTypes::ResTablePackageType => {
                let chunk = ResTablePackage::parse(&mut axml_cursor);
                println!("chunk: {:#?}", chunk);
                panic!("TODO: RES_TABLE_PACKAGE_TYPE");
            },
            XmlTypes::ResTableTypeType => panic!("TODO: RES_TABLE_TYPE_TYPE"),
            XmlTypes::ResTableTypeSpecType => panic!("TODO: RES_TABLE_TYPE_SPEC_TYPE"),
            XmlTypes::ResTableLibraryType => panic!("TODO: RES_TABLE_LIBRARY_TYPE"),
        }
    }

    let result = writer.into_inner().into_inner();
    let str_result = String::from_utf8(result).unwrap();

    if args.output.is_some() {
        let mut file = fs::File::create(&args.output.unwrap()).unwrap();
        file.write_all(str_result.as_bytes()).unwrap();
    } else {
        println!("{str_result}");
    }
}

