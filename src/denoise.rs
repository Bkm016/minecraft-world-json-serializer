//! 去噪声处理 - 移除运行时变化的字段

use crate::config::DenoiseConfig;
use fastnbt::Value;

/// 区块级噪声字段（默认值，用于向后兼容）
pub const CHUNK_NOISE_FIELDS: &[&str] = &[
    "LastUpdate",
    "InhabitedTime",
    "blending_data",
    "PostProcessing",
    "isLightOn",
];

/// 区块级激进去噪字段（默认值）
pub const CHUNK_AGGRESSIVE_FIELDS: &[&str] = &["Heightmaps"];

/// 存档级噪声字段（默认值）
pub const LEVEL_NOISE_FIELDS: &[&str] = &[
    "Time",
    "DayTime",
    "LastPlayed",
    "thunderTime",
    "rainTime",
    "clearWeatherTime",
    "WanderingTraderSpawnChance",
    "WanderingTraderSpawnDelay",
    "WanderingTraderId",
    "ServerBrands",
    "WasModded",
];

/// 对区块进行去噪处理（使用默认字段）
pub fn denoise_chunk(value: &mut Value, aggressive: bool) {
    if let Value::Compound(map) = value {
        for field in CHUNK_NOISE_FIELDS {
            map.remove(*field);
        }
        if aggressive {
            for field in CHUNK_AGGRESSIVE_FIELDS {
                map.remove(*field);
            }
        }
    }
}

/// 对区块进行去噪处理（使用配置）
pub fn denoise_chunk_with_config(value: &mut Value, aggressive: bool, config: &DenoiseConfig) {
    if let Value::Compound(map) = value {
        for field in &config.chunk.fields {
            map.remove(field);
        }
        if aggressive {
            for field in &config.chunk.aggressive_fields {
                map.remove(field);
            }
        }
    }
}

/// 对 level.dat 进行去噪处理（使用默认字段）
pub fn denoise_level(value: &mut Value) {
    if let Value::Compound(map) = value {
        if let Some(Value::Compound(data)) = map.get_mut("Data") {
            for field in LEVEL_NOISE_FIELDS {
                data.remove(*field);
            }
            // 重置天气
            data.insert("raining".to_string(), Value::Byte(0));
            data.insert("thundering".to_string(), Value::Byte(0));
        }
    }
}

/// 对 level.dat 进行去噪处理（使用配置）
pub fn denoise_level_with_config(value: &mut Value, config: &DenoiseConfig) {
    if let Value::Compound(map) = value {
        if let Some(Value::Compound(data)) = map.get_mut("Data") {
            for field in &config.level.fields {
                data.remove(field);
            }
            // 重置天气
            if config.level.reset_weather {
                data.insert("raining".to_string(), Value::Byte(0));
                data.insert("thundering".to_string(), Value::Byte(0));
            }
        }
    }
}

/// 恢复区块的默认值（还原时使用）
pub fn restore_defaults(value: &mut Value) {
    if let Value::Compound(map) = value {
        map.entry("LastUpdate".to_string())
            .or_insert(Value::Long(0));
        map.entry("InhabitedTime".to_string())
            .or_insert(Value::Long(0));
        map.entry("isLightOn".to_string()).or_insert(Value::Byte(1));
    }
}
