extern crate varint;
extern crate bytebuffer_new as bytebuffer;
extern crate base64;
use bytebuffer::{ByteBuffer, Endian};
use varint::{VarintRead};
use std::io::Cursor;
mod hex;
use hex::{Hex};
#[macro_use]
extern crate serde_json;
use serde_json::{Value, to_string, map::Map, map::Iter};

pub fn encode(json: &Value) -> String {
    let mut buff:ByteBuffer = ByteBuffer::new();
    buff.set_endian(Endian::LittleEndian);
    let loaded_buff = _encode(json, &mut buff);
    base64::encode(&loaded_buff.to_bytes())
}

fn _encode<'a>(json: &Value, buff: &'a mut ByteBuffer) -> &'a ByteBuffer {
   match json {
       json if json.is_null() => buff.write_u8(Hex::NULL as u8),
       json if json.is_string() => {
           if json.as_str().unwrap().len() > 0 {
            buff.write_u8(Hex::ESTRING as u8)
           } else {
               buff.write_u8(Hex::STRING as u8);
               buff.write_u32(json.as_str().unwrap().len() as u32);
               buff.write_string(json.as_str().unwrap())
           }
       },
       json if json.is_i64() => {
         buff.write_u8(Hex::LONG as u8);
         buff.write_i64(json.as_i64().unwrap())
       },
       json if json.is_f64() => {
           buff.write_u8(Hex::DOUBLE as u8);
           buff.write_f64(json.as_f64().unwrap())
       },
       json if json.is_boolean() && json.as_bool().unwrap() == true => {
           buff.write_u8(Hex::TRUE as u8)
       },
       json if json.is_boolean() && json.as_bool().unwrap() == false => {
           buff.write_u8(Hex::FALSE as u8)
       },
       json if json.is_array() && json.as_array().unwrap().len() == 0 => {
           buff.write_u8(Hex::EARRAY as u8)
       },
       json if json.is_object() && json.as_object().unwrap().is_empty() => {
           buff.write_u8(Hex::EOBJECT as u8)
       },
       json if json.is_array() => {
           buff.write_u8(Hex::ARRAY as u8);
           let array = json.as_array().unwrap();
           buff.write_u32(array.len() as u32);
           for value in array {
               _encode(&value, buff);
           }
       },
       json if json.is_object() => {
           buff.write_u8(Hex::OBJECT as u8);
           let key_len = json.as_object().unwrap().len();
           buff.write_u32(key_len as u32);
           let map:&Map<String,Value> = json.as_object().unwrap();
           for key in map.keys() {
              let value = map.get(key);
            if value.is_some() {
                buff.write_u8(Hex::STRING as u8);
                buff.write_string(key);
                _encode(&value.unwrap(), buff);
              }
           }
       },
       _ => {
           buff.write_u8(Hex::NULL as u8)
       }
    }
    buff
}

pub fn decode(pson_str: &str) {
    let bytes: &[u8] = pson_str.as_bytes();
    let mut _buff = ByteBuffer::from_bytes(bytes);
    let _le = _buff.set_endian(Endian::LittleEndian);
    _decode(&mut _buff);
}

fn _decode(buff: &mut ByteBuffer) -> Value {
        let byte: u8 = buff.read_u8();
        let val;
        if byte <= Hex::MAX as u8 {
           val = json!(zig_zag_decode32_from_u8(byte));
        } else {
          val = match byte {
                byte if byte == Hex::NULL as u8 => json!(null),
                byte if byte == Hex::TRUE as u8 => json!(true),
                byte if byte == Hex::FALSE as u8 => json!(false),
                byte if byte == Hex::EOBJECT as u8 => json!({}),
                byte if byte == Hex::EARRAY as u8 => json!([]),
                byte if byte == Hex::ESTRING as u8 => json!(""),
                byte if byte == Hex::OBJECT as u8 => {
                    let data =  buff.to_bytes();
                    let(_digested, undigested) = data.split_at(buff.get_rpos());
                    let mut cursor = Cursor::new(undigested.to_vec());
                    use std::collections::BTreeMap;
                    let mut object = BTreeMap::new();
                    let mut keys_len = cursor.read_unsigned_varint_32().unwrap() as i32;
                    buff.read_u8(); // move after size byte, TODO - clean 
                    let mut result: Value = json!({});
                    while keys_len  > 0 {
                        let key_object = _decode(buff);
                        let key = to_string(&key_object).ok().unwrap();
                        let value_object = _decode(buff);
                        let value = value_object.clone();
                        object.insert(key, value);
                        result = json!(object);
                        keys_len = minus_one(keys_len);
                    }
                    result
                },
                byte if byte == Hex::ARRAY as u8 => {
                    let data =  buff.to_bytes();
                    let(_digested, undigested) = data.split_at(buff.get_rpos());
                    let mut cursor = Cursor::new(undigested.to_vec());
                    let mut vector = Vec::new();
                    let mut keys_len = cursor.read_unsigned_varint_32().unwrap() as i32;
                    while --keys_len >= 0 {
                        vector.push(_decode(buff).clone());
                    }
                    json!(vector)
                },
                byte if byte == Hex::INTEGER as u8 => {
                    json!(zig_zag_decode32_from_u32(buff.read_u32()))
                },
                byte if byte == Hex::LONG as u8 =>{
                    json!(zig_zag_decode32_from_u64(buff.read_u64()))
                },
                byte if byte == Hex::FLOAT as u8 =>{
                    json!(buff.read_f32())
                },
                byte if byte == Hex::DOUBLE as u8 =>{
                    json!(buff.read_f64())
                },
                byte if byte == Hex::STRING as u8 =>{
                    let size = buff.read_u8();
                    let val = String::from_utf8(buff.read_bytes(size as usize)).unwrap();
                    json!(val)  
                },
                _ => json!(1)
            };
        }
        val
    }

    fn minus_one(value: i32) -> i32 {
        let mut value = value.clone();
        value = value - 1;
        value
    }

    fn zig_zag_decode32_from_u8(byte: u8) -> i32 {
        (((byte as u32 >> 1) ^ (- ((byte as u32 & 1)as i32)) as u32) as i32 | 0 )as i32
    }

    fn zig_zag_decode32_from_u32(varint: u32) -> i32 {
        (((varint >> 1) ^ (- ((varint & 1)as i32)) as u32) as i32 | 0 )as i32
    }

    fn zig_zag_decode32_from_u64(varint: u64) -> i64 {
        (((varint >> 1) ^ (- ((varint & 1)as i64)) as u64) as i64 | 0 )as i64
    }

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{from_value, Map};
    #[test]
    fn zig_zag_decode32_test() {
        let mut byte: u8 = 0x01;
        assert_eq!(-1 as i32, zig_zag_decode32_from_u8(byte));
        byte = 0x02;
        assert_eq!(1 as i32, zig_zag_decode32_from_u8(byte));
        byte = 0x00;
        assert_eq!(0 as i32, zig_zag_decode32_from_u8(byte));
        byte = 0x04;
        assert_eq!(2 as i32, zig_zag_decode32_from_u8(byte));
        byte = 0xEE;
        assert_eq!(119 as i32, zig_zag_decode32_from_u8(byte));
        assert_eq!(-120 as i32, zig_zag_decode32_from_u8(Hex::MAX as u8));
        byte = 0x03;
        assert_eq!(-2 as i32, zig_zag_decode32_from_u8(byte));
    }

    #[test]
    fn decode_test(){
        let mut buff:ByteBuffer = ByteBuffer::from_bytes(&[0xF6, 0x01, 0xFC, 0x01, 0x61, 0xFC, 0x01, 0x62]);
        buff.set_endian(Endian::LittleEndian);
        let bytes = &buff.to_bytes();
        let encode_str = base64::encode(&bytes);
        let mut buff_from_str: ByteBuffer = ByteBuffer::from_bytes(&base64::decode(&encode_str).unwrap());
        let value = _decode(&mut buff_from_str);
        assert!(value.is_object());
        let mut expected = Map::new();
        expected.insert(json!("a").to_string(), from_value(json!("b")).unwrap());
        assert_eq!(value.as_object().unwrap(), &expected);
        let chiper = encode(&value);
        assert_eq!(&chiper, &buff);
    }
}

