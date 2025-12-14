# mcj - Minecraft World JSON Serializer

将 Minecraft 世界文件转换为 Git 友好的 JSON 格式，支持双向转换。

## 功能特性

- **导出**: 将 Minecraft 世界（level.dat + region/*.mca）转换为 JSON
- **还原**: 从 JSON 重建完整的 Minecraft 世界
- **克隆**: 一步完成导出→还原，生成去噪后的干净世界副本
- **去噪处理**: 自动移除运行时变化的字段，确保 Git diff 干净
- **并行处理**: 利用多核 CPU 加速处理
- **可配置**: 支持自定义去噪字段和默认行为

## 编译

```bash
cargo build --release
```

编译产物位于 `target/release/mcj`（Linux/macOS）或 `target/release/mcj.exe`（Windows）

## 使用方法

### 导出世界

```bash
mcj export <世界路径> [-o <输出路径>]

# 示例
mcj export ./world -o ./world_json

# 禁用去噪
mcj export ./world --no-denoise

# 激进去噪（移除更多字段如 Heightmaps）
mcj export ./world --aggressive
```

### 还原世界

```bash
mcj restore <JSON路径> [-o <输出路径>]

# 示例
mcj restore ./world_json -o ./world_restored

# 不恢复默认值
mcj restore ./world_json --no-restore-defaults
```

### 克隆世界

```bash
mcj clone <源世界> <目标位置>

# 示例
mcj clone ./world ./world_clean

# 保留中间 JSON 文件
mcj clone ./world ./world_clean --json-dir ./world_json
```

### 生成配置文件

```bash
mcj config                    # 生成 mcj.toml
mcj config -o custom.toml     # 指定输出路径
mcj config --force            # 覆盖已存在的文件
```

## 配置文件

mcj 按以下优先级查找配置：

1. 命令行 `-c <path>` 指定的配置
2. 当前目录的 `mcj.toml`
3. 用户配置目录 `~/.config/mcj/config.toml`
4. 内置默认值

配置文件示例（`mcj.toml`）：

```toml
[export]
denoise = true       # 默认启用去噪
aggressive = false   # 默认不启用激进模式

[restore]
restore_defaults = true  # 默认恢复被去除的字段

[denoise.chunk]
fields = [
    "LastUpdate",
    "InhabitedTime",
    "blending_data",
    "PostProcessing",
    "isLightOn",
]
aggressive_fields = ["Heightmaps"]

[denoise.level]
fields = [
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
]
reset_weather = true  # 重置天气状态
```

## 输出格式

```
world_json/
├── level.json          # 存档元数据
└── region/
    └── r.{rx}.{rz}/    # 每个 region 一个目录
        ├── c.0.0.json  # 每个 chunk 一个文件
        ├── c.0.1.json
        └── ...
```

### JSON 类型编码

NBT 类型通过后缀/前缀映射到 JSON：

| NBT 类型 | JSON 表示 | 示例 |
|----------|-----------|------|
| Byte | `"<n>b"` | `"1b"`, `"-128b"` |
| Short | `"<n>s"` | `"32767s"` |
| Int | `<n>` | `42` |
| Long | `"<n>L"` | `"9223372036854775807L"` |
| Float | `"<n>f"` | `"3.14f"` |
| Double | `<n>` | `3.14159` |
| String | `"<s>"` | `"hello"` |
| ByteArray | `"B;<base64>"` | `"B;SGVsbG8="` |
| IntArray | `"I;<base64>"` | `"I;AAAABQ=="` |
| LongArray | `"L;<base64>"` | `"L;AAAAAAAAABQ="` |
| List (empty) | `{"[]": "End"}` | `{"[]": "End"}` |
| List | `[...]` | `["1b", "2b"]` |
| Compound | `{...}` | `{"key": "value"}` |

## 去噪处理

去噪会移除运行时频繁变化但不影响游戏内容的字段：

**区块级别**（每次游戏运行都会变化）：
- `LastUpdate` - 区块最后更新时间
- `InhabitedTime` - 玩家在区块内的累计时间
- `isLightOn` - 光照计算状态
- `PostProcessing` - 待处理任务
- `blending_data` - 区块混合数据

**激进模式额外移除**：
- `Heightmaps` - 高度图（可由游戏重新计算）

**存档级别**：
- `Time`, `DayTime` - 游戏时间
- `LastPlayed` - 最后游玩时间
- 天气相关计时器
- 流浪商人数据

## 跨平台编译

```bash
# Windows
cargo build --release --target x86_64-pc-windows-msvc

# Linux
cargo build --release --target x86_64-unknown-linux-gnu

# macOS
cargo build --release --target x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin  # Apple Silicon
```

## 许可证

MIT
