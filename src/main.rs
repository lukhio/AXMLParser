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
use byteorder::{
    LittleEndian,
    ReadBytesExt
};
use quick_xml::Writer;
use quick_xml::events::{Event, BytesEnd, BytesStart};
use quick_xml::events::attributes::Attribute;
use std::borrow::Cow;

mod xml_types;
mod chunk_header;
mod string_pool;
mod resource_map;
mod data_value_type;
mod res_value;

use xml_types::XmlTypes;
use chunk_header::ChunkHeader;
use string_pool::StringPool;
use resource_map::ResourceMap;
use data_value_type::DataValueType;
use res_value::ResValue;


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

fn parse_start_namespace(axml_buff: &mut Cursor<Vec<u8>>,
                         strings: &[String],
                         namespaces: &mut HashMap::<String, String>) {
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

    let prefix_str = strings.get(prefix as usize).unwrap();
    let uri_str = strings.get(uri as usize).unwrap();
    namespaces.insert(uri_str.to_string(), prefix_str.to_string());
}

fn parse_end_namespace(axml_buff: &mut Cursor<Vec<u8>>,
                       strings: &[String]) {
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
}

fn parse_start_element(axml_buff: &mut Cursor<Vec<u8>>,
                       strings: &[String],
                       namespace_prefixes: &HashMap::<String, String>) -> Result<(String, Vec<(String, String)>), Error> {
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

    let mut decoded_attrs = Vec::<(String, String)>::new();
    for _ in 0..attribute_count {
        let attr_namespace = axml_buff.read_u32::<LittleEndian>().unwrap();
        let attr_name = axml_buff.read_u32::<LittleEndian>().unwrap();
        let attr_raw_val = axml_buff.read_u32::<LittleEndian>().unwrap();
        let data_value_type = ResValue::from_buff(axml_buff).unwrap();

        let mut decoded_attr_key = String::new();
        let mut decoded_attr_val = String::new();

        if attr_namespace != 0xffffffff {
            let ns_prefix = namespace_prefixes.get(strings.get(attr_namespace as usize).unwrap()).unwrap();
            decoded_attr_key.push_str(ns_prefix);
            decoded_attr_key.push(':');
        } else {
            // TODO
        }

        decoded_attr_key.push_str(strings.get(attr_name as usize).unwrap());

        if attr_raw_val != 0xffffffff {
            decoded_attr_val.push_str(&format!("{}", strings.get(attr_raw_val as usize).unwrap()));
        } else {
            match data_value_type.data_type {
                DataValueType::TYPE_NULL => println!("TODO: DataValueType::TYPE_NULL"),
                DataValueType::TYPE_REFERENCE => {
                    decoded_attr_val.push_str("type1/");
                    decoded_attr_val.push_str(&data_value_type.data.to_string());
                },
                DataValueType::TYPE_ATTRIBUTE => println!("TODO: DataValueType::TYPE_ATTRIBUTE"),
                DataValueType::TYPE_STRING => println!("TODO: DataValueType::TYPE_STRING"),
                DataValueType::TYPE_FLOAT => println!("TODO: DataValueType::TYPE_FLOAT"),
                DataValueType::TYPE_DIMENSION => println!("TODO: DataValueType::TYPE_DIMENSION"),
                DataValueType::TYPE_FRACTION => println!("TODO: DataValueType::TYPE_FRACTION"),
                DataValueType::TYPE_DYNAMIC_REFERENCE => println!("TODO: DataValueType::TYPE_DYNAMIC_REFERENCE"),
                DataValueType::TYPE_DYNAMIC_ATTRIBUTE => println!("TODO: DataValueType::TYPE_DYNAMIC_ATTRIBUTE"),
                DataValueType::TYPE_INT_DEC => decoded_attr_val.push_str(&data_value_type.data.to_string()),
                DataValueType::TYPE_INT_HEX => {
                    decoded_attr_val.push_str("0x");
                    decoded_attr_val.push_str(&format!("{:x}", &data_value_type.data).to_string());
                },
                DataValueType::TYPE_INT_BOOLEAN => {
                    if data_value_type.data == 0 {
                        decoded_attr_val.push_str("false");
                    } else {
                        decoded_attr_val.push_str("true");
                    }
                },
                DataValueType::TYPE_INT_COLOR_ARGB8 => println!("TODO: DataValueType::TYPE_INT_COLOR_ARGB8"),
                DataValueType::TYPE_INT_COLOR_RGB8 => println!("TODO: DataValueType::TYPE_INT_COLOR_RGB8"),
                DataValueType::TYPE_INT_COLOR_ARGB4 => println!("TODO: DataValueType::TYPE_INT_COLOR_ARGB4"),
                DataValueType::TYPE_INT_COLOR_RGB4 => println!("TODO: DataValueType::TYPE_INT_COLOR_RGB4"),
                _ => println!("DataValueType::TYPE_NULL"),
            }
        }
        decoded_attrs.push((decoded_attr_key, decoded_attr_val));
    }

    Ok((strings.get(name as usize).unwrap().to_string(), decoded_attrs))
}

fn parse_end_element(axml_buff: &mut Cursor<Vec<u8>>,
                     strings: &[String]) -> Result<String, Error> {
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

    Ok(strings.get(name as usize).unwrap().to_string())
}

fn handle_event(writer: &mut Writer<Cursor<Vec<u8>>>,
                element_name: String,
                element_attrs: Vec<(String, String)>,
                namespace_prefixes: &HashMap::<String, String>,
                block_type: u16) {
    match block_type {
        XmlTypes::RES_XML_START_ELEMENT_TYPE => {
            let mut elem = BytesStart::owned(element_name.as_bytes(), element_name.len());

            if element_name == "manifest" {
                for (k, v) in namespace_prefixes.iter() {
                    if v == "android" {
                        let mut key = String::new();
                        key.push_str("xmlns:");
                        key.push_str(v);
                        let attr = Attribute {
                            key: key.as_bytes(),
                            value: Cow::Borrowed(k.as_bytes())
                        };
                        elem.push_attribute(attr);
                        break;
                    }
                }
            }

            for (attr_key, attr_val) in element_attrs {
                let attr = Attribute {
                    key: attr_key.as_bytes(),
                    value: Cow::Borrowed(attr_val.as_bytes())
                };
                elem.push_attribute(attr);
            }

            assert!(writer.write_event(Event::Start(elem)).is_ok());

        },
        XmlTypes::RES_XML_END_ELEMENT_TYPE => {
            assert!(writer.write_event(Event::End(BytesEnd::borrowed(element_name.as_bytes()))).is_ok());
        },
        _ => println!("{:02X}, other", block_type),
    }
}

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
    if fpath.ends_with(".xml") {
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
    let header = ChunkHeader::from_buff(&mut axml_buff, XmlTypes::RES_XML_TYPE)
                 .expect("Error: cannot parse AXML header");

    /* Now parsing the rest of the file */
    let mut global_strings = Vec::new();
    let mut namespace_prefixes = HashMap::<String, String>::new();

    /* Output stuff */
    let mut writer = Writer::new(Cursor::new(Vec::new()));

    loop {
        let block_type = get_next_block_type(&mut axml_buff);
        let block_type = match block_type {
            Ok(block) => block,
            Err(e) => break,
        };

        match block_type {
            XmlTypes::RES_NULL_TYPE => continue,
            XmlTypes::RES_STRING_POOL_TYPE => {
                StringPool::from_buff(&mut axml_buff, &mut global_strings)
                           .expect("Error: cannot parse string pool header");
            },
            XmlTypes::RES_TABLE_TYPE => panic!("TODO: RES_TABLE_TYPE"),
            XmlTypes::RES_XML_TYPE => panic!("TODO: RES_XML_TYPE"),

            XmlTypes::RES_XML_START_NAMESPACE_TYPE => {
                parse_start_namespace(&mut axml_buff, &global_strings, &mut namespace_prefixes);
            },
            XmlTypes::RES_XML_END_NAMESPACE_TYPE => {
                parse_end_namespace(&mut axml_buff, &global_strings);
            },
            XmlTypes::RES_XML_START_ELEMENT_TYPE => {
                let (element_name, attrs) = parse_start_element(&mut axml_buff, &global_strings, &namespace_prefixes).unwrap();
                handle_event(&mut writer, element_name, attrs, &namespace_prefixes, XmlTypes::RES_XML_START_ELEMENT_TYPE);
            },
            XmlTypes::RES_XML_END_ELEMENT_TYPE => {
                let element_name = parse_end_element(&mut axml_buff, &global_strings).unwrap();
                handle_event(&mut writer, element_name, Vec::new(), &namespace_prefixes, XmlTypes::RES_XML_END_ELEMENT_TYPE);
            },
            XmlTypes::RES_XML_CDATA_TYPE => panic!("TODO: RES_XML_CDATA_TYPE"),
            XmlTypes::RES_XML_LAST_CHUNK_TYPE => panic!("TODO: RES_XML_LAST_CHUNK_TYPE"),

            XmlTypes::RES_XML_RESOURCE_MAP_TYPE => {
                let resource_map = ResourceMap::from_buff(&mut axml_buff)
                                                .expect("Error: cannot parse resource map");
            },

            XmlTypes::RES_TABLE_PACKAGE_TYPE => panic!("TODO: RES_TABLE_PACKAGE_TYPE"),
            XmlTypes::RES_TABLE_TYPE_TYPE => panic!("TODO: RES_TABLE_TYPE_TYPE"),
            XmlTypes::RES_TABLE_TYPE_SPEC_TYPE => panic!("TODO: RES_TABLE_TYPE_SPEC_TYPE"),
            XmlTypes::RES_TABLE_LIBRARY_TYPE => panic!("TODO: RES_TABLE_LIBRARY_TYPE"),

            _ => println!("{:02X}, other", block_type),
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

