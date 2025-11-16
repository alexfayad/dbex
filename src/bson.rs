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

/// Encode a single element
/// Format: [type][key_name][0x00][value_bytes]
fn encode_element(key: &str, value: &BsonValue, buffer: &mut Vec<u8>) {
    // [type]
    let type_byte = get_type_byte(value);
    buffer.push(type_byte); // [type]

    // [key_name]
    buffer.extend_from_slice(key.as_bytes());

    // [0x00] (null terminator)
    buffer.push(0x00);

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
            let length = (bytes.len() + 1) as i32; // +1 for null terminator
            buffer.extend_from_slice(&length.to_le_bytes());
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
            encode_document_internal(&doc.data, buffer);
        }
        BsonValue::Array(arr) => {
            // Array is encoded like a document with numeric string keys "0", "1", etc.
            // [size: i32][elements...][0x00]
            let mut arr_buffer = Vec::new();
            for (i, val) in arr.iter().enumerate() {
                encode_element(&i.to_string(), val, &mut arr_buffer);
            }
            arr_buffer.push(0x00);
            
            // Write array size (including size field itself)
            let size = (arr_buffer.len() + 4) as i32;
            buffer.extend_from_slice(&size.to_le_bytes());
            buffer.extend_from_slice(&arr_buffer);
        }
        BsonValue::Binary(bin) => {
            // [length: i32][subtype: u8][bytes]
            let length = bin.len() as i32;
            buffer.extend_from_slice(&length.to_le_bytes());
            buffer.push(0x00); // Generic binary subtype
            buffer.extend_from_slice(bin);
        }
    }
}

/// Decode BSON binary format to Document
pub fn decode_document(_bytes: &[u8]) -> Result<Document, String> {
    let mut buffer = _bytes.to_vec();
    let doc = decode_document_internal(&mut buffer);
    Ok(doc)
}

fn decode_document_internal(buffer: &mut Vec<u8>) -> Document {
    let mut doc = Document::new(0);
    while let Some(byte) = buffer.pop() {
        if byte == 0x00 {
            break;
        }
        let key = decode_string(buffer);
        let value = decode_value(buffer);
        doc.insert(key, value);
    }
    doc
}

fn decode_string(buffer: &mut Vec<u8>) -> String {
    let mut string = String::new();
    while let Some(byte) = buffer.pop() {
        if byte == 0x00 {
            break;
        }
        string.push(byte as char);
    }
    string
}

fn decode_value(buffer: &mut Vec<u8>) -> BsonValue {
    let type_byte = buffer.pop().unwrap();
    match type_byte {
        types::DOUBLE => {
            // Read 8 bytes (little-endian, but we're reading backwards so reverse)
            let mut bytes = [0u8; 8];
            for i in 0..8 {
                bytes[7 - i] = buffer.pop().unwrap(); // Read backwards, store forwards
            }
            let d = f64::from_le_bytes(bytes);
            BsonValue::Double(d)
        }
        types::STRING => {
            let s = decode_string(buffer);
            BsonValue::String(s)
        }
        types::DOCUMENT => {
            let doc = decode_document_internal(buffer);
            BsonValue::Document(Box::new(doc))
        }
        types::ARRAY => {
            // TODO: Implement decode_array
            BsonValue::Array(Vec::new())
        }
        types::BINARY => {
            // TODO: Implement decode_binary
            BsonValue::Binary(Vec::new())
        }
        types::BOOLEAN => {
            let b = buffer.pop().unwrap() == 0x01;
            BsonValue::Boolean(b)
        }
        types::NULL => {
            BsonValue::Null
        }
        types::INT32 => {
            // Read 4 bytes (little-endian, but we're reading backwards so reverse)
            let mut bytes = [0u8; 4];
            for i in 0..4 {
                bytes[3 - i] = buffer.pop().unwrap(); // Read backwards, store forwards
            }
            let i = i32::from_le_bytes(bytes);
            BsonValue::Int32(i)
        }
        types::INT64 => {
            // Read 8 bytes (little-endian, but we're reading backwards so reverse)
            let mut bytes = [0u8; 8];
            for i in 0..8 {
                bytes[7 - i] = buffer.pop().unwrap(); // Read backwards, store forwards
            }
            let i = i64::from_le_bytes(bytes);
            BsonValue::Int64(i)
        }
        _ => {
            BsonValue::Null
        }
    }
}
