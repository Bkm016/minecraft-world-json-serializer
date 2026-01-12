//! 导出世界为 JSON 格式

use crate::config::{Area, Config, DenoiseConfig, ExportConfig, FieldMappingConfig};
use crate::denoise::{
    denoise_chunk, denoise_chunk_with_config, denoise_level, denoise_level_with_config,
};
use crate::mca::{parse_mca_filename, read_mca};
use crate::nbt_json::{nbt_to_json, shorten_json_keys, FieldMapper};
use anyhow::{Context, Result};
use fastnbt::Value;
use rayon::prelude::*;
use serde_json::{json, Value as JsonValue};
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;
use std::sync::Arc;

/// 维度定义
const DIMENSIONS: &[(&str, &str)] = &[
    ("", "主世界"),    // 主世界 region/
    ("DIM-1", "地狱"), // 地狱 DIM-1/region/
    ("DIM1", "末地"),  // 末地 DIM1/region/
];

/// 导出整个世界（使用默认去噪字段）
pub fn export_world(
    world_path: &Path,
    output_path: &Path,
    denoise: bool,
    aggressive: bool,
) -> Result<()> {
    fs::create_dir_all(output_path)?;

    // 导出 level.dat
    let level_dat = world_path.join("level.dat");
    if level_dat.exists() {
        println!("导出 level.dat");
        export_level_dat(&level_dat, &output_path.join("level.json"), denoise)?;
    }

    // 导出所有维度
    for (dim_folder, dim_name) in DIMENSIONS {
        let (region_path, region_output) = if dim_folder.is_empty() {
            (world_path.join("region"), output_path.join("region"))
        } else {
            (
                world_path.join(dim_folder).join("region"),
                output_path.join(dim_folder).join("region"),
            )
        };

        if !region_path.exists() {
            continue;
        }

        let mca_files: Vec<_> = fs::read_dir(&region_path)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "mca"))
            .collect();

        if mca_files.is_empty() {
            continue;
        }

        println!("导出 {} ({} 个 region 文件)", dim_name, mca_files.len());

        mca_files.par_iter().for_each(|entry| {
            let mca_path = entry.path();
            if let Err(e) = export_mca(&mca_path, &region_output, denoise, aggressive) {
                eprintln!("  失败 {:?}: {}", mca_path.file_name().unwrap(), e);
            } else {
                println!("  完成 {:?}", mca_path.file_name().unwrap());
            }
        });
    }

    println!("导出完成");
    Ok(())
}

/// 导出整个世界（使用配置）
pub fn export_world_with_config(
    world_path: &Path,
    output_path: &Path,
    denoise: bool,
    aggressive: bool,
    config: &Config,
) -> Result<()> {
    fs::create_dir_all(output_path)?;

    // 导出 level.dat
    let level_dat = world_path.join("level.dat");
    if level_dat.exists() {
        println!("导出 level.dat");
        export_level_dat_with_config(
            &level_dat,
            &output_path.join("level.json"),
            denoise,
            &config.denoise,
            &config.field_mapping,
        )?;
    }

    let denoise_config = Arc::new(config.denoise.clone());
    let export_config = Arc::new(config.export.clone());
    let field_mapper = Arc::new(FieldMapper::from_config(&config.field_mapping));

    // 导出所有维度
    for (dim_folder, dim_name) in DIMENSIONS {
        let (region_path, region_output) = if dim_folder.is_empty() {
            (world_path.join("region"), output_path.join("region"))
        } else {
            (
                world_path.join(dim_folder).join("region"),
                output_path.join(dim_folder).join("region"),
            )
        };

        if !region_path.exists() {
            continue;
        }

        let mca_files: Vec<_> = fs::read_dir(&region_path)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "mca"))
            .collect();

        if mca_files.is_empty() {
            continue;
        }

        println!("导出 {} ({} 个 region 文件)", dim_name, mca_files.len());

        mca_files.par_iter().for_each(|entry| {
            let mca_path = entry.path();
            if let Err(e) = export_mca_with_config(
                &mca_path,
                &region_output,
                denoise,
                aggressive,
                &denoise_config,
                &export_config,
                &field_mapper,
            ) {
                eprintln!("  失败 {:?}: {}", mca_path.file_name().unwrap(), e);
            } else {
                println!("  完成 {:?}", mca_path.file_name().unwrap());
            }
        });
    }

    println!("导出完成");
    Ok(())
}

/// 导出整个世界（使用配置，支持区域过滤）
pub fn export_world_with_area(
    world_path: &Path,
    output_path: &Path,
    denoise: bool,
    aggressive: bool,
    config: &Config,
    area: Option<&Area>,
) -> Result<()> {
    fs::create_dir_all(output_path)?;

    // 导出 level.dat
    let level_dat = world_path.join("level.dat");
    if level_dat.exists() {
        println!("导出 level.dat");
        export_level_dat_with_config(
            &level_dat,
            &output_path.join("level.json"),
            denoise,
            &config.denoise,
            &config.field_mapping,
        )?;
    }

    if let Some(a) = area {
        println!(
            "工作区域: ({}, {}) ~ ({}, {})",
            a.min.x as i32, a.min.z as i32, a.max.x as i32, a.max.z as i32
        );
    }

    let denoise_config = Arc::new(config.denoise.clone());
    let export_config = Arc::new(config.export.clone());
    let field_mapper = Arc::new(FieldMapper::from_config(&config.field_mapping));

    // 导出所有维度
    for (dim_folder, dim_name) in DIMENSIONS {
        let (region_path, region_output) = if dim_folder.is_empty() {
            (world_path.join("region"), output_path.join("region"))
        } else {
            (
                world_path.join(dim_folder).join("region"),
                output_path.join(dim_folder).join("region"),
            )
        };

        if !region_path.exists() {
            continue;
        }

        let mca_files: Vec<_> = fs::read_dir(&region_path)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "mca"))
            .filter(|e| {
                // 如果有区域过滤，跳过不在区域内的 region
                if let Some(area) = area {
                    let filename = e.path();
                    let filename = filename.file_name().unwrap().to_str().unwrap();
                    if let Some((rx, rz)) = parse_mca_filename(filename) {
                        return area.may_contain_region(rx, rz);
                    }
                }
                true
            })
            .collect();

        if mca_files.is_empty() {
            continue;
        }

        println!("导出 {} ({} 个 region 文件)", dim_name, mca_files.len());

        mca_files.par_iter().for_each(|entry| {
            let mca_path = entry.path();
            if let Err(e) = export_mca_with_config(
                &mca_path,
                &region_output,
                denoise,
                aggressive,
                &denoise_config,
                &export_config,
                &field_mapper,
            ) {
                eprintln!("  失败 {:?}: {}", mca_path.file_name().unwrap(), e);
            } else {
                println!("  完成 {:?}", mca_path.file_name().unwrap());
            }
        });
    }

    println!("导出完成");
    Ok(())
}

/// 导出 level.dat 文件（使用默认去噪字段）
pub fn export_level_dat(level_path: &Path, output_path: &Path, denoise: bool) -> Result<()> {
    let file = File::open(level_path)?;
    let mut decoder = flate2::read::GzDecoder::new(file);
    let mut data = Vec::new();
    decoder.read_to_end(&mut data)?;

    let mut value: Value = fastnbt::from_bytes(&data)?;

    if denoise {
        denoise_level(&mut value);
    }

    let json = json!({
        "_gzip": 1,
        "_data": nbt_to_json(&value)
    });

    let output = serde_json::to_string_pretty(&json)?;
    fs::write(output_path, output)?;
    Ok(())
}

/// 导出 level.dat 文件（使用配置）
pub fn export_level_dat_with_config(
    level_path: &Path,
    output_path: &Path,
    denoise: bool,
    denoise_config: &DenoiseConfig,
    field_mapping_config: &FieldMappingConfig,
) -> Result<()> {
    let file = File::open(level_path)?;
    let mut decoder = flate2::read::GzDecoder::new(file);
    let mut data = Vec::new();
    decoder.read_to_end(&mut data)?;

    let mut value: Value = fastnbt::from_bytes(&data)?;

    if denoise {
        denoise_level_with_config(&mut value, denoise_config);
    }

    let mut json_data = nbt_to_json(&value);

    // 应用字段名映射
    let mapper = FieldMapper::from_config(field_mapping_config);
    mapper.shorten_json_keys(&mut json_data);

    let json = json!({
        "_gzip": 1,
        "_data": json_data
    });

    let output = serde_json::to_string_pretty(&json)?;
    fs::write(output_path, output)?;
    Ok(())
}

/// 单个切片的最大大小（字节）
const MAX_SLICE_SIZE: usize = 8 * 1024 * 1024; // 8MB

/// 导出单个 MCA 文件（使用默认去噪字段）
/// 超过 8MB 自动切片
pub fn export_mca(
    mca_path: &Path,
    output_dir: &Path,
    denoise: bool,
    aggressive: bool,
) -> Result<()> {
    let filename = mca_path.file_name().unwrap().to_str().unwrap();
    let (rx, rz) = parse_mca_filename(filename).context("无效的 MCA 文件名")?;

    let mut chunks = read_mca(mca_path)?;
    if chunks.is_empty() {
        return Ok(());
    }

    fs::create_dir_all(output_dir)?;

    let mut all_chunks = Vec::new();

    for chunk in &mut chunks {
        // 跳过非完整区块
        if !is_full_chunk(&chunk.data) {
            continue;
        }

        if denoise {
            denoise_chunk(&mut chunk.data, aggressive);
        }

        let mut json = nbt_to_json(&chunk.data);
        // 添加坐标到 JSON
        if let JsonValue::Object(ref mut obj) = json {
            obj.insert("x".to_string(), json!(chunk.x));
            obj.insert("z".to_string(), json!(chunk.z));
        }

        // 过滤空 sections 和空值
        filter_empty_sections(&mut json);
        filter_empty_values(&mut json);

        // 跳过没有实际数据的区块
        if !has_chunk_data(&json) {
            continue;
        }

        // 缩短字段名（最后一步，在所有检查之后）
        shorten_json_keys(&mut json);

        all_chunks.push(json);
    }

    if all_chunks.is_empty() {
        return Ok(());
    }

    // 按大小切片写入
    write_region_sliced(output_dir, rx, rz, &all_chunks)?;

    Ok(())
}

/// 导出单个 MCA 文件（使用配置）
pub fn export_mca_with_config(
    mca_path: &Path,
    output_dir: &Path,
    denoise: bool,
    aggressive: bool,
    denoise_config: &DenoiseConfig,
    export_config: &ExportConfig,
    field_mapper: &FieldMapper,
) -> Result<()> {
    let filename = mca_path.file_name().unwrap().to_str().unwrap();
    let (rx, rz) = parse_mca_filename(filename).context("无效的 MCA 文件名")?;

    let mut chunks = read_mca(mca_path)?;
    if chunks.is_empty() {
        return Ok(());
    }

    fs::create_dir_all(output_dir)?;

    let mut all_chunks = Vec::new();

    for chunk in &mut chunks {
        // 跳过非完整区块
        if !is_full_chunk(&chunk.data) {
            continue;
        }

        if denoise {
            denoise_chunk_with_config(&mut chunk.data, aggressive, denoise_config);
        }

        let mut json = nbt_to_json(&chunk.data);
        // 添加坐标到 JSON
        if let JsonValue::Object(ref mut obj) = json {
            obj.insert("x".to_string(), json!(chunk.x));
            obj.insert("z".to_string(), json!(chunk.z));
        }

        // 过滤空 sections 和空值
        filter_empty_sections(&mut json);
        filter_empty_values(&mut json);

        // 跳过没有实际数据的区块（可配置）
        if export_config.skip_empty_chunks && !has_chunk_data(&json) {
            continue;
        }

        // 缩短字段名（最后一步，在所有检查之后）
        field_mapper.shorten_json_keys(&mut json);

        all_chunks.push(json);
    }

    if all_chunks.is_empty() {
        return Ok(());
    }

    // 按大小切片写入
    write_region_sliced(output_dir, rx, rz, &all_chunks)?;

    Ok(())
}

/// 按大小切片写入 region 文件
fn write_region_sliced(output_dir: &Path, rx: i32, rz: i32, chunks: &[JsonValue]) -> Result<()> {
    // 序列化所有区块
    let serialized: Vec<String> = chunks
        .iter()
        .map(|c| serde_json::to_string(c).unwrap_or_default())
        .collect();

    let mut slice_id = 0;
    let mut current_slice: Vec<&str> = Vec::new();
    let mut current_size = 0usize;

    for chunk_str in &serialized {
        let chunk_size = chunk_str.len();

        // 如果当前切片加上这个区块会超过限制，先写入当前切片
        if !current_slice.is_empty() && current_size + chunk_size > MAX_SLICE_SIZE {
            let file_path = output_dir.join(format!("r.{}.{}.{}.json", rx, rz, slice_id));
            write_chunks_direct(&file_path, &current_slice)?;
            slice_id += 1;
            current_slice.clear();
            current_size = 0;
        }

        current_slice.push(chunk_str);
        current_size += chunk_size;
    }

    // 写入最后一个切片
    if !current_slice.is_empty() {
        let file_path = output_dir.join(format!("r.{}.{}.{}.json", rx, rz, slice_id));
        write_chunks_direct(&file_path, &current_slice)?;
    }

    Ok(())
}

/// 直接写入已序列化的区块
fn write_chunks_direct(path: &Path, chunks: &[&str]) -> Result<()> {
    let total_size: usize = chunks.iter().map(|s| s.len()).sum();
    let mut output = String::with_capacity(total_size + 100);

    output.push_str("{\"chunks\":[\n");
    for (i, chunk) in chunks.iter().enumerate() {
        output.push_str(chunk);
        if i < chunks.len() - 1 {
            output.push(',');
        }
        output.push('\n');
    }
    output.push_str("]}\n");

    fs::write(path, output)?;
    Ok(())
}

/// 检查区块是否完整生成
fn is_full_chunk(data: &Value) -> bool {
    if let Value::Compound(map) = data {
        if let Some(Value::String(status)) = map.get("Status") {
            // 只导出完整的区块
            return status == "minecraft:full" || status == "full";
        }
    }
    false
}

/// 过滤空 sections（只有空气的 section）
fn filter_empty_sections(chunk: &mut JsonValue) {
    if let JsonValue::Object(ref mut obj) = chunk {
        if let Some(JsonValue::Array(sections)) = obj.get_mut("sections") {
            sections.retain(|sec| !is_empty_section(sec));
        }
    }
}

/// 检查 section 是否为空（只有空气，不管 biome）
fn is_empty_section(sec: &JsonValue) -> bool {
    if let JsonValue::Object(obj) = sec {
        // 只检查 block_states 是否为空气
        if let Some(block_states) = obj.get("block_states") {
            // 如果有 data 字段，说明不是简单的单一方块
            if block_states.get("data").is_some() {
                return false;
            }
            if let Some(palette) = block_states.get("palette") {
                if let JsonValue::Array(arr) = palette {
                    // palette 只有一个元素且是空气
                    if arr.len() == 1 {
                        if let Some(first) = arr.first() {
                            let name = first.get("Name").and_then(|n| n.as_str()).unwrap_or("");
                            return name == "air" || name == "minecraft:air";
                        }
                    }
                }
            }
        }
        false
    } else {
        false
    }
}

/// 过滤空值（空对象、空列表、空列表标记）
fn filter_empty_values(value: &mut JsonValue) {
    match value {
        JsonValue::Object(obj) => {
            // 递归处理所有值
            for v in obj.values_mut() {
                filter_empty_values(v);
            }
            // 移除空值
            obj.retain(|_, v| !is_empty_json_value(v));
        }
        JsonValue::Array(arr) => {
            for v in arr.iter_mut() {
                filter_empty_values(v);
            }
        }
        _ => {}
    }
}

/// 检查 JSON 值是否为空
fn is_empty_json_value(v: &JsonValue) -> bool {
    match v {
        JsonValue::Object(obj) => {
            // 空列表标记 {"[]": "End"}
            if obj.len() == 1 && obj.contains_key("[]") {
                return true;
            }
            obj.is_empty()
        }
        JsonValue::Array(arr) => arr.is_empty(),
        _ => false,
    }
}

/// 检查区块是否有实际数据（sections 或 block_entities）
fn has_chunk_data(chunk: &JsonValue) -> bool {
    if let JsonValue::Object(obj) = chunk {
        // 检查 sections 是否有内容
        if let Some(JsonValue::Array(sections)) = obj.get("sections") {
            if !sections.is_empty() {
                return true;
            }
        }
        // 检查 block_entities 是否有内容
        if let Some(JsonValue::Array(entities)) = obj.get("block_entities") {
            if !entities.is_empty() {
                return true;
            }
        }
    }
    false
}
