//! NBT 与 JSON 之间的转换

use anyhow::Result;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use fastnbt::{ByteArray, IntArray, LongArray, Value};
use serde_json::{json, Map, Value as JsonValue};
use std::collections::HashMap;

/// 将 fastnbt Value 转换为紧凑 JSON 格式
pub fn nbt_to_json(value: &Value) -> JsonValue {
    match value {
        Value::Byte(v) => JsonValue::String(format!("{}b", v)),
        Value::Short(v) => JsonValue::String(format!("{}s", v)),
        Value::Int(v) => JsonValue::Number((*v).into()),
        Value::Long(v) => JsonValue::String(format!("{}L", v)),
        Value::Float(v) => JsonValue::String(format!("{}f", v)),
        Value::Double(v) => {
            if let Some(n) = serde_json::Number::from_f64(*v) {
                JsonValue::Number(n)
            } else {
                JsonValue::String(format!("{}d", v))
            }
        }
        Value::String(s) => {
            // 检查是否需要转义
            let needs_escape = is_type_like_string(s);
            if needs_escape {
                JsonValue::String(format!("{}\\0", s))
            } else {
                JsonValue::String(s.clone())
            }
        }
        Value::ByteArray(arr) => {
            let bytes: Vec<u8> = arr.iter().map(|&b| b as u8).collect();
            JsonValue::String(format!("B;{}", BASE64.encode(&bytes)))
        }
        Value::IntArray(arr) => {
            let mut bytes = Vec::with_capacity(arr.len() * 4);
            for &v in arr.iter() {
                bytes.extend_from_slice(&v.to_be_bytes());
            }
            JsonValue::String(format!("I;{}", BASE64.encode(&bytes)))
        }
        Value::LongArray(arr) => {
            let mut bytes = Vec::with_capacity(arr.len() * 8);
            for &v in arr.iter() {
                bytes.extend_from_slice(&v.to_be_bytes());
            }
            JsonValue::String(format!("L;{}", BASE64.encode(&bytes)))
        }
        Value::List(list) => {
            if list.is_empty() {
                json!({"[]": "End"})
            } else {
                JsonValue::Array(list.iter().map(nbt_to_json).collect())
            }
        }
        Value::Compound(map) => {
            let obj: Map<String, JsonValue> = map
                .iter()
                .map(|(k, v)| (k.clone(), nbt_to_json(v)))
                .collect();
            JsonValue::Object(obj)
        }
    }
}

/// 检查字符串是否看起来像类型标记
fn is_type_like_string(s: &str) -> bool {
    if s.len() < 2 {
        return false;
    }
    // 检查 "123b", "123s", "123L", "1.5f" 格式
    if let Some(last) = s.chars().last() {
        if matches!(last, 'b' | 's' | 'L' | 'f') {
            // 这些后缀都是 ASCII（1 字节），可以安全切片
            let prefix = &s[..s.len() - 1];
            if prefix.parse::<f64>().is_ok() {
                return true;
            }
        }
    }
    // 检查 "B;", "I;", "L;" 前缀（都是 ASCII）
    if s.len() > 2 && s.as_bytes().get(1) == Some(&b';') {
        let first = s.as_bytes()[0];
        if matches!(first, b'B' | b'I' | b'L') {
            return true;
        }
    }
    false
}

/// 将 JSON 转换回 fastnbt Value
pub fn json_to_nbt(json: &JsonValue) -> Result<Value> {
    match json {
        JsonValue::Object(obj) => {
            // 检查空列表标记
            if obj.len() == 1 && obj.contains_key("[]") {
                return Ok(Value::List(vec![]));
            }
            let mut map = HashMap::new();
            for (k, v) in obj {
                map.insert(k.clone(), json_to_nbt(v)?);
            }
            Ok(Value::Compound(map))
        }
        JsonValue::Array(arr) => {
            let list: Result<Vec<Value>> = arr.iter().map(json_to_nbt).collect();
            Ok(Value::List(list?))
        }
        JsonValue::String(s) => parse_string_value(s),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                if i >= i32::MIN as i64 && i <= i32::MAX as i64 {
                    Ok(Value::Int(i as i32))
                } else {
                    Ok(Value::Long(i))
                }
            } else if let Some(f) = n.as_f64() {
                Ok(Value::Double(f))
            } else {
                Ok(Value::Int(0))
            }
        }
        JsonValue::Bool(b) => Ok(Value::Byte(if *b { 1 } else { 0 })),
        JsonValue::Null => Ok(Value::Byte(0)),
    }
}

/// 解析字符串值（可能包含类型标记）
fn parse_string_value(s: &str) -> Result<Value> {
    // 转义字符串（\0 是 2 字节 ASCII）
    if s.ends_with("\\0") {
        return Ok(Value::String(s[..s.len() - 2].to_string()));
    }

    // 数组类型（B;, I;, L; 都是 ASCII 前缀）
    if s.len() > 2 && s.as_bytes().get(1) == Some(&b';') {
        let prefix = s.as_bytes()[0];
        let b64 = &s[2..];
        let bytes = BASE64.decode(b64)?;

        match prefix {
            b'B' => {
                let arr: Vec<i8> = bytes.iter().map(|&b| b as i8).collect();
                return Ok(Value::ByteArray(ByteArray::new(arr)));
            }
            b'I' => {
                let arr: Vec<i32> = bytes
                    .chunks(4)
                    .map(|c| i32::from_be_bytes([c[0], c[1], c[2], c[3]]))
                    .collect();
                return Ok(Value::IntArray(IntArray::new(arr)));
            }
            b'L' => {
                let arr: Vec<i64> = bytes
                    .chunks(8)
                    .map(|c| i64::from_be_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]))
                    .collect();
                return Ok(Value::LongArray(LongArray::new(arr)));
            }
            _ => {}
        }
    }

    // 数值类型后缀（b, s, L, f 都是 1 字节 ASCII）
    if let Some(last) = s.chars().last() {
        if matches!(last, 'b' | 's' | 'L' | 'f') {
            let prefix = &s[..s.len() - 1]; // 安全：后缀是 1 字节 ASCII
            match last {
                'b' => {
                    if let Ok(v) = prefix.parse::<i8>() {
                        return Ok(Value::Byte(v));
                    }
                }
                's' => {
                    if let Ok(v) = prefix.parse::<i16>() {
                        return Ok(Value::Short(v));
                    }
                }
                'L' => {
                    if let Ok(v) = prefix.parse::<i64>() {
                        return Ok(Value::Long(v));
                    }
                }
                'f' => {
                    if let Ok(v) = prefix.parse::<f32>() {
                        return Ok(Value::Float(v));
                    }
                }
                _ => {}
            }
        }
    }

    // 普通字符串
    Ok(Value::String(s.to_string()))
}
