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
    "CarvingMasks",
    // starlight 相关字段（状态和版本都要移除，让游戏重新计算）
    "starlight.light_version",
    "starlight.blocklight_state",
    "starlight.skylight_state",
];

/// 区块级激进去噪字段（默认值）
pub const CHUNK_AGGRESSIVE_FIELDS: &[&str] = &[
    "Heightmaps",
    "fluid_ticks",
    "block_ticks",
    "structures",
];

/// Section 级别需要移除的光照字段（默认移除，让游戏重新计算）
pub const SECTION_LIGHT_FIELDS: &[&str] = &[
    "BlockLight",
    "SkyLight",
];

/// Section 级激进去噪字段（额外的）
pub const SECTION_AGGRESSIVE_FIELDS: &[&str] = &[];

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
    "Player",
];

/// 对区块进行去噪处理（使用默认字段）
pub fn denoise_chunk(value: &mut Value, aggressive: bool) {
    if let Value::Compound(map) = value {
        for field in CHUNK_NOISE_FIELDS {
            map.remove(*field);
        }
        
        // 默认移除 section 级别的光照数据（让游戏重新计算）
        if let Some(Value::List(sections)) = map.get_mut("sections") {
            for section in sections.iter_mut() {
                if let Value::Compound(sec_map) = section {
                    for field in SECTION_LIGHT_FIELDS {
                        sec_map.remove(*field);
                    }
                }
            }
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
        
        // 默认移除 section 级别的光照数据
        if let Some(Value::List(sections)) = map.get_mut("sections") {
            for section in sections.iter_mut() {
                if let Value::Compound(sec_map) = section {
                    for field in SECTION_LIGHT_FIELDS {
                        sec_map.remove(*field);
                    }
                }
            }
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
        // isLightOn=0 让游戏重新计算光照（因为激进模式可能移除了光照数据）
        map.entry("isLightOn".to_string())
            .or_insert(Value::Byte(0));
    }
}
