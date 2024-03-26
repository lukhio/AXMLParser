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
use quick_xml::name::QName;
use std::borrow::Cow;

use axml_parser::xml_types::XmlTypes;
use axml_parser::chunk_header::ChunkHeader;
use axml_parser::string_pool::StringPool;
use axml_parser::resource_map::ResourceMap;
use axml_parser::data_value_type::DataValueType;
use axml_parser::res_value::ResValue;
use axml_parser::res_table::{
    ResTable,
    ResTablePackage,
};


fn parse_start_namespace(axml_buff: &mut Cursor<Vec<u8>>,
                         strings: &[String],
                         namespaces: &mut HashMap::<String, String>) {
    /* Go back 2 bytes, to account from the block type */
    let offset = axml_buff.position();
    axml_buff.set_position(offset - 2);

    /* Parse chunk header */
    let header = ChunkHeader::from_buff(axml_buff, XmlTypes::ResXmlStartNamespaceType)
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
    let header = ChunkHeader::from_buff(axml_buff, XmlTypes::ResXmlEndNamespaceType)
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
    let header = ChunkHeader::from_buff(axml_buff, XmlTypes::ResXmlStartElementType)
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
            decoded_attr_val.push_str(&strings.get(attr_raw_val as usize).unwrap().to_string());
        } else {
            match data_value_type.data_type {
                DataValueType::TypeNull => println!("TODO: DataValueType::TypeNull"),
                DataValueType::TypeReference => {
                    decoded_attr_val.push_str("type1/");
                    decoded_attr_val.push_str(&data_value_type.data.to_string());
                },
                DataValueType::TypeAttribute => println!("TODO: DataValueType::TypeAttribute"),
                DataValueType::TypeString => println!("TODO: DataValueType::TypeString"),
                DataValueType::TypeFloat => println!("TODO: DataValueType::TypeFloat"),
                DataValueType::TypeDimension => println!("TODO: DataValueType::TypeDimension"),
                DataValueType::TypeFraction => println!("TODO: DataValueType::TypeFraction"),
                DataValueType::TypeDynamicReference => println!("TODO: DataValueType::TypeDynamicReference"),
                DataValueType::TypeDynamicAttribute => println!("TODO: DataValueType::TypeDynamicAttribute"),
                DataValueType::TypeIntDec => decoded_attr_val.push_str(&data_value_type.data.to_string()),
                DataValueType::TypeIntHex => {
                    decoded_attr_val.push_str("0x");
                    decoded_attr_val.push_str(&format!("{:x}", &data_value_type.data).to_string());
                },
                DataValueType::TypeIntBoolean => {
                    if data_value_type.data == 0 {
                        decoded_attr_val.push_str("false");
                    } else {
                        decoded_attr_val.push_str("true");
                    }
                },
                DataValueType::TypeIntColorArgb8 => println!("TODO: DataValueType::TypeIntColorArgb8"),
                DataValueType::TypeIntColorRgb8 => println!("TODO: DataValueType::TypeIntColorRgb8"),
                DataValueType::TypeIntColorArgb4 => println!("TODO: DataValueType::TypeIntColorArgb4"),
                DataValueType::TypeIntColorRgb4 => println!("TODO: DataValueType::TypeIntColorRgb4"),
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
    let header = ChunkHeader::from_buff(axml_buff, XmlTypes::ResXmlEndElementType)
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
                block_type: XmlTypes) {
    match block_type {
        XmlTypes::ResXmlStartElementType => {
            let mut elem = BytesStart::new(&element_name);

            if element_name == "manifest" {
                for (k, v) in namespace_prefixes.iter() {
                    if v == "android" {
                        let mut key = String::new();
                        key.push_str("xmlns:");
                        key.push_str(v);
                        let attr = Attribute {
                            key: QName(key.as_bytes()),
                            value: Cow::Borrowed(k.as_bytes())
                        };
                        elem.push_attribute(attr);
                        break;
                    }
                }
            }

            for (attr_key, attr_val) in element_attrs {
                let attr = Attribute {
                    key: QName(attr_key.as_bytes()),
                    value: Cow::Borrowed(attr_val.as_bytes())
                };
                elem.push_attribute(attr);
            }

            assert!(writer.write_event(Event::Start(elem)).is_ok());

        },
        XmlTypes::ResXmlEndElementType => {
            assert!(writer.write_event(Event::End(BytesEnd::new(element_name))).is_ok());
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
                parse_start_namespace(&mut axml_buff, &global_strings, &mut namespace_prefixes);
            },
            XmlTypes::ResXmlEndNamespaceType => {
                parse_end_namespace(&mut axml_buff, &global_strings);
            },
            XmlTypes::ResXmlStartElementType => {
                let (element_name, attrs) = parse_start_element(&mut axml_buff, &global_strings, &namespace_prefixes).unwrap();
                handle_event(&mut writer, element_name, attrs, &namespace_prefixes, XmlTypes::ResXmlStartElementType);
            },
            XmlTypes::ResXmlEndElementType => {
                let element_name = parse_end_element(&mut axml_buff, &global_strings).unwrap();
                handle_event(&mut writer, element_name, Vec::new(), &namespace_prefixes, XmlTypes::ResXmlEndElementType);
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

