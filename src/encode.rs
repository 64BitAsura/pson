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
use serde_json::{Value, to_string};

