use std::collections::HashMap;
use std::borrow::Cow;
use std::io::{
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

use crate::xml_types::XmlTypes;
use crate::chunk_header::ChunkHeader;
use crate::data_value_type::DataValueType;
use crate::res_value::ResValue;


pub fn parse_start_namespace(axml_buff: &mut Cursor<Vec<u8>>,
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

pub fn parse_end_namespace(axml_buff: &mut Cursor<Vec<u8>>,
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

pub fn parse_start_element(axml_buff: &mut Cursor<Vec<u8>>,
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

pub fn parse_end_element(axml_buff: &mut Cursor<Vec<u8>>,
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

pub fn handle_event<T> (writer: &mut Writer<T>,
                        element_name: String,
                        element_attrs: Vec<(String, String)>,
                        namespace_prefixes: &HashMap::<String, String>,
                        block_type: XmlTypes) where T: std::io::Write {
    match block_type {
        XmlTypes::ResXmlStartElementType => {
            // let mut elem = BytesStart::from_content(element_name.as_bytes(), element_name.len());
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
