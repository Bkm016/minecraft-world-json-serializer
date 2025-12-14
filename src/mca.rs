//! MCA 区域文件解析与写入

use anyhow::Result;
use fastnbt::Value;
use regex::Regex;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;

/// 扇区大小（字节）
pub const SECTOR_SIZE: usize = 4096;

/// 区块数据
pub struct ChunkData {
    pub x: i32,
    pub z: i32,
    pub data: Value,
}

/// 读取 MCA 文件中的所有区块
pub fn read_mca(path: &Path) -> Result<Vec<ChunkData>> {
    let mut file = File::open(path)?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;

    if data.len() < SECTOR_SIZE * 2 {
        return Ok(vec![]);
    }

    let mut chunks = Vec::new();

    for i in 0..1024 {
        let offset =
            u32::from_be_bytes([0, data[i * 4], data[i * 4 + 1], data[i * 4 + 2]]) as usize;
        let sector_count = data[i * 4 + 3] as usize;

        if offset == 0 || sector_count == 0 {
            continue;
        }

        let x = (i % 32) as i32;
        let z = (i / 32) as i32;

        let chunk_offset = offset * SECTOR_SIZE;
        if chunk_offset + 5 > data.len() {
            continue;
        }

        let length = u32::from_be_bytes([
            data[chunk_offset],
            data[chunk_offset + 1],
            data[chunk_offset + 2],
            data[chunk_offset + 3],
        ]) as usize;

        let compression = data[chunk_offset + 4];

        if chunk_offset + 5 + length - 1 > data.len() {
            continue;
        }

        let compressed = &data[chunk_offset + 5..chunk_offset + 4 + length];

        let nbt_data = match compression {
            1 => {
                // Gzip
                let mut decoder = flate2::read::GzDecoder::new(compressed);
                let mut decompressed = Vec::new();
                decoder.read_to_end(&mut decompressed)?;
                decompressed
            }
            2 => {
                // Zlib
                let mut decoder = flate2::read::ZlibDecoder::new(compressed);
                let mut decompressed = Vec::new();
                decoder.read_to_end(&mut decompressed)?;
                decompressed
            }
            3 => compressed.to_vec(), // 无压缩
            _ => continue,
        };

        match fastnbt::from_bytes::<Value>(&nbt_data) {
            Ok(value) => chunks.push(ChunkData { x, z, data: value }),
            Err(e) => eprintln!("警告: 无法解析区块 ({}, {}): {}", x, z, e),
        }
    }

    Ok(chunks)
}

/// 将区块数据写入 MCA 文件
pub fn write_mca(path: &Path, chunks: &[ChunkData]) -> Result<()> {
    if chunks.is_empty() {
        return Ok(());
    }

    let mut locations = vec![0u8; SECTOR_SIZE];
    let timestamps = vec![0u8; SECTOR_SIZE];
    let mut chunk_sectors: Vec<Vec<u8>> = Vec::new();
    let mut current_sector = 2u32;

    for chunk in chunks {
        let nbt_data = fastnbt::to_bytes(&chunk.data)?;

        // Zlib 压缩
        let mut encoder =
            flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(&nbt_data)?;
        let compressed = encoder.finish()?;

        let chunk_length = compressed.len() + 5;
        let sector_count = (chunk_length + SECTOR_SIZE - 1) / SECTOR_SIZE;

        // 构建 chunk 数据
        let mut chunk_data = Vec::with_capacity(sector_count * SECTOR_SIZE);
        chunk_data.extend_from_slice(&((compressed.len() + 1) as u32).to_be_bytes());
        chunk_data.push(2); // Zlib
        chunk_data.extend_from_slice(&compressed);
        chunk_data.resize(sector_count * SECTOR_SIZE, 0);

        // 写入位置表
        let index = (chunk.x & 31) + (chunk.z & 31) * 32;
        let idx = index as usize * 4;
        let offset_bytes = current_sector.to_be_bytes();
        locations[idx] = offset_bytes[1];
        locations[idx + 1] = offset_bytes[2];
        locations[idx + 2] = offset_bytes[3];
        locations[idx + 3] = sector_count as u8;

        chunk_sectors.push(chunk_data);
        current_sector += sector_count as u32;
    }

    // 写入文件
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = File::create(path)?;
    file.write_all(&locations)?;
    file.write_all(&timestamps)?;
    for sector in chunk_sectors {
        file.write_all(&sector)?;
    }

    Ok(())
}

/// 解析 MCA 文件名，返回 (rx, rz)
pub fn parse_mca_filename(filename: &str) -> Option<(i32, i32)> {
    let re = Regex::new(r"r\.(-?\d+)\.(-?\d+)\.mca").ok()?;
    let caps = re.captures(filename)?;
    let rx = caps.get(1)?.as_str().parse().ok()?;
    let rz = caps.get(2)?.as_str().parse().ok()?;
    Some((rx, rz))
}
