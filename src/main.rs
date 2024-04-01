#![allow(non_snake_case, unused_variables, dead_code)]

use std::{
    env,
    fs,
    process::exit,
    collections::HashMap,
};
use std::io::{
    Read,
    Write,
    Error,
    Cursor,
};

use quick_xml::Writer;

use axml_parser::chunk_header::ChunkHeader;
use axml_parser::resource_map::ResourceMap;
use axml_parser::res_table::{
    ResTable,
    ResTablePackage
};
use axml_parser::string_pool::StringPool;
use axml_parser::xml_types::XmlTypes;
use axml_parser::parser;

fn main() {
    /* Check CLI arguments */
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 || args.len() > 3 {
        println!("Usage: ./{:} [AXML|APK]", args[0]);
        exit(22);
    }

    let fpath = &args[1];

    /* The argument can be either an binary XML file, or an APK.
     * If we are dealing with an APK, we must first extract the binary XML from it */
    let mut raw_file;
    let mut axml_vec_buff = Vec::new();
    if fpath == "resources.arsc" || fpath.ends_with(".xml") {
        raw_file = fs::File::open(fpath).expect("Error: cannot open AXML file");
        raw_file.read_to_end(&mut axml_vec_buff).expect("Error: cannot read AXML file");
    } else {
        let zipfile = std::fs::File::open(fpath).unwrap();
        let mut archive = zip::ZipArchive::new(zipfile).unwrap();
        let mut file = match archive.by_name("AndroidManifest.xml") {
            Ok(file) => file,
            Err(..) => {
                panic!("Error: cannot find AndroidManifest.xml in APK");
            }
        };

        file.read_to_end(&mut axml_vec_buff).expect("Error: cannot read AXML file");
    }

    let mut axml_buff = Cursor::new(axml_vec_buff);

    /* Now parsing the rest of the file */
    let mut global_strings = Vec::new();
    let mut namespace_prefixes = HashMap::<String, String>::new();

    /* Output stuff */
    let mut writer = Writer::new(Cursor::new(Vec::new()));

    loop {
        let block_type = XmlTypes::parse_block_type(&mut axml_buff);
        let block_type = match block_type {
            Ok(block) => block,
            Err(e) => break,
        };

        println!("BLOCK TYPE: {:#02X}", block_type);
        match block_type {
            XmlTypes::ResNullType => continue,
            XmlTypes::ResStringPoolType => {
                let foo = StringPool::from_buff(&mut axml_buff, &mut global_strings)
                                      .expect("Error: cannot parse string pool header");
                println!("===================");
                println!("foo: {:?}", foo);
                println!("===================");
                print!("fooprint() ");
                foo.print();
                println!("===================");
                // panic!(".");
            },
            XmlTypes::ResTableType => {
                println!("HEREHERE");
                ResTable::parse(&mut axml_buff); // .expect("Error: cannot parse resource table");
                // ###############################
                // panic!("STOP")
                // ###############################
            },
            XmlTypes::ResXmlType => {
                // TODO: should we do something more here?
                /* Go back 2 bytes, to account from the block type */
                let initial_offset = axml_buff.position();
                axml_buff.set_position(initial_offset - 2);

                let _ = ChunkHeader::from_buff(&mut axml_buff, XmlTypes::ResXmlType)
                        .expect("Error: cannot parse AXML header");
            },
            XmlTypes::ResXmlStartNamespaceType => {
                parser::parse_start_namespace(&mut axml_buff, &global_strings, &mut namespace_prefixes);
            },
            XmlTypes::ResXmlEndNamespaceType => {
                parser::parse_end_namespace(&mut axml_buff, &global_strings);
            },
            XmlTypes::ResXmlStartElementType => {
                let (element_name, attrs) = parser::parse_start_element(&mut axml_buff, &global_strings, &namespace_prefixes).unwrap();
                parser::handle_event(&mut writer, element_name, attrs, &namespace_prefixes, XmlTypes::ResXmlStartElementType);
            },
            XmlTypes::ResXmlEndElementType => {
                let element_name = parser::parse_end_element(&mut axml_buff, &global_strings).unwrap();
                parser::handle_event(&mut writer, element_name, Vec::new(), &namespace_prefixes, XmlTypes::ResXmlEndElementType);
            },
            XmlTypes::ResXmlCDataType => panic!("TODO: RES_XML_CDATA_TYPE"),
            XmlTypes::ResXmlLastChunkType => panic!("TODO: RES_XML_LAST_CHUNK_TYPE"),

            XmlTypes::ResXmlResourceMapType => {
                let resource_map = ResourceMap::from_buff(&mut axml_buff)
                                                .expect("Error: cannot parse resource map");
            },

            XmlTypes::ResTablePackageType => {
                println!("=======================================");
                let chunk = ResTablePackage::parse(&mut axml_buff);
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

    if args.len() == 3 {
        let mut file = fs::File::create(&args[2]).unwrap();
        file.write_all(str_result.as_bytes()).unwrap();
    } else {
        println!("{}", str_result);
    }
}

