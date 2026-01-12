//! Minecraft 世界 JSON 序列化工具 - 用于 Git 存储

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use mcj::{export_world_with_config, restore_world_with_config, Config};

/// Minecraft 世界 JSON 序列化工具 - 用于 Git 存储
#[derive(Parser)]
#[command(name = "mcj", version, about)]
struct Cli {
    /// 配置文件路径
    #[arg(short, long, global = true)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 导出世界为 JSON 格式
    Export {
        /// 世界文件夹路径
        world: PathBuf,
        /// 输出文件夹路径
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// 覆盖已存在的输出目录
        #[arg(long, visible_alias = "override")]
        overwrite: bool,
        /// 禁用去噪声处理
        #[arg(long)]
        no_denoise: bool,
        /// 禁用激进去噪（默认启用）
        #[arg(long)]
        no_aggressive: bool,
    },
    /// 从 JSON 还原世界
    Restore {
        /// JSON 文件夹路径
        json_dir: PathBuf,
        /// 输出世界文件夹路径
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// 不恢复默认值
        #[arg(long)]
        no_restore_defaults: bool,
    },
    /// 克隆世界（经过去噪处理）
    Clone {
        /// 源世界文件夹
        source: PathBuf,
        /// 目标世界文件夹
        dest: PathBuf,
        /// 保留中间 JSON 到指定目录
        #[arg(long)]
        json_dir: Option<PathBuf>,
        /// 禁用去噪声处理
        #[arg(long)]
        no_denoise: bool,
        /// 禁用激进去噪（默认启用）
        #[arg(long)]
        no_aggressive: bool,
    },
    /// 生成默认配置文件
    Config {
        /// 输出路径（默认: mcj.toml）
        #[arg(short, long, default_value = "mcj.toml")]
        output: PathBuf,
        /// 覆盖已存在的文件
        #[arg(long)]
        force: bool,
    },
}

fn load_config(config_path: Option<PathBuf>) -> Config {
    if let Some(path) = config_path {
        match Config::load_from_file(&path) {
            Ok(config) => {
                eprintln!("已加载配置: {}", path.display());
                return config;
            }
            Err(e) => {
                eprintln!("警告: 无法加载配置 {}: {}", path.display(), e);
            }
        }
    }
    Config::load()
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = load_config(cli.config);

    match cli.command {
        Commands::Export {
            world,
            output,
            overwrite,
            no_denoise,
            no_aggressive,
        } => {
            let output_path = output.unwrap_or_else(|| {
                let mut p = world.clone();
                p.set_file_name(format!(
                    "{}_json",
                    world.file_name().unwrap().to_str().unwrap()
                ));
                p
            });

            // 检查输出目录
            if output_path.exists() {
                if overwrite {
                    // 只清理导出会生成的内容，保留 .git 等
                    let level_json = output_path.join("level.json");
                    if level_json.exists() {
                        fs::remove_file(&level_json)?;
                    }
                    let region_dir = output_path.join("region");
                    if region_dir.exists() {
                        fs::remove_dir_all(&region_dir)?;
                    }
                } else {
                    anyhow::bail!("输出目录已存在: {:?}\n使用 --overwrite 覆盖", output_path);
                }
            }

            // 使用配置默认值，命令行参数优先
            let do_denoise = if no_denoise {
                false
            } else {
                config.export.denoise
            };
            let do_aggressive = if no_aggressive { false } else { true }; // 默认启用激进模式

            println!("导出世界: {:?}", world);
            println!("输出目录: {:?}", output_path);
            println!("去噪声: {}", if do_denoise { "是" } else { "否" });
            if do_denoise {
                println!("激进模式: {}", if do_aggressive { "是" } else { "否" });
            }
            println!();

            let start = Instant::now();
            export_world_with_config(&world, &output_path, do_denoise, do_aggressive, &config)?;
            println!("\n耗时: {:.2}s", start.elapsed().as_secs_f64());
        }

        Commands::Restore {
            json_dir,
            output,
            no_restore_defaults,
        } => {
            let output_path = output.unwrap_or_else(|| {
                let mut p = json_dir.clone();
                p.set_file_name(format!(
                    "{}_restored",
                    json_dir.file_name().unwrap().to_str().unwrap()
                ));
                p
            });

            // 使用配置默认值，命令行参数优先
            let do_restore_defaults = if no_restore_defaults {
                false
            } else {
                config.restore.restore_defaults
            };

            println!("还原 JSON: {:?}", json_dir);
            println!("输出目录: {:?}", output_path);
            println!(
                "恢复默认值: {}",
                if do_restore_defaults { "是" } else { "否" }
            );
            println!();

            let start = Instant::now();
            restore_world_with_config(&json_dir, &output_path, do_restore_defaults, &config)?;
            println!("\n耗时: {:.2}s", start.elapsed().as_secs_f64());
        }

        Commands::Clone {
            source,
            dest,
            json_dir,
            no_denoise,
            no_aggressive,
        } => {
            if dest.exists() {
                anyhow::bail!("目标路径已存在: {:?}", dest);
            }

            // 使用配置默认值，命令行参数优先
            let do_denoise = if no_denoise {
                false
            } else {
                config.export.denoise
            };
            let do_aggressive = if no_aggressive { false } else { true }; // 默认启用激进模式

            println!("克隆世界: {:?}", source);
            println!("目标位置: {:?}", dest);
            println!("去噪声: {}", if do_denoise { "是" } else { "否" });
            if do_denoise {
                println!("激进模式: {}", if do_aggressive { "是" } else { "否" });
            }
            println!();

            let start = Instant::now();

            let temp_dir = json_dir.clone().unwrap_or_else(|| {
                std::env::temp_dir().join(format!("mcj_{}", std::process::id()))
            });
            let use_temp = json_dir.is_none();

            println!("========================================");
            println!("步骤 1/2: 导出为 JSON");
            println!("========================================");
            export_world_with_config(&source, &temp_dir, do_denoise, do_aggressive, &config)?;

            println!();
            println!("========================================");
            println!("步骤 2/2: 还原为世界");
            println!("========================================");
            restore_world_with_config(&temp_dir, &dest, config.restore.restore_defaults, &config)?;

            if use_temp {
                let _ = fs::remove_dir_all(&temp_dir);
            }

            println!("\n克隆完成! 总耗时: {:.2}s", start.elapsed().as_secs_f64());
            if json_dir.is_some() {
                println!("JSON 已保留在: {:?}", temp_dir);
            }
        }

        Commands::Config { output, force } => {
            if output.exists() && !force {
                anyhow::bail!("文件已存在: {:?}\n使用 --force 覆盖", output);
            }

            let default_config = Config::default();
            default_config.save_to_file(&output)?;
            println!("已生成配置文件: {:?}", output);
            println!("\n配置项说明:");
            println!("  [export]");
            println!(
                "    denoise = {}      # 默认启用去噪",
                default_config.export.denoise
            );
            println!(
                "    aggressive = {}   # 默认启用激进模式",
                default_config.export.aggressive
            );
            println!("  [restore]");
            println!(
                "    restore_defaults = {}  # 默认恢复默认值",
                default_config.restore.restore_defaults
            );
            println!("  [denoise.chunk]");
            println!("    fields = [...]         # 区块去噪字段");
            println!("    aggressive_fields = [...]  # 激进去噪字段");
            println!("  [denoise.level]");
            println!("    fields = [...]         # 存档去噪字段");
            println!(
                "    reset_weather = {}     # 重置天气",
                default_config.denoise.level.reset_weather
            );
        }
    }

    Ok(())
}
