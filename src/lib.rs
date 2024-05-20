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
    pub pkg_name: String,

    pub activities: Vec<String>,
    pub services: Vec<String>,
    pub providers: Vec<String>,
    pub receivers: Vec<String>,

    // TODO: does not includes permissions requested from within components
    pub created_perms: Vec<String>,
    pub requested_perms: Vec<String>,

    pub main_entry_point: Option<String>,
}

/// Open the file, read the contents, and create a `Cursor` of the raw data
/// for easier handling when parsing the XML data.
fn create_cursor(file_path: &str) -> Cursor<Vec<u8>> {

    let mut axml_cursor = Vec::new();

    let arg_type = match file_path.split(".").last() {
        Some("apk") => cli::ArgType::Apk,
        Some("xml") => cli::ArgType::Axml,
        Some("arsc") => cli::ArgType::Arsc,
        _ => panic!("Cannot infer file type from path"),
    };

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
fn get_manifest_contents(mut axml_cursor: Cursor<Vec<u8>>) -> ManifestContents {
    let mut contents = ManifestContents::default();

    let mut global_strings = Vec::new();
    let mut namespace_prefixes = HashMap::<String, String>::new();
    // let mut writer = Vec::new();

    loop {
        if let Ok(block_type) = XmlTypes::parse_block_type(&mut axml_cursor) {
            match block_type {
                XmlTypes::ResNullType => continue,
                XmlTypes::ResStringPoolType => {
                    let _ = StringPool::from_buff(&mut axml_cursor, &mut global_strings);
                },
                XmlTypes::ResTableType => {
                    let _ = ResTable::parse(&mut axml_cursor);
                },
                XmlTypes::ResXmlType => {
                    axml_cursor.set_position(axml_cursor.position() - 2);
                    let _ = ChunkHeader::from_buff(&mut axml_cursor, XmlTypes::ResXmlType);
                },
                XmlTypes::ResXmlStartNamespaceType => {
                    parser::parse_start_namespace(&mut axml_cursor, &global_strings, &mut namespace_prefixes);
                },
                XmlTypes::ResXmlEndNamespaceType => {
                    parser::parse_end_namespace(&mut axml_cursor, &global_strings);
                },
                XmlTypes::ResXmlStartElementType => {
                    let (element_type, attrs) = parser::parse_start_element(&mut axml_cursor, &global_strings, &namespace_prefixes).unwrap();

                    // Get element name from the attributes
                    // We only care about package name, activites, services, content providers and
                    // broadcast receivers which all have their name in the "android" namespace
                    let mut element_name = String::new();

                    for (attr_key, attr_val) in attrs.iter() {
                        if attr_key == "android:name" {
                            element_name = attr_val.to_string();
                            break;
                        }
                    }

                    match element_type.as_str() {
                        "activity" => contents.activities.push(element_name),
                        "service"  => contents.services.push(element_name),
                        "provider" => contents.providers.push(element_name),
                        "receiver" => contents.receivers.push(element_name),
                        "permission" => contents.created_perms.push(element_name),
                        "uses-permission" => contents.requested_perms.push(element_name),
                        "action" if element_name == "android.intent.action.MAIN" => contents.main_entry_point = contents.activities.last().cloned(),
                        _ => { }
                    }

                    // Package name is in the "manifest" element and with the "package" key
                    if element_type == "manifest" {
                        for (attr_key, attr_val) in attrs.iter() {
                            if attr_key == "package" {
                                contents.pkg_name = attr_val.to_string();
                                break;
                            }
                        }
                    }
                },
                XmlTypes::ResXmlEndElementType => {
                    parser::parse_end_element(&mut axml_cursor, &global_strings).unwrap();
                },

                XmlTypes::ResXmlResourceMapType => {
                    let _ = ResourceMap::from_buff(&mut axml_cursor);
                },

                _ => { },
            }
        }
        else  {
            break;
        }
    }

    contents
}

/// Convenience function to parse the manifest of an APK
pub fn parse_app_manifest(file_path: &str) -> ManifestContents {
    let cursor = create_cursor(file_path);
    get_manifest_contents(cursor)
}
