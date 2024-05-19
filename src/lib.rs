pub mod cli;
pub mod parser;
pub mod xml_types;
pub mod chunk_header;
pub mod string_pool;
pub mod resource_map;
pub mod data_value_type;
pub mod res_value;
pub mod res_table;

use std::fs;
use std::io::{
    Read,
    Cursor,
};
use crate::cli::ArgType;

pub fn create_cursor(arg_type: ArgType,
                     file_path: &str) -> Cursor<Vec<u8>> {

    let mut axml_vec_buff = Vec::new();

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
        raw_file.read_to_end(&mut axml_vec_buff).expect("Error: cannot read manifest from app");
    } else {
        let mut raw_file = fs::File::open(file_path).expect("Error: cannot open AXML file");
        raw_file.read_to_end(&mut axml_vec_buff).expect("Error: cannot read AXML file");
    }

    Cursor::new(axml_vec_buff)
}
