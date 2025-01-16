# IP 地理位置服务

[English](README_EN.md) | 简体中文

基于 Rust 构建的高性能 IP 地理位置服务，为 IP 地址和域名提供详细的地理和网络信息。

## 特性

- 🌍 IP 地理位置查询，提供详细信息
- 🏙️ 支持中国和国际地区位置查询
- 🔄 ASN（自治系统编号）信息
- 🌐 支持 IPv4 和 IPv6 地址
- 🚀 基于高性能 Axum web 框架
- 🗺️ 多数据库支持（GeoCN.mmdb、GeoLite2-City.mmdb、GeoLite2-ASN.mmdb）
- 🌐 RESTful API 接口
- 🔍 自动域名解析（支持 A 和 AAAA 记录）

## 环境要求

- Rust 2021 edition 或更高版本
- MaxMind GeoIP2 数据库（GeoCN.mmdb、GeoLite2-City.mmdb、GeoLite2-ASN.mmdb）

## 安装

1. 克隆仓库：
```bash
git clone https://github.com/upteka/ipgeo-api-rust.git
cd ipgeo-api-rust
```

2. 构建项目：
```bash
cargo build --release
```

## 配置

服务会在 `MMDB_PATH` 环境变量指定的目录中查找 MaxMind 数据库文件。如果未设置，默认使用当前目录。

需要的数据库文件：
- `GeoCN.mmdb` - 中国地区数据库
- `GeoLite2-City.mmdb` - 全球城市数据库
- `GeoLite2-ASN.mmdb` - ASN 信息数据库

## 使用方法

1. 启动服务：
```bash
MMDB_PATH=/path/to/mmdb ./target/release/ipgeo
```

2. 服务提供以下 API 接口：

- 通过查询参数查询：
  ```
  GET /?host={ip或域名}
  ```

- 通过路径参数查询：
  ```
  GET /{ip或域名}
  ```

对于域名查询，服务会自动：
1. 解析 A 记录（IPv4）和 AAAA 记录（IPv6）
2. 查询每个解析到的 IP 地址的地理位置信息
3. 在单个响应中返回组合结果

### 响应示例

```json
{
  "host": "example.com",
  "ips": [
    {
      "ip": "93.184.216.34",
      "as": {
        "number": 15133,
        "name": "EdgeCast Networks",
        "info": ""
      },
      "addr": "93.184.216.0/24",
      "location": {
        "latitude": 34.0655,
        "longitude": -118.2389
      },
      "country": {
        "code": "US",
        "name": "United States"
      },
      "registered_country": {
        "code": "US",
        "name": "United States"
      },
      "regions": ["California", "Los Angeles"],
      "regions_short": ["CA", "LA"]
    }
  ]
}
```

## 项目依赖

- `axum` - Web 框架
- `tokio` - 异步运行时
- `maxminddb` - MaxMind DB 读取器
- `serde` - 序列化框架
- `serde_json` - JSON 支持

## Docker 支持

项目包含 Docker 支持，便于部署。构建和运行命令：

```bash
docker build -t ipgeo .
docker run -p 3000:3000 -v /path/to/mmdb:/mmdb -e MMDB_PATH=/mmdb ipgeo
```

## 开源协议

本项目采用 GNU 通用公共许可证第3版 (GPL-3.0) 开源。详情请参阅 [LICENSE](LICENSE) 文件。

## 贡献

欢迎提交贡献！请随时向 [GitHub 仓库](https://github.com/upteka/ipgeo-api-rust) 提交 Pull Request。 