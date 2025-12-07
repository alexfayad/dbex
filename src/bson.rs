use crate::{Document, BsonValue};

/// BSON type markers (single bytes)
pub mod types {
    pub const DOUBLE: u8 = 0x01;
    pub const STRING: u8 = 0x02;
    pub const DOCUMENT: u8 = 0x03;
    pub const ARRAY: u8 = 0x04;
    pub const BINARY: u8 = 0x05;
    pub const BOOLEAN: u8 = 0x08;
    pub const NULL: u8 = 0x0A;
    pub const INT32: u8 = 0x10;
    pub const INT64: u8 = 0x12;
}

/// Get the BSON type byte for a value
fn get_type_byte(value: &BsonValue) -> u8 {
    match value {
        BsonValue::Double(_) => types::DOUBLE,
        BsonValue::String(_) => types::STRING,
        BsonValue::Document(_) => types::DOCUMENT,
        BsonValue::Array(_) => types::ARRAY,
        BsonValue::Binary(_) => types::BINARY,
        BsonValue::Boolean(_) => types::BOOLEAN,
        BsonValue::Null => types::NULL,
        BsonValue::Int32(_) => types::INT32,
        BsonValue::Int64(_) => types::INT64,
    }
}

/// Encode a Document to BSON binary format
pub fn encode_document(doc: &Document) -> Vec<u8> {
    let mut buffer = Vec::new();
    // Get Document ID and encode it
    buffer.extend_from_slice(&doc.id.to_le_bytes());

    // Encode Document data 
    encode_document_internal(&doc.data, &mut buffer);
    buffer
}

/// Internal document encoding (recursive)
/// Document format: 
/// [size: i32]
/// [elements...]
/// [0x00] (null terminator)
fn encode_document_internal(data: &std::collections::HashMap<String, BsonValue>, buffer: &mut Vec<u8>) {
    let start_pos = buffer.len();
    
    // Reserve space for size (4 bytes)
    buffer.extend_from_slice(&[0u8; 4]);
    
    // Encode each field
    for (key, value) in data {
        encode_element(key, value, buffer);
    }
    
    // End of document marker
    buffer.push(0x00);
    
    // Write the size at the beginning
    let size = (buffer.len() - start_pos) as i32;
    let size_bytes = size.to_le_bytes();
    buffer[start_pos..start_pos + 4].copy_from_slice(&size_bytes);
}

fn encode_object(data: &std::collections::HashMap<String, BsonValue>, buffer: &mut Vec<u8>) {
    // Encode each field
    for (key, value) in data {
        encode_element(key, value, buffer);
    }
    
    // End of object marker
    buffer.push(0x00);
}

/// Encode a single element
/// Format: [key_name][0x00][type][value_bytes]
fn encode_element(key: &str, value: &BsonValue, buffer: &mut Vec<u8>) {
    // [key_name]
    buffer.extend_from_slice(key.as_bytes());

    // [0x00] (null terminator)
    buffer.push(0x00);

    // [type]
    let type_byte = get_type_byte(value);
    buffer.push(type_byte); // [type]

    // [value_bytes]
    encode_value(value, buffer);
}

/// Encode a BSON value
/// Format: [value_bytes]
fn encode_value(value: &BsonValue, buffer: &mut Vec<u8>) {
    match value {
        BsonValue::Double(d) => {
            // 8 bytes, little-endian
            buffer.extend_from_slice(&d.to_le_bytes());
        }
        BsonValue::String(s) => {
            // [length: i32][string_bytes][0x00]
            let bytes = s.as_bytes();
            buffer.extend_from_slice(bytes);
            buffer.push(0x00);
        }
        BsonValue::Int32(i) => {
            // 4 bytes, little-endian
            buffer.extend_from_slice(&i.to_le_bytes());
        }
        BsonValue::Int64(i) => {
            // 8 bytes, little-endian
            buffer.extend_from_slice(&i.to_le_bytes());
        }
        BsonValue::Boolean(b) => {
            // 1 byte: 0x00 (false) or 0x01 (true)
            buffer.push(if *b { 0x01 } else { 0x00 });
        }
        BsonValue::Null => {
            // No bytes - type marker is enough
        }
        BsonValue::Document(doc) => {
            // Recursive document encoding
            encode_object(&doc.data, buffer);
        }
        BsonValue::Array(arr) => {
            // Array is encoded like a document with numeric string keys "0", "1", etc.
            // Format: [size: i32][elements...][0x00]
            let mut arr_buffer = Vec::new(); // New buffer for array elements
            for (i, val) in arr.iter().enumerate() {
                encode_element(&i.to_string(), val, &mut arr_buffer);
            }

            // [0x00] (null terminator)
            arr_buffer.push(0x00);
            
            // [size: i32]
            let size = (arr_buffer.len() + 4) as i32;
            buffer.extend_from_slice(&size.to_le_bytes());

            // [elements...]
            buffer.extend_from_slice(&arr_buffer);
        }
        BsonValue::Binary(bin) => {
            // Format: [length: i32][subtype: u8][bytes]

            // [length: i32]
            let length = bin.len() as i32;
            buffer.extend_from_slice(&length.to_le_bytes());

            // [subtype: u8]
            buffer.push(0x00);

            // [bytes]
            buffer.extend_from_slice(bin);
        }
    }
}

/// Decode BSON binary format to Document
pub fn decode_document(_bytes: &[u8]) -> Result<Document, String> {
    let buffer = _bytes.to_vec();
    let mut pos = 0;

    let doc = decode_internal_document(&buffer, &mut pos);
    Ok(doc)
}
fn decode_internal_document(buffer: &Vec<u8>, pos: &mut usize) -> Document{

    let id = u64::from_le_bytes([buffer[*pos], buffer[*pos + 1], buffer[*pos + 2], buffer[*pos + 3], buffer[*pos + 4], buffer[*pos + 5], buffer[*pos + 6], buffer[*pos + 7]]);
    *pos += 8; // 8 bytes for id

    let mut doc = Document::new();
    doc.id = id;
    
    let size = i32::from_le_bytes([buffer[*pos], buffer[*pos + 1], buffer[*pos + 2], buffer[*pos + 3]]) as usize + 8; // 8 bytes for id
    *pos += 4;

    while *pos < size {
        let key = decode_string(buffer, pos);
        *pos += 1;
        let type_byte = buffer[*pos];
        *pos += 1;
        let value = decode_value(buffer, pos, type_byte);
        doc.insert(key, value);
        *pos += 1;
    }
    doc
}

fn decode_string(buffer: &Vec<u8>, pos: &mut usize) -> String {
    let mut string = String::new();
    while buffer[*pos] != 0x00 {
        string.push(buffer[*pos] as char);
        *pos += 1;
    }
    string
}

fn decode_object(buffer: &Vec<u8>, pos: &mut usize) -> Document{
    let mut doc = Document::new();
    while buffer[*pos] != 0x00 {
        let key = decode_string(buffer, pos);
        *pos += 1;
        let type_byte = buffer[*pos];
        *pos += 1;
        let value = decode_value(buffer, pos, type_byte);
        doc.insert(key, value);
        *pos += 1;
    }
    doc
}

fn decode_value(buffer: &Vec<u8>, pos: &mut usize, type_byte: u8) -> BsonValue {

    let value = match type_byte {
        types::DOUBLE => {
            let mut bytes = [0u8; 8];
            for i in 0..8 {
                bytes[7 - i] = buffer[*pos];
                *pos += 1;
            }
            let d = f64::from_le_bytes(bytes);
            BsonValue::Double(d)
        }
        types::STRING => {
            let string = decode_string(buffer, pos);
            BsonValue::String(string)
        }
        types::DOCUMENT => {
            let doc = decode_object(buffer, pos);
            BsonValue::Document(Box::new(doc))
        }
        types::ARRAY => {
            let mut array = Vec::new();
            while buffer[*pos] != 0x00 {
                array.push(decode_value(buffer, pos, buffer[*pos]));
            }
            BsonValue::Array(array)
        }
        types::BINARY => {
            // Format: [length: i32][subtype: u8][bytes]
            let length = buffer[*pos] as i32;
            *pos += 1;
            let mut bytes = Vec::new();
            for _ in 0..length {
                bytes.push(buffer[*pos]);
                *pos += 1;
            }
            BsonValue::Binary(bytes)
        }
        types::BOOLEAN => {
            let b = buffer[*pos] == 0x01;
            BsonValue::Boolean(b)
        }
        types::NULL => {
            BsonValue::Null
        }
        types::INT32 => {
            // Format: [value: i32]
            let mut bytes = [0u8; 4];
            for i in 0..4 {
                bytes[3 - i] = buffer[*pos];
                *pos += 1;
            }
            let i = i32::from_le_bytes(bytes);
            BsonValue::Int32(i)
        }
        types::INT64 => {
            // Format: [value: i64]
            let mut bytes = [0u8; 8];
            for i in 0..8 {
                bytes[7 - i] = buffer[*pos];
                *pos += 1;
            }
            let i = i64::from_le_bytes(bytes);
            BsonValue::Int64(i)
        }
        _ => BsonValue::Null,
    };
    *pos += 1;
    value
}
