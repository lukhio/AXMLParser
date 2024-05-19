pub mod cli;
pub mod parser;
pub mod xml_types;
pub mod chunk_header;
pub mod string_pool;
pub mod resource_map;
pub mod data_value_type;
pub mod res_value;
pub mod res_table;

use std::{
    fs,
    collections::HashMap,
};
use std::io::{
    Read,
    Cursor,
};
use crate::cli::ArgType;
use crate::chunk_header::ChunkHeader;
use crate::resource_map::ResourceMap;
use crate::res_table::{
    ResTable,
    ResTablePackage
};
use crate::string_pool::StringPool;
use crate::xml_types::XmlTypes;

/// Representation of an app's manifest contents
#[derive(Debug, Default)]
pub struct ManifestContents {
    pkg_name: String,
    activities: Vec<String>,
    services: Vec<String>,
    providers: Vec<String>,
    receivers: Vec<String>,
}

/// Open the file, read the contents, and create a `Cursor` of the raw data
/// for easier handling when parsing the XML data.
pub fn create_cursor(arg_type: ArgType,
                     file_path: &str) -> Cursor<Vec<u8>> {

    let mut axml_cursor = Vec::new();

    if arg_type == cli::ArgType::Apk {
        // If we are dealing with an APK, we must first extract the binary XML from it
        // In this case we assume the user wants to decode the app manifest so we extract that

        let zipfile = std::fs::File::open(file_path).unwrap();
        let mut archive = zip::ZipArchive::new(zipfile).unwrap();
        let mut raw_file = match archive.by_name("AndroidManifest.xml") {
            Ok(file) => file,
            Err(..) => {
                panic!("Error: no AndroidManifest.xml in APK");
            }
        };
        raw_file.read_to_end(&mut axml_cursor).expect("Error: cannot read manifest from app");
    } else {
        let mut raw_file = fs::File::open(file_path).expect("Error: cannot open AXML file");
        raw_file.read_to_end(&mut axml_cursor).expect("Error: cannot read AXML file");
    }

    Cursor::new(axml_cursor)
}

/// Parse an app's manifest and extract interesting contents
/// For now, only these elements are extracted, although that
/// list might get longer in the future:
///
///   * package name
///   * list of activities names
///   * list of services names
///   * list of content providers names
///   * list of broadcast receiver names
pub fn get_manifest_contents(mut axml_cursor: Cursor<Vec<u8>>) -> ManifestContents {
    let mut contents = ManifestContents::default();

    let mut global_strings = Vec::new();
    let mut namespace_prefixes = HashMap::<String, String>::new();
    // let mut writer = Vec::new();

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
                let (element_type, attrs) = parser::parse_start_element(&mut axml_cursor, &global_strings, &namespace_prefixes).unwrap();
                eprintln!("{element_type}");

                // Get element name from the attributes
                // We only care about activites, services, content providers and
                // broadcast receivers which all have their name in the "android" namespace
                let mut element_name = String::new();
                for (attr_key, attr_val) in attrs.iter() {
                    if attr_key == "android:name" {
                        element_name = attr_val.to_string();
                        break;
                    }
                }

                // Check that we indeed have a name
                if element_name.is_empty() {
                    continue;
                }

                match element_type.as_str() {
                    "activity" => contents.activities.push(element_name),
                    "service"  => contents.services.push(element_name),
                    "provider" => contents.providers.push(element_name),
                    "receiver" => contents.receivers.push(element_name),
                    _ => { }
                }
                // parser::handle_event(&mut writer, element_name, attrs, &namespace_prefixes, XmlTypes::ResXmlStartElementType);
            },
            XmlTypes::ResXmlEndElementType => {
                let element_name = parser::parse_end_element(&mut axml_cursor, &global_strings).unwrap();
//                 parser::handle_event(&mut writer, element_name, Vec::new(), &namespace_prefixes, XmlTypes::ResXmlEndElementType);
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

    contents
}
