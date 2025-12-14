//! 导出世界为 JSON 格式

use crate::config::{Config, DenoiseConfig};
use crate::denoise::{denoise_chunk, denoise_chunk_with_config, denoise_level, denoise_level_with_config};
use crate::mca::{parse_mca_filename, read_mca};
use crate::nbt_json::nbt_to_json;
use anyhow::{Context, Result};
use fastnbt::Value;
use rayon::prelude::*;
use serde_json::{json, Value as JsonValue};
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;
use std::sync::Arc;

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

    // 导出 region
    let region_path = world_path.join("region");
    if region_path.exists() {
        let region_output = output_path.join("region");

        let mca_files: Vec<_> = fs::read_dir(&region_path)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "mca"))
            .collect();

        println!("导出 {} 个 region 文件 (并行处理)", mca_files.len());

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
        export_level_dat_with_config(&level_dat, &output_path.join("level.json"), denoise, &config.denoise)?;
    }

    // 导出 region
    let region_path = world_path.join("region");
    if region_path.exists() {
        let region_output = output_path.join("region");

        let mca_files: Vec<_> = fs::read_dir(&region_path)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "mca"))
            .collect();

        println!("导出 {} 个 region 文件 (并行处理)", mca_files.len());

        let denoise_config = Arc::new(config.denoise.clone());

        mca_files.par_iter().for_each(|entry| {
            let mca_path = entry.path();
            if let Err(e) = export_mca_with_config(&mca_path, &region_output, denoise, aggressive, &denoise_config) {
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
    config: &DenoiseConfig,
) -> Result<()> {
    let file = File::open(level_path)?;
    let mut decoder = flate2::read::GzDecoder::new(file);
    let mut data = Vec::new();
    decoder.read_to_end(&mut data)?;

    let mut value: Value = fastnbt::from_bytes(&data)?;

    if denoise {
        denoise_level_with_config(&mut value, config);
    }

    let json = json!({
        "_gzip": 1,
        "_data": nbt_to_json(&value)
    });

    let output = serde_json::to_string_pretty(&json)?;
    fs::write(output_path, output)?;
    Ok(())
}

/// 导出单个 MCA 文件（使用默认去噪字段）
pub fn export_mca(mca_path: &Path, output_dir: &Path, denoise: bool, aggressive: bool) -> Result<()> {
    let filename = mca_path.file_name().unwrap().to_str().unwrap();
    let (rx, rz) = parse_mca_filename(filename).context("无效的 MCA 文件名")?;

    let mut chunks = read_mca(mca_path)?;
    if chunks.is_empty() {
        return Ok(());
    }

    let region_dir = output_dir.join(format!("r.{}.{}", rx, rz));
    fs::create_dir_all(&region_dir)?;

    for chunk in &mut chunks {
        if denoise {
            denoise_chunk(&mut chunk.data, aggressive);
        }

        let json = nbt_to_json(&chunk.data);
        let chunk_file = region_dir.join(format!("c.{}.{}.json", chunk.x, chunk.z));
        write_chunk_file(&chunk_file, &json)?;
    }

    Ok(())
}

/// 导出单个 MCA 文件（使用配置）
pub fn export_mca_with_config(
    mca_path: &Path,
    output_dir: &Path,
    denoise: bool,
    aggressive: bool,
    config: &DenoiseConfig,
) -> Result<()> {
    let filename = mca_path.file_name().unwrap().to_str().unwrap();
    let (rx, rz) = parse_mca_filename(filename).context("无效的 MCA 文件名")?;

    let mut chunks = read_mca(mca_path)?;
    if chunks.is_empty() {
        return Ok(());
    }

    let region_dir = output_dir.join(format!("r.{}.{}", rx, rz));
    fs::create_dir_all(&region_dir)?;

    for chunk in &mut chunks {
        if denoise {
            denoise_chunk_with_config(&mut chunk.data, aggressive, config);
        }

        let json = nbt_to_json(&chunk.data);
        let chunk_file = region_dir.join(format!("c.{}.{}.json", chunk.x, chunk.z));
        write_chunk_file(&chunk_file, &json)?;
    }

    Ok(())
}

/// 写入区块 JSON 文件（优化格式）
fn write_chunk_file(path: &Path, chunk: &JsonValue) -> Result<()> {
    let mut lines = vec!["{".to_string()];

    if let JsonValue::Object(obj) = chunk {
        let sections = obj.get("sections");

        // 写入非 sections 字段
        let mut keys: Vec<_> = obj.keys().filter(|k| *k != "sections").collect();
        keys.sort();

        for k in keys {
            let v = &obj[k];
            let compact = serde_json::to_string(v)?;
            lines.push(format!("\"{}\":{},", k, compact));
        }

        // 写入 sections
        if let Some(sections) = sections {
            match sections {
                JsonValue::Array(arr) => {
                    lines.push("\"sections\":[".to_string());
                    for (i, sec) in arr.iter().enumerate() {
                        let sec_line = format_section(sec)?;
                        let comma = if i < arr.len() - 1 { "," } else { "" };
                        lines.push(format!("{}{}", sec_line, comma));
                    }
                    lines.push("]".to_string());
                }
                _ => {
                    let compact = serde_json::to_string(sections)?;
                    lines.push(format!("\"sections\":{}", compact));
                }
            }
        }
    }

    lines.push("}".to_string());
    fs::write(path, lines.join("\n") + "\n")?;
    Ok(())
}

/// 格式化单个 section，关键字段分行显示
fn format_section(sec: &JsonValue) -> Result<String> {
    if let JsonValue::Object(obj) = sec {
        // 定义字段顺序：Y 优先，然后按字母序
        let priority_keys = ["Y"];
        let mut lines = Vec::new();
        let mut first = true;

        // 先写 Y
        for key in &priority_keys {
            if let Some(v) = obj.get(*key) {
                let prefix = if first { "{" } else { "" };
                first = false;
                lines.push(format!("{}\"{}\":{}", prefix, key, serde_json::to_string(v)?));
            }
        }

        // 再按字母序写其他字段
        let mut other_keys: Vec<_> = obj.keys()
            .filter(|k| !priority_keys.contains(&k.as_str()))
            .collect();
        other_keys.sort();

        for key in other_keys {
            let v = &obj[key];
            let prefix = if first { "{" } else { "" };
            first = false;
            lines.push(format!("{}\"{}\":{}", prefix, key, serde_json::to_string(v)?));
        }

        if lines.is_empty() {
            Ok("{}".to_string())
        } else {
            Ok(lines.join(",\n") + "}")
        }
    } else {
        Ok(serde_json::to_string(sec)?)
    }
}
