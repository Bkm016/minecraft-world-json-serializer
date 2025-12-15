//! 从 JSON 还原世界

use crate::denoise::restore_defaults;
use crate::mca::{write_mca, ChunkData};
use crate::nbt_json::json_to_nbt;
use anyhow::{Context, Result};
use rayon::prelude::*;
use regex::Regex;
use serde_json::Value as JsonValue;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

/// 还原整个世界
pub fn restore_world(
    json_path: &Path,
    output_path: &Path,
    restore_default_values: bool,
) -> Result<()> {
    fs::create_dir_all(output_path)?;

    // 还原 level.dat
    let level_json = json_path.join("level.json");
    if level_json.exists() {
        println!("还原 level.dat");
        restore_level_dat(&level_json, &output_path.join("level.dat"))?;
    }

    // 还原 region
    let region_json_path = json_path.join("region");
    if region_json_path.exists() {
        let region_output = output_path.join("region");
        fs::create_dir_all(&region_output)?;

        // 收集所有 region JSON 文件，按 (rx, rz) 分组
        // 支持切片格式: r.{rx}.{rz}.{id}.json
        let region_re = Regex::new(r"r\.(-?\d+)\.(-?\d+)\.(\d+)\.json")?;
        let mut region_files: std::collections::HashMap<(i32, i32), Vec<std::path::PathBuf>> =
            std::collections::HashMap::new();

        for entry in fs::read_dir(&region_json_path)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let filename = path.file_name().unwrap().to_str().unwrap();
            if let Some(caps) = region_re.captures(filename) {
                let rx: i32 = caps.get(1).unwrap().as_str().parse()?;
                let rz: i32 = caps.get(2).unwrap().as_str().parse()?;
                region_files.entry((rx, rz)).or_default().push(path);
            }
        }

        println!("还原 {} 个 region (并行处理)", region_files.len());

        let region_list: Vec<_> = region_files.into_iter().collect();
        region_list.par_iter().for_each(|((rx, rz), files)| {
            if let Err(e) = restore_region_slices(*rx, *rz, files, &region_output, restore_default_values) {
                eprintln!("  失败 r.{}.{}: {}", rx, rz, e);
            } else {
                println!("  完成 r.{}.{}", rx, rz);
            }
        });
    }

    println!("还原完成");
    Ok(())
}

/// 还原 level.dat 文件
pub fn restore_level_dat(json_path: &Path, output_path: &Path) -> Result<()> {
    let content = fs::read_to_string(json_path)?;
    let json: JsonValue = serde_json::from_str(&content)?;

    let data = json.get("_data").context("缺少 _data 字段")?;
    let value = json_to_nbt(data)?;

    let nbt_data = fastnbt::to_bytes(&value)?;

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let file = File::create(output_path)?;
    let mut encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    encoder.write_all(&nbt_data)?;
    encoder.finish()?;

    Ok(())
}

/// 从多个切片文件还原单个 region
pub fn restore_region_slices(
    rx: i32,
    rz: i32,
    files: &[std::path::PathBuf],
    output_dir: &Path,
    restore_default_values: bool,
) -> Result<()> {
    let mut chunks = Vec::new();

    for file_path in files {
        let content = fs::read_to_string(file_path)?;
        let json: JsonValue = serde_json::from_str(&content)?;

        let chunks_array = json
            .get("chunks")
            .and_then(|v| v.as_array())
            .context("缺少 chunks 数组")?;

        for chunk_json in chunks_array {
            let cx = chunk_json
                .get("x")
                .and_then(|v| v.as_i64())
                .context("区块缺少 x 坐标")? as i32;
            let cz = chunk_json
                .get("z")
                .and_then(|v| v.as_i64())
                .context("区块缺少 z 坐标")? as i32;

            // 移除 x, z 字段后转换为 NBT
            let mut chunk_obj = chunk_json.clone();
            if let JsonValue::Object(ref mut obj) = chunk_obj {
                obj.remove("x");
                obj.remove("z");
            }

            let mut value = json_to_nbt(&chunk_obj)?;

            if restore_default_values {
                restore_defaults(&mut value);
            }

            chunks.push(ChunkData {
                x: cx,
                z: cz,
                data: value,
            });
        }
    }

    if !chunks.is_empty() {
        let output_file = output_dir.join(format!("r.{}.{}.mca", rx, rz));
        write_mca(&output_file, &chunks)?;
    }

    Ok(())
}
