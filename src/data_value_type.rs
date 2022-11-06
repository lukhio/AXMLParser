use std::io::Error;

/* Data value types
 *
 * Note: we ignore TypeFirstInt, TypeFirstColorInt, and TypeLastColorInt which hold the same values
 * as actual data types (respectively TypeIntDec, TypeIntColorArgb8, and TypeIntColorRgb4).
 */
pub enum DataValueType {
    /* The 'data' is either 0 or 1, specifying this resource is either undefined or empty,
     * respectively */
    TypeNull	            = 0x00,
    /* The 'data' holds a ResTable_ref, a reference to another resource table entry */
    TypeReference	        = 0x01,
    /* The 'data' holds an attribute resource identifier */
    TypeAttribute	        = 0x02,
    /* The 'data' holds an index into the containing resource table's global value string pool */
    TypeString	            = 0x03,
    /* The 'data' holds a single-precision floating point number */
    TypeFloat	            = 0x04,
    /* The 'data' holds a complex number encoding a dimension value, such as "100in" */
    TypeDimension	        = 0x05,
    /* The 'data' holds a complex number encoding a fraction of a container */
    TypeFraction	        = 0x06,
    /* The 'data' holds a dynamic ResTable_ref, which needs to be resolved before it can be used
    * like a TYPE_REFERENCE */
    TypeDynamicReference    = 0x07,
    /* The 'data' holds an attribute resource identifier, which needs to be resolved before it can
    * be used like a TYPE_ATTRIBUTE */
    TypeDynamicAttribute	= 0x08,

    /* Beginning of integer flavors... */
    /* The 'data' is a raw integer value of the form n..n */
    TypeIntDec	            = 0x10,
    /* The 'data' is a raw integer value of the form 0xn..n */
    TypeIntHex	            = 0x11,
    /* The 'data' is either 0 or 1, for input "false" or "true" respectively */
    TypeIntBoolean	        = 0x12,

    /* Beginning of color integer flavors... */
    /* The 'data' is a raw integer value of the form #aarrggbb */
    TypeIntColorArgb8		= 0x1c,
    /* The 'data' is a raw integer value of the form #rrggbb */
    TypeIntColorRgb8		= 0x1d,
    /* The 'data' is a raw integer value of the form #argb */
    TypeIntColorArgb4		= 0x1e,
    /* The 'data' is a raw integer value of the form #rgb */
    TypeIntColorRgb4		= 0x1f,
}

impl DataValueType {
    pub fn from_val(value: u8) -> Result<Self, Error> {
        let data_value_type = match value {
            0x00 => DataValueType::TypeNull,
            0x01 => DataValueType::TypeReference,
            0x02 => DataValueType::TypeAttribute,
            0x03 => DataValueType::TypeString,
            0x04 => DataValueType::TypeFloat,
            0x05 => DataValueType::TypeDimension,
            0x06 => DataValueType::TypeFraction,
            0x07 => DataValueType::TypeDynamicReference,
            0x08 => DataValueType::TypeDynamicAttribute,
            0x10 => DataValueType::TypeIntDec,
            0x11 => DataValueType::TypeIntHex,
            0x12 => DataValueType::TypeIntBoolean,
            0x1c => DataValueType::TypeIntColorArgb8,
            0x1d => DataValueType::TypeIntColorRgb8,
            0x1e => DataValueType::TypeIntColorArgb4,
            0x1f => DataValueType::TypeIntColorRgb4,
            _ => panic!("Error: unknown data value type {:02X}", value)
        };

        Ok(data_value_type)
    }
}
