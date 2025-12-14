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

        let region_dirs: Vec<_> = fs::read_dir(&region_json_path)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path().is_dir()
                    && e.file_name()
                        .to_str()
                        .map_or(false, |n| n.starts_with("r."))
            })
            .collect();

        println!("还原 {} 个 region (并行处理)", region_dirs.len());

        region_dirs.par_iter().for_each(|entry| {
            let dir_path = entry.path();
            if let Err(e) = restore_region_dir(&dir_path, &region_output, restore_default_values) {
                eprintln!("  失败 {:?}: {}", dir_path.file_name().unwrap(), e);
            } else {
                println!("  完成 {:?}", dir_path.file_name().unwrap());
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

/// 还原单个 region 目录
pub fn restore_region_dir(
    region_dir: &Path,
    output_dir: &Path,
    restore_default_values: bool,
) -> Result<()> {
    let dir_name = region_dir.file_name().unwrap().to_str().unwrap();
    let re = Regex::new(r"r\.(-?\d+)\.(-?\d+)")?;
    let caps = re.captures(dir_name).context("无效的 region 目录名")?;
    let rx: i32 = caps.get(1).unwrap().as_str().parse()?;
    let rz: i32 = caps.get(2).unwrap().as_str().parse()?;

    let mut chunks = Vec::new();
    let chunk_re = Regex::new(r"c\.(-?\d+)\.(-?\d+)\.json")?;

    for entry in fs::read_dir(region_dir)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let filename = path.file_name().unwrap().to_str().unwrap();
        if let Some(caps) = chunk_re.captures(filename) {
            let cx: i32 = caps.get(1).unwrap().as_str().parse()?;
            let cz: i32 = caps.get(2).unwrap().as_str().parse()?;

            let content = fs::read_to_string(&path)?;
            let json: JsonValue = serde_json::from_str(&content)?;
            let mut value = json_to_nbt(&json)?;

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
