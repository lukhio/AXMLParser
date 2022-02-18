#![allow(dead_code)]

pub struct DataValueType {}

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

    pub fn from_val(value: u8) -> u8 {
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
