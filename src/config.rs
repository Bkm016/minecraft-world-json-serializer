//! 配置文件加载与管理

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// 主配置结构
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// 导出配置
    pub export: ExportConfig,
    /// 还原配置
    pub restore: RestoreConfig,
    /// 去噪配置
    pub denoise: DenoiseConfig,
}

/// 导出配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ExportConfig {
    /// 默认启用去噪
    pub denoise: bool,
    /// 默认启用激进模式
    pub aggressive: bool,
}

/// 还原配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RestoreConfig {
    /// 默认恢复默认值
    pub restore_defaults: bool,
}

/// 去噪配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DenoiseConfig {
    /// 区块级去噪配置
    pub chunk: ChunkDenoiseConfig,
    /// 存档级去噪配置
    pub level: LevelDenoiseConfig,
}

/// 区块级去噪配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ChunkDenoiseConfig {
    /// 普通去噪字段
    pub fields: Vec<String>,
    /// 激进去噪字段
    pub aggressive_fields: Vec<String>,
}

/// 存档级去噪配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LevelDenoiseConfig {
    /// 去噪字段
    pub fields: Vec<String>,
    /// 重置天气状态
    pub reset_weather: bool,
}

// ============== 默认值 ==============

impl Default for Config {
    fn default() -> Self {
        Self {
            export: ExportConfig::default(),
            restore: RestoreConfig::default(),
            denoise: DenoiseConfig::default(),
        }
    }
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            denoise: true,
            aggressive: false,
        }
    }
}

impl Default for RestoreConfig {
    fn default() -> Self {
        Self {
            restore_defaults: true,
        }
    }
}

impl Default for DenoiseConfig {
    fn default() -> Self {
        Self {
            chunk: ChunkDenoiseConfig::default(),
            level: LevelDenoiseConfig::default(),
        }
    }
}

impl Default for ChunkDenoiseConfig {
    fn default() -> Self {
        Self {
            fields: vec![
                "LastUpdate".to_string(),
                "InhabitedTime".to_string(),
                "blending_data".to_string(),
                "PostProcessing".to_string(),
                "isLightOn".to_string(),
            ],
            aggressive_fields: vec!["Heightmaps".to_string()],
        }
    }
}

impl Default for LevelDenoiseConfig {
    fn default() -> Self {
        Self {
            fields: vec![
                "Time".to_string(),
                "DayTime".to_string(),
                "LastPlayed".to_string(),
                "thunderTime".to_string(),
                "rainTime".to_string(),
                "clearWeatherTime".to_string(),
                "WanderingTraderSpawnChance".to_string(),
                "WanderingTraderSpawnDelay".to_string(),
                "WanderingTraderId".to_string(),
                "ServerBrands".to_string(),
                "WasModded".to_string(),
            ],
            reset_weather: true,
        }
    }
}

// ============== 配置加载 ==============

impl Config {
    /// 从文件加载配置
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    /// 保存配置到文件
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)?;
        Ok(())
    }

    /// 获取默认配置文件路径
    pub fn default_config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("mcj").join("config.toml"))
    }

    /// 按优先级加载配置：
    /// 1. 当前目录的 mcj.toml
    /// 2. 用户配置目录的 config.toml
    /// 3. 默认配置
    pub fn load() -> Self {
        // 当前目录
        let local_config = Path::new("mcj.toml");
        if local_config.exists() {
            if let Ok(config) = Self::load_from_file(local_config) {
                eprintln!("已加载配置: mcj.toml");
                return config;
            }
        }

        // 用户配置目录
        if let Some(user_config) = Self::default_config_path() {
            if user_config.exists() {
                if let Ok(config) = Self::load_from_file(&user_config) {
                    eprintln!("已加载配置: {}", user_config.display());
                    return config;
                }
            }
        }

        // 默认配置
        Self::default()
    }

    /// 生成默认配置文件内容
    pub fn default_toml() -> String {
        let config = Self::default();
        toml::to_string_pretty(&config).unwrap_or_default()
    }
}
